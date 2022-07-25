use axum::extract::{ConnectInfo, Path};
use axum::http::{Request, Response};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde::Deserialize;
use serde::Serialize;

use hyper::{client::HttpConnector, Body, StatusCode, Version};

use std::net::SocketAddr;
type Client = hyper::client::Client<HttpConnector, Body>;
use hyper::client::connect::dns::GaiResolver;
use hyper_reverse_proxy::ReverseProxy;

use crate::configuration::{Config, ConfigFile, HostType};
use crate::users::User;
use crate::users::{check_authorization, Admin};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct App {
    pub id: usize,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub is_proxy: bool,
    pub host: String,
    pub forward_to: String,
    pub secured: bool,
    pub login: String,
    pub password: String,
    pub openpath: String,
    pub roles: Vec<String>,
}

lazy_static::lazy_static! {
    static ref  PROXY_CLIENT: ReverseProxy<HttpConnector<GaiResolver>> = {
        ReverseProxy::new(
            Client::new()
        )
    };
}

pub async fn proxy_handler(
    user: Option<User>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    app: HostType,
    mut req: Request<Body>,
) -> Response<Body> {
    *Request::version_mut(&mut req) = Version::HTTP_11;

    if let Some(value) = check_authorization(&app, &user) {
        return value;
    }

    let app = match app {
        HostType::App(app) => app,
        _ => panic!("Service is not an app !"),
    };

    match PROXY_CLIENT
        .call(
            addr.ip(),
            format!("http://{}", app.forward_to).as_str(),
            req,
        )
        .await
    {
        Ok(response) => response,
        Err(_error) => {
            eprint!("_error: {:?}", _error);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap()
        }
    }
}

pub async fn get_apps(
    config: Config,
    _admin: Admin,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // Return all the apps as Json
    let encoded = serde_json::to_string(&config.apps).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not encode apps".to_owned(),
        )
    })?;
    Ok((StatusCode::OK, encoded))
}

pub async fn delete_app(
    config_file: Extension<ConfigFile>,
    mut config: Config,
    _admin: Admin,
    Path(app_id): Path<(String, usize)>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Find the app
    if let Some(pos) = config.apps.iter().position(|a| a.id == app_id.1) {
        // It is an existing app, delete it
        config.apps.remove(pos);
    } else {
        // If the app doesn't exist, respond with an error
        return Err((StatusCode::BAD_REQUEST, "app doesn't exist"));
    }

    config
        .to_file_or_internal_server_error(&config_file)
        .await?;

    Ok((StatusCode::OK, "app deleted successfully"))
}

pub async fn add_app(
    config_file: Extension<ConfigFile>,
    mut config: Config,
    _admin: Admin,
    Json(payload): Json<App>,
) -> Result<(StatusCode, &'static str), (StatusCode, &'static str)> {
    // Find the app
    if let Some(app) = config.apps.iter_mut().find(|a| a.id == payload.id) {
        *app = payload;
    } else {
        config.apps.push(payload);
    }

    config
        .to_file_or_internal_server_error(&config_file)
        .await?;

    Ok((StatusCode::CREATED, "app created or updated successfully"))
}
