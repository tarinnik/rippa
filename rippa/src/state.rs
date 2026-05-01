use std::collections::HashMap;

use log::debug;
use makemkv::{MakeMkv, MakeMkvInfo, MakeMkvProgress, error::MakeMkvError, title::TitleList};
use tokio::{
    sync::watch::{Receiver, channel},
    task::JoinHandle,
};

use crate::error::RippaError;

#[derive(Default)]
pub struct RippaState {
    pub makemkv: Option<MakeMkv>,
    pub makemkv_state: MakeMkvState,
    pub makemkv_info: Option<MakeMkvInfo>,
    pub titles: Option<TitleList>,
    makemkv_tasks: MakeMkvTasks,
    makemkv_rip_progress_rx: Option<Receiver<MakeMkvProgress>>,
    pub makemkv_rip_progress: MakeMkvProgress,
}

impl RippaState {
    pub fn new() -> Self {
        Self {
            makemkv: Some(MakeMkv::new()),
            ..Default::default()
        }
    }

    pub async fn makemkv_init(&mut self) -> Result<(), RippaError> {
        let mut makemkv = self
            .makemkv
            .take()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;

        self.makemkv_tasks.init = Some(tokio::spawn(async move {
            let result = makemkv.init().await;
            (result, makemkv)
        }));
        self.makemkv_state = MakeMkvState::Initialising;
        Ok(())
    }

    pub async fn makemkv_init_check(&mut self) -> Result<(), RippaError> {
        if let Some((result, makemkv)) = makemkv_task_check(&mut self.makemkv_tasks.init).await? {
            self.makemkv = Some(makemkv);
            self.makemkv_info = Some(result?);
            self.makemkv_state = MakeMkvState::Init;
        }
        Ok(())
    }

    pub async fn makemkv_disc_data(&mut self) -> Result<(), RippaError> {
        let mut makemkv = self
            .makemkv
            .take()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;

        self.makemkv_tasks.disc_data = Some(tokio::spawn(async move {
            let result = makemkv.get_disc_data().await;
            (result, makemkv)
        }));
        self.makemkv_state = MakeMkvState::GettingData;
        Ok(())
    }

    pub async fn makemkv_disc_data_check(&mut self) -> Result<(), RippaError> {
        if let Some((result, makemkv)) =
            makemkv_task_check(&mut self.makemkv_tasks.disc_data).await?
        {
            self.makemkv = Some(makemkv);
            self.titles = Some(result?);
            self.makemkv_state = MakeMkvState::LoadedData;
        }
        Ok(())
    }

    pub async fn makemkv_rip(
        &mut self,
        title_map: HashMap<usize, Vec<usize>>,
    ) -> Result<(), RippaError> {
        let makemkv = self
            .makemkv
            .as_mut()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;
        let titles = self.titles.as_mut().ok_or(RippaError::InvalidTitle)?;

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
        self.makemkv_rip_progress = MakeMkvProgress::default();
        self.makemkv_rip_progress_rx = Some(progress_rx);

        let mut makemkv = self
            .makemkv
            .take()
            .ok_or(RippaError::MakeMkvAlreadyRunning)?;
        self.makemkv_tasks.rip = Some(tokio::spawn(async move {
            let result = makemkv.rip_all_selected(progress_tx).await;
            (result, makemkv)
        }));
        self.makemkv_state = MakeMkvState::Ripping;

        Ok(())
    }

    pub async fn makemkv_rip_check(&mut self) -> Result<(), RippaError> {
        // Get progress
        let rx = self
            .makemkv_rip_progress_rx
            .as_mut()
            .ok_or(RippaError::MakeMkvNotRunning)?;

        if let Ok(true) = rx.has_changed() {
            self.makemkv_rip_progress = *rx.borrow_and_update();
        }

        // Check if rip is done
        if let Some((result, makemkv)) = makemkv_task_check(&mut self.makemkv_tasks.rip).await? {
            result?;
            self.makemkv = Some(makemkv);
            self.makemkv_state = MakeMkvState::Done;
            self.makemkv_rip_progress_rx = None;
        }

        Ok(())
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum MakeMkvState {
    #[default]
    NotInit,
    Initialising,
    Init,
    GettingData,
    LoadedData,
    Ripping,
    Done,
}

type MakeMkvInitResult = (Result<MakeMkvInfo, MakeMkvError>, MakeMkv);
type MakeMkvDiscResult = (Result<TitleList, MakeMkvError>, MakeMkv);
type MakeMkvRipResult = (Result<(), MakeMkvError>, MakeMkv);

#[derive(Default)]
struct MakeMkvTasks {
    init: Option<JoinHandle<MakeMkvInitResult>>,
    disc_data: Option<JoinHandle<MakeMkvDiscResult>>,
    rip: Option<JoinHandle<MakeMkvRipResult>>,
}

async fn makemkv_task_check<T>(task: &mut Option<JoinHandle<T>>) -> Result<Option<T>, RippaError> {
    let handle = task.take().ok_or(RippaError::MakeMkvNotRunning)?;
    if handle.is_finished() {
        Ok(Some(handle.await?))
    } else {
        *task = Some(handle);
        Ok(None)
    }
}
