use axum::{
    response::Html,
    routing::{any, get, post},
    Extension, Router,
};
use axum_extra::extract::cookie::Key;
use hyper::{Body, Request};
use tokio::sync::broadcast::Sender;

use tower::{ServiceBuilder, ServiceExt};

use crate::{
    apps::proxy_handler,
    configuration::{load_config, HostType},
    davs::webdav_handler,
    users::{add_user, local_auth},
};

pub struct Server {
    pub router: Router,
    pub port: u16,
}

impl Server {
    pub async fn build(config_file: &str, tx: Sender<()>) -> Result<Self, anyhow::Error> {
        let config = load_config(config_file).await?;

        let key = Key::generate();

        async fn website_handler() -> Html<String> {
            Html(format!("Hello world from main server !"))
        }

        let admin_router = Router::new().route("/users", post(add_user));

        let website_router = Router::new()
            .route(
                "/reload",
                get(|| async move {
                    tx.send(()).expect("Could not send reload command!");
                    Html(format!("Apps reloaded !"))
                }),
            )
            .route("/auth/local", post(local_auth))
            .nest("/api/admin", admin_router)
            .route("/", any(website_handler));

        let proxy_router = Router::new().route("/*path", any(proxy_handler));
        let webdav_router = Router::new().route("/*path", any(webdav_handler));

        let router = Router::new()
            .route(
                "/*path",
                any(
                    |hostype: Option<HostType>, request: Request<Body>| async move {
                        match hostype {
                            Some(HostType::App(_)) => proxy_router.oneshot(request).await,
                            Some(HostType::Dav(_)) => webdav_router.oneshot(request).await,
                            None => website_router.oneshot(request).await,
                        }
                    },
                ),
            )
            .layer(
                ServiceBuilder::new()
                    .layer(Extension(key))
                    .layer(Extension(config.1)), /*.layer(
                                                     CorsLayer::new()
                                                         .allow_origin(config.0.hostname.parse::<HeaderValue>().unwrap())
                                                         .allow_headers([
                                                             ACCEPT,
                                                             ACCEPT_ENCODING,
                                                             AUTHORIZATION,
                                                             CONTENT_LENGTH,
                                                             COOKIE,
                                                         ])
                                                         .allow_methods([
                                                             Method::POST,
                                                             Method::GET,
                                                             Method::OPTIONS,
                                                             Method::PUT,
                                                             Method::DELETE,
                                                         ])
                                                         .allow_credentials(true),
                                                 ),*/
            );

        Ok(Server {
            router: router,
            port: config.0.http_port,
        })
    }
}
