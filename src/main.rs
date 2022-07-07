use anyhow::Result;
use async_rustls::rustls::Session;
use futures_util::future::poll_fn;
use hyper::server::accept::Accept;
use hyper::server::conn::{AddrIncoming, Http};
use rustls_acme::caches::DirCache;
use rustls_acme::AcmeConfig;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio::signal;
use tokio::sync::broadcast;
use tokio_stream::StreamExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tokio_util::compat::TokioAsyncReadCompatExt;
use tower::MakeService;
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
        let mock1_listener =
            std::net::TcpListener::bind("127.0.0.1:8081").expect("failed to bind to port");
        tokio::spawn(mock_proxied_server(mock1_listener));
        let mock2_listener =
            std::net::TcpListener::bind("127.0.0.1:8082").expect("failed to bind to port");
        tokio::spawn(mock_proxied_server(mock2_listener));
    }

    let continue_main_loop = std::sync::Arc::new(std::sync::Mutex::new(true));

    if config.0.auto_tls {
        let listener = tokio::net::TcpListener::bind(format!("[::]:443"))
            .await
            .unwrap();
        let mut addr_incoming = AddrIncoming::from_listener(listener).unwrap();
        loop {
            info!("Starting Main loop");
            let continue_main_loop = continue_main_loop.clone();
            if !(*continue_main_loop.lock().unwrap()) {
                break;
            };
            let config = vestibule::configuration::load_config(CONFIG_FILE).await?;
            let mut domains: Vec<String> = config
                .0
                .apps
                .iter()
                .map(|app| format!("{}.{}", app.host.to_owned(), config.0.hostname))
                .chain(
                    config
                        .0
                        .davs
                        .iter()
                        .map(|dav| format!("{}.{}", dav.host.to_owned(), config.0.hostname)),
                )
                .collect();
            domains.insert(0, config.0.hostname);
            let mut state = AcmeConfig::new(domains)
                .contact_push(format!("mailto:{}", config.0.letsencrypt_email))
                .cache(DirCache::new("./letsencrypt_cache"))
                .state();
            let acceptor = state.acceptor();

            tokio::spawn(async move {
                loop {
                    match state.next().await.unwrap() {
                        Ok(ok) => info!("ACME (let's encrypt) event: {:?}", ok),
                        Err(err) => error!("ACME (let's encrypt) error: {:?}", err),
                    }
                }
            });

            let app = Server::build(CONFIG_FILE, tx.clone()).await?;

            let mut app = app
                .router
                .into_make_service_with_connect_info::<SocketAddr>();

            let continue_inner_loop = std::sync::Arc::new(std::sync::Mutex::new(true));

            loop {
                info!("Starting TLS loop");
                let continue_inner_loop = continue_inner_loop.clone();
                let continue_main_loop = continue_main_loop.clone();
                if !(*continue_inner_loop.lock().unwrap()) {
                    break;
                };
                let stream = poll_fn(|cx| Pin::new(&mut addr_incoming).poll_accept(cx))
                    .await
                    .unwrap()
                    .unwrap();
                let acceptor = acceptor.clone();

                let app = app.make_service(&stream).await.unwrap();
                let mut rx = tx.subscribe();

                tokio::spawn(async move {
                    let tls = acceptor.accept(stream.compat()).await.unwrap().compat();
                    match tls.get_ref().get_ref().1.get_alpn_protocol() {
                        Some(_acme_tls_alpn_name) => {
                            info!("received TLS-ALPN-01 validation request")
                        }
                        _ => {
                            let future = Http::new().serve_connection(tls, app);
                            tokio::pin!(future);
                            tokio::select! {
                                _ = &mut future => {},
                                _ = rx.recv() => {
                                    info!("Reloading configuration...");
                                    *continue_inner_loop.lock().unwrap() = false;
                                    future.as_mut().graceful_shutdown();
                                },
                                _ = shutdown_signal() => {
                                        info!("Shutting down...");
                                        *continue_main_loop.lock().unwrap() = false;
                                        *continue_inner_loop.lock().unwrap() = false;
                                        future.as_mut().graceful_shutdown();
                                },
                            }
                        }
                    }
                });
            }
        }
    } else {
        loop {
            let continue_main_loop = continue_main_loop.clone();
            if !(*continue_main_loop.lock().unwrap()) {
                break;
            };
            let mut rx = tx.subscribe();
            let app = Server::build(CONFIG_FILE, tx.clone()).await?;
            let addr = SocketAddr::from(([127, 0, 0, 1], app.port));
            axum::Server::bind(&addr)
                .serve(
                    app.router
                        .into_make_service_with_connect_info::<SocketAddr>(),
                )
                .with_graceful_shutdown(async move {
                    tokio::select! {
                        _ = rx.recv() => {},
                        _ = shutdown_signal() => {*continue_main_loop.lock().unwrap() = false;},
                    }
                })
                .await?;
        }
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
