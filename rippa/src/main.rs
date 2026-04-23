//#![forbid(clippy::unwrap_used)]

use crate::routes::get_router;
use makemkv::MakeMkv;
use tokio::net::TcpListener;

mod error;
mod routes;
mod state;
mod templates;

#[tokio::main]
async fn main() {
    flexi_logger::Logger::try_with_str("debug")
        .unwrap()
        .start()
        .unwrap();

    if let Err(e) = run_makemkv().await {
        eprintln!("MakeMKV error: {:#}", e);
    }

    // if let Err(e) = start_server().await {
    //     eprintln!("Server error: {}", e);
    // }
}

async fn start_server() -> anyhow::Result<()> {
    let router = get_router();

    let listener = TcpListener::bind("0.0.0.0:45566").await?;
    axum::serve(listener, router).await?;

    Ok(())
}

async fn run_makemkv() -> anyhow::Result<()> {
    let mut makemkv = MakeMkv::new();
    println!("init");
    makemkv.init().await?;
    println!("set output folder");
    makemkv.set_output_folder("/data/media/dmp").await?;
    println!("wait for disc");
    makemkv.wait_for_disc_inserted().await?;
    println!("get disc data");
    makemkv.get_disc_data().await?;

    println!("Disc data: {:?}", &makemkv.titles);

    Ok(())
}
