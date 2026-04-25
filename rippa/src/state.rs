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

    pub async fn makemkv_check_init(&mut self) -> Result<(), RippaError> {
        if let Some((result, makemkv)) = self.makemkv_tasks.check_init().await? {
            self.makemkv = Some(makemkv);
            self.makemkv_info = Some(result?);
            self.makemkv_state = MakeMkvState::Init;
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
}

type MakeMkvInitResult = (Result<MakeMkvInfo, MakeMkvError>, MakeMkv);

#[derive(Default)]
struct MakeMkvTasks {
    init: Option<JoinHandle<MakeMkvInitResult>>,
    disc_data: Option<JoinHandle<Result<TitleList, MakeMkvError>>>,
    rip: Option<JoinHandle<Result<(), MakeMkvError>>>,
}

impl MakeMkvTasks {
    async fn check_init(&mut self) -> Result<Option<MakeMkvInitResult>, RippaError> {
        let init = self.init.take().ok_or(RippaError::MakeMkvNotRunning)?;
        if init.is_finished() {
            Ok(Some(init.await?))
        } else {
            self.init = Some(init);
            Ok(None)
        }
    }
}
