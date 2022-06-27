use std::net::SocketAddr;

use anyhow::Result;
use vestibule::logger;
use vestibule::server::Server;

#[macro_use]
extern crate log;

#[tokio::main]
async fn main() -> Result<()> {
    logger::init()?;
    info!("Starting server...");
    let app = Server::build("vestibule.yaml").await?;
    let addr = SocketAddr::from(([127, 0, 0, 1], app.port));
    println!("reverse proxy listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            app.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;
    Ok(())
}
