#![forbid(clippy::unwrap_used)]

use crate::routes::get_router;
use tokio::net::TcpListener;

mod error;
mod routes;
mod state;
mod templates;
mod util;

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

    let listener = TcpListener::bind("[::]:45566").await?;
    axum::serve(listener, router).await?;

    Ok(())
}
