#![forbid(clippy::unwrap_used)]

mod error;
mod routes;
mod state;
mod templates;
mod util;

use crate::routes::get_router;
use tokio::net::TcpListener;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    flexi_logger::Logger::try_with_str("debug")?.start()?;

    if let Err(e) = start_server().await {
        eprintln!("Server error: {}", e);
    }

    Ok(())
}

async fn start_server() -> anyhow::Result<()> {
    let router = get_router();

    let listener = TcpListener::bind("[::]:34543").await?;
    axum::serve(listener, router).await?;

    Ok(())
}
