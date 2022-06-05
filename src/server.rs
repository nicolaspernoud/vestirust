use std::sync::Arc;

use axum::{extract::Host, response::Html, routing::any, Extension, Router};
use hyper::{Body, Request};
use tokio::sync::Mutex;
use tower::ServiceExt;

use crate::{
    apps::proxy_handler,
    configuration::{load_config, reload_config, Config, HostType},
    mocks::mock_proxied_server,
};

pub struct Server {
    pub router: Router,
    pub port: u16,
}

impl Server {
    pub async fn build(config_file: &str) -> Result<Self, anyhow::Error> {
        let config = load_config(config_file).await?;
        let cfg = config.lock().await;
        let port = cfg.http_port;
        if cfg.debug_mode {
            tokio::spawn(mock_proxied_server(port, 1));
            tokio::spawn(mock_proxied_server(port, 2));
        }
        drop(cfg);
        async fn website_handler() -> Html<String> {
            Html(format!("Hello world from main server !"))
        }
        async fn reload_handler(Extension(config): Extension<Arc<Mutex<Config>>>) -> Html<String> {
            reload_config(&config)
                .await
                .expect("Failed to reload configuration");
            Html(format!("Apps reloaded !"))
        }
        let website_router = Router::new()
            .route("/reload", any(reload_handler))
            .route("/", any(website_handler));

        let proxy_router = Router::new().route("/*path", any(proxy_handler));

        async fn webdav_handler() -> Html<String> {
            Html(format!("This is webdav handler!"))
        }
        let webdav_router = Router::new().route("/*path", any(webdav_handler));

        let router = Router::new()
            .route(
                "/*path",
                any(
                    |Extension(config): Extension<Arc<Mutex<Config>>>,
                     Host(hostname): Host,
                     request: Request<Body>| async move {
                        let hostname = hostname.split(":").next().unwrap();
                        match config.lock().await.hosts_map.get(hostname) {
                            Some(HostType::App(_)) => proxy_router.oneshot(request).await,
                            Some(HostType::Dav(_)) => webdav_router.oneshot(request).await,
                            None => website_router.oneshot(request).await,
                        }
                    },
                ),
            )
            .layer(Extension(config));

        Ok(Server {
            router: router,
            port: port,
        })
    }
}
