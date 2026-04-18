use std::io;
use tokio::time::error::Elapsed;

#[derive(Debug, Clone, thiserror::Error)]
pub enum MakeMkvError {
    #[error("Error ocurred: {0}")]
    MakeMkv(String),
    #[error("Invalid MakeMKV command received: {0}")]
    InvalidCommand(String),
    #[error("Invalid MakeMKV response: {0}")]
    InvalidResponse(String),
    #[error("No disc drive detected")]
    DriveNotDetected,
    #[error("Unable to parse drive info: {0}")]
    DriveInfo(String),
}

impl From<io::Error> for MakeMkvError {
    fn from(value: io::Error) -> Self {
        Self::MakeMkv(format!("IO Error: {}", value))
    }
}

impl From<Elapsed> for MakeMkvError {
    fn from(_value: Elapsed) -> Self {
        Self::MakeMkv(String::from("Timeout receiving data"))
    }
}

impl From<anyhow::Error> for MakeMkvError {
    fn from(value: anyhow::Error) -> Self {
        Self::MakeMkv(format!("{:#}", value))
    }
}
