use axum::extract::ConnectInfo;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct App {
    pub id: i64,
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

use axum::http::{Request, Response};

use hyper::{client::HttpConnector, Body, StatusCode, Version};

use std::net::SocketAddr;
type Client = hyper::client::Client<HttpConnector, Body>;
use hyper::client::connect::dns::GaiResolver;
use hyper_reverse_proxy::ReverseProxy;

use crate::configuration::HostType;
use crate::users::check_authorization;
use crate::users::User;

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
