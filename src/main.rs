use std::net::SocketAddr;

use anyhow::Result;
use tokio::signal;
use tokio::sync::broadcast;
use vestibule::logger;
use vestibule::mocks::mock_proxied_server;
use vestibule::server::Server;

#[macro_use]
extern crate log;

const CONFIG_FILE: &'static str = "vestibule.yaml";

#[tokio::main]
async fn main() -> Result<()> {
    logger::init()?;
    info!("Starting server...");

    let (tx, _) = broadcast::channel(16);
    let config = vestibule::configuration::load_config(CONFIG_FILE).await?;
    if config.0.debug_mode {
        tokio::spawn(mock_proxied_server(config.0.http_port, 1));
        tokio::spawn(mock_proxied_server(config.0.http_port, 2));
    }

    let continue_loop = std::sync::Arc::new(tokio::sync::Mutex::new(true));

    loop {
        if !(*continue_loop.lock().await) {
            break;
        };
        let mut rx = tx.subscribe();
        let app = Server::build(CONFIG_FILE, tx.clone()).await?;
        let addr = SocketAddr::from(([127, 0, 0, 1], app.port));
        let continue_loop = continue_loop.clone();
        axum::Server::bind(&addr)
            .serve(
                app.router
                    .into_make_service_with_connect_info::<SocketAddr>(),
            )
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = rx.recv() => {},
                    _ = shutdown_signal() => {*continue_loop.lock().await = false;},
                }
            })
            .await?;
    }

    info!("Graceful shutdown done !");

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
