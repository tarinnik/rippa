use crate::error::RippaError;
use log::{debug, error};
use makemkv::{MakeMkv, MakeMkvInfo, MakeMkvProgress, error::MakeMkvError, title::TitleList};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{
    select,
    sync::{
        RwLock,
        mpsc::{Receiver as MpscReceiver, Sender as MpscSender},
        watch::{Receiver as WatchReceiver, channel, error::RecvError},
    },
    task::JoinHandle,
    time::sleep,
};

#[derive(Default)]
pub struct MakeMkvState {
    pub status: MakeMkvStatus,
    pub info: Option<MakeMkvInfo>,
    pub titles: Option<TitleList>,
    pub rip_progress: MakeMkvProgress,
}

pub struct MakeMkvTask {
    makemkv: Option<MakeMkv>,
    state: Arc<RwLock<MakeMkvState>>,
    command_rx: MpscReceiver<MakeMkvCommand>,
    command_response_tx: MpscSender<Result<(), RippaError>>,
    tasks: MakeMkvTasks,
    rip_progress_rx: Option<WatchReceiver<MakeMkvProgress>>,
}

impl MakeMkvTask {
    pub fn new(
        state: Arc<RwLock<MakeMkvState>>,
        command_rx: MpscReceiver<MakeMkvCommand>,
        command_response_tx: MpscSender<Result<(), RippaError>>,
    ) -> Self {
        Self {
            makemkv: Some(MakeMkv::new()),
            state,
            command_rx,
            command_response_tx,
            tasks: MakeMkvTasks::default(),
            rip_progress_rx: None,
        }
    }

    pub async fn run(&mut self) {
        loop {
            select! {
                // Command from web server
                Some(command) = self.command_rx.recv() => {
                    self.process_command(command).await;
                }

                // Check if init is finished
                Ok(()) = makemkv_task_check(&self.tasks.init) => {
                    self.finish_init().await;
                }

                // Check if get disc data is finished
                Ok(()) = makemkv_task_check(&self.tasks.load) => {
                    self.finish_load_disc().await;
                }

                // Check if rip is finished
                Ok(()) = makemkv_task_check(&self.tasks.rip) => {
                    self.finish_rip().await;
                }

                // Check rip progress
                Some(Ok(())) = watch_receiver_changed(&mut self.rip_progress_rx) => {
                    self.get_progress().await;
                }

                else => {
                    // All paths disabled
                    error!("MakeMkvTask: all paths disabled, nothing to do, exiting");
                    break;
                }
            }
        }
    }

    async fn process_command(&mut self, command: MakeMkvCommand) {
        let result = match command {
            MakeMkvCommand::Init => self.init().await,
            MakeMkvCommand::Load => self.load_disc().await,
            MakeMkvCommand::Rip(data) => self.rip(data).await,
        };

        self.command_response_tx.send(result).await;
    }

    async fn init(&mut self) -> Result<(), RippaError> {
        let mut makemkv = self
            .makemkv
            .take()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;

        self.tasks.init = Some(tokio::spawn(async move {
            let result = makemkv.init().await;
            (result, makemkv)
        }));

        let mut state = self.state.write().await;
        state.status = MakeMkvStatus::Initialising;
        Ok(())
    }

    async fn finish_init(&mut self) -> Result<(), RippaError> {
        let (result, makemkv) = self
            .tasks
            .init
            .take()
            .ok_or(RippaError::MakeMkvNotRunning)?
            .await?;

        self.makemkv = Some(makemkv);
        let mut state = self.state.write().await;
        state.info = Some(result?);
        state.status = MakeMkvStatus::Init;

        Ok(())
    }

    async fn load_disc(&mut self) -> Result<(), RippaError> {
        let mut makemkv = self
            .makemkv
            .take()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;

        self.tasks.load = Some(tokio::spawn(async move {
            let result = makemkv.get_disc_data().await;
            (result, makemkv)
        }));

        let mut state = self.state.write().await;
        state.status = MakeMkvStatus::GettingData;
        Ok(())
    }

    async fn finish_load_disc(&mut self) -> Result<(), RippaError> {
        let (result, makemkv) = self
            .tasks
            .load
            .take()
            .ok_or(RippaError::MakeMkvNotRunning)?
            .await?;

        self.makemkv = Some(makemkv);
        let mut state = self.state.write().await;
        state.titles = Some(result?);
        state.status = MakeMkvStatus::LoadedData;

        Ok(())
    }

    async fn rip(&mut self, title_map: HashMap<usize, Vec<usize>>) -> Result<(), RippaError> {
        let makemkv = self
            .makemkv
            .as_mut()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;

        let mut state = self.state.write().await;
        let titles = state.titles.as_mut().ok_or(RippaError::InvalidTitle)?;

        // Enable/disable the titles
        for (title_index, title) in titles.titles.iter_mut().enumerate() {
            if let Some(title) = title {
                let track_list = title_map
                    .get(&title_index)
                    .cloned()
                    .unwrap_or_else(Vec::new);

                let title_enabled = title_map.contains_key(&title_index);
                debug!("Title {} enabled: {}", title_index, title_enabled);
                makemkv.enable(title, title_enabled).await?;

                // Enable/disable the tracks
                for (track_index, track) in title.tracks.iter_mut().enumerate() {
                    if let Some(track) = track {
                        let track_enabled = track_list.contains(&track_index);
                        debug!("Track {} enabled: {}", track_index, track_enabled);
                        makemkv.enable(track, track_enabled).await?;
                    }
                }
            }
        }

        makemkv.set_output_folder("/data/media/dmp/").await?;

        let (progress_tx, progress_rx) = channel(MakeMkvProgress::default());
        state.rip_progress = MakeMkvProgress::default();
        self.rip_progress_rx = Some(progress_rx);

        let mut makemkv = self
            .makemkv
            .take()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;
        self.tasks.rip = Some(tokio::spawn(async move {
            let result = makemkv.rip_all_selected(progress_tx).await;
            (result, makemkv)
        }));
        state.status = MakeMkvStatus::Ripping;

        Ok(())
    }

    async fn finish_rip(&mut self) -> Result<(), RippaError> {
        let (result, makemkv) = self
            .tasks
            .rip
            .take()
            .ok_or(RippaError::MakeMkvNotRunning)?
            .await?;

        result?;
        self.makemkv = Some(makemkv);
        self.rip_progress_rx = None;
        let mut state = self.state.write().await;
        state.status = MakeMkvStatus::Done;

        Ok(())
    }

    async fn get_progress(&mut self) {
        if let Some(rx) = &mut self.rip_progress_rx {
            let mut state = self.state.write().await;
            state.rip_progress = *rx.borrow_and_update();
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum MakeMkvStatus {
    #[default]
    NotInit,
    Initialising,
    Init,
    GettingData,
    LoadedData,
    Ripping,
    Done,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MakeMkvCommand {
    Init,
    Load,
    Rip(HashMap<usize, Vec<usize>>),
}

type MakeMkvInitResult = (Result<MakeMkvInfo, MakeMkvError>, MakeMkv);
type MakeMkvLoadResult = (Result<TitleList, MakeMkvError>, MakeMkv);
type MakeMkvRipResult = (Result<(), MakeMkvError>, MakeMkv);

#[derive(Default)]
struct MakeMkvTasks {
    init: Option<JoinHandle<MakeMkvInitResult>>,
    load: Option<JoinHandle<MakeMkvLoadResult>>,
    rip: Option<JoinHandle<MakeMkvRipResult>>,
}

async fn makemkv_task_check<T>(task: &Option<JoinHandle<T>>) -> Result<(), RippaError> {
    let handle = task.as_ref().ok_or(RippaError::MakeMkvNotRunning)?;
    while !handle.is_finished() {
        sleep(Duration::from_millis(250)).await;
    }

    Ok(())
}

async fn watch_receiver_changed<T>(
    rx: &mut Option<WatchReceiver<T>>,
) -> Option<Result<(), RecvError>> {
    Some(rx.as_mut()?.changed().await)
}
