use std::sync::Arc;

use async_session::MemoryStore;
use axum::{
    extract::Host,
    response::Html,
    routing::{any, get},
    Extension, Router,
};
use hyper::{Body, Request};
use tokio::sync::broadcast::Sender;

use tower::{ServiceBuilder, ServiceExt};

use crate::{
    apps::proxy_handler,
    configuration::{load_config, ConfigMap, HostType},
    davs::webdav_handler,
    middlewares::strip_port_from_host_middleware,
    users::local_auth,
};

pub struct Server {
    pub router: Router,
    pub port: u16,
}

impl Server {
    pub async fn build(config_file: &str, tx: Sender<()>) -> Result<Self, anyhow::Error> {
        let config = load_config(config_file).await?;

        async fn website_handler(_user: crate::users::User) -> Html<String> {
            Html(format!("Hello world from main server !"))
        }

        let store = MemoryStore::new();

        let website_router = Router::new()
            .route(
                "/reload",
                get(|| async move {
                    tx.send(()).expect("Could not send reload command!");
                    Html(format!("Apps reloaded !"))
                }),
            )
            .route("/auth/local", get(local_auth))
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
                        match configmap.get(&hostname) {
                            Some(HostType::App(_)) => proxy_router.oneshot(request).await,
                            Some(HostType::Dav(_)) => webdav_router.oneshot(request).await,
                            None => website_router.oneshot(request).await,
                        }
                    },
                ),
            )
            .layer(
                ServiceBuilder::new()
                    .layer(Extension(store))
                    .layer(Extension(config.1))
                    .layer(axum::middleware::from_fn(strip_port_from_host_middleware)),
            );

        Ok(Server {
            router: router,
            port: config.0.http_port,
        })
    }
}
