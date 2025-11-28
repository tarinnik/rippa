use crate::routes::get_router;
use anyhow::Result;
use tokio::net::TcpListener;

mod error;
mod routes;
mod state;
mod templates;

#[tokio::main]
async fn main() {
    if let Err(e) = start_server().await {
        eprintln!("Server error: {}", e);
    }
}

async fn start_server() -> Result<()> {
    let router = get_router();

    let listener = TcpListener::bind("0.0.0.0:45566").await?;
    axum::serve(listener, router).await?;

    Ok(())
}
