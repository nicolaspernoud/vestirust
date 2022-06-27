use std::sync::Arc;

use axum::{extract::Host, response::Html, routing::any, Extension, Router};
use hyper::{Body, Request};

use tower::ServiceExt;

use crate::{
    apps::proxy_handler,
    configuration::{load_config, ConfigMap, HostType},
    davs::webdav_handler,
    mocks::mock_proxied_server,
};

pub struct Server {
    pub router: Router,
    pub port: u16,
}

impl Server {
    pub async fn build(config_file: &str) -> Result<Self, anyhow::Error> {
        let config = load_config(config_file).await?;
        let port = config.0.http_port;
        if config.0.debug_mode {
            tokio::spawn(mock_proxied_server(port, 1));
            tokio::spawn(mock_proxied_server(port, 2));
        }
        async fn website_handler() -> Html<String> {
            Html(format!("Hello world from main server !"))
        }
        /*async fn reload_handler(Extension(configmap): Extension<Arc<ConfigMap>>) -> Html<String> {
            reload_config(&config)
                .await
                .expect("Failed to reload configuration");
            Html(format!("Apps reloaded !"))
        }*/
        let website_router = Router::new()
            //.route("/reload", any(reload_handler))
            .route("/", any(website_handler));

        let proxy_router = Router::new().route("/*path", any(proxy_handler));
        let webdav_router = Router::new().route("/*path", any(webdav_handler));

        let router = Router::new()
            .route(
                "/*path",
                any(
                    |Extension(configmap): Extension<Arc<ConfigMap>>,
                     Host(hostname): Host,
                     request: Request<Body>| async move {
                        let hostname = hostname.split(":").next().unwrap();
                        match configmap.get(hostname) {
                            Some(HostType::App(_)) => proxy_router.oneshot(request).await,
                            Some(HostType::Dav(_)) => webdav_router.oneshot(request).await,
                            None => website_router.oneshot(request).await,
                        }
                    },
                ),
            )
            .layer(Extension(config.1));

        Ok(Server {
            router: router,
            port: port,
        })
    }
}
