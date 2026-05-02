pub mod config;
pub mod makemkv;

use crate::{
    error::RippaError,
    state::{
        config::Config,
        makemkv::{MakeMkvCommand, MakeMkvState, MakeMkvTask},
    },
};
use std::sync::Arc;
use tokio::sync::{
    RwLock,
    mpsc::{Receiver, Sender, channel},
};

const BUFFER_SIZE: usize = 32;

pub struct RippaState {
    pub makemkv: Arc<RwLock<MakeMkvState>>,
    pub makemkv_command: Sender<MakeMkvCommand>,
    pub makemkv_result: Receiver<Result<(), RippaError>>,
    pub config: Arc<RwLock<Config>>,
}

impl RippaState {
    pub fn new() -> Self {
        let makemkv_state = Arc::new(RwLock::new(MakeMkvState::default()));

        let (command_tx, command_rx) = channel(BUFFER_SIZE);
        let (result_tx, result_rx) = channel(BUFFER_SIZE);
        let makemkv_state_clone = makemkv_state.clone();

        tokio::spawn(async move {
            MakeMkvTask::new(makemkv_state_clone, command_rx, result_tx)
                .run()
                .await;
        });

        Self {
            makemkv: makemkv_state,
            makemkv_command: command_tx,
            makemkv_result: result_rx,
            config: Arc::new(RwLock::new(Config::new())),
        }
    }

    pub async fn send_command(&mut self, command: MakeMkvCommand) -> Result<(), RippaError> {
        self.makemkv_command
            .send(command)
            .await
            .expect("Makemkv task command channel closed");
        self.makemkv_result
            .recv()
            .await
            .expect("Makemkv task result channel closed")
    }
}
