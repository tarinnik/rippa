use makemkv::{MakeMkv, MakeMkvInfo, error::MakeMkvError, title::TitleList};
use tokio::task::JoinHandle;

use crate::error::RippaError;

#[derive(Default)]
pub struct RippaState {
    pub makemkv: Option<MakeMkv>,
    pub makemkv_state: MakeMkvState,
    pub makemkv_info: Option<MakeMkvInfo>,
    pub titles: Option<TitleList>,
    makemkv_tasks: MakeMkvTasks,
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
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum MakeMkvState {
    #[default]
    NotInit,
    Initialising,
    Init,
    GettingData,
    LoadedData,
}

type MakeMkvInitResult = (Result<MakeMkvInfo, MakeMkvError>, MakeMkv);
type MakeMkvDiscResult = (Result<TitleList, MakeMkvError>, MakeMkv);

#[derive(Default)]
struct MakeMkvTasks {
    init: Option<JoinHandle<MakeMkvInitResult>>,
    disc_data: Option<JoinHandle<MakeMkvDiscResult>>,
    rip: Option<JoinHandle<Result<(), MakeMkvError>>>,
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
