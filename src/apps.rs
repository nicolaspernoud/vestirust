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

use axum::{
    extract::Host,
    http::{Request, Response},
    Extension,
};

use hyper::{client::HttpConnector, Body, StatusCode, Version};

use std::net::SocketAddr;
use std::sync::Arc;
type Client = hyper::client::Client<HttpConnector, Body>;
use hyper::client::connect::dns::GaiResolver;
use hyper_reverse_proxy::ReverseProxy;

use crate::configuration::ConfigMap;
use crate::configuration::HostType;

lazy_static::lazy_static! {
    static ref  PROXY_CLIENT: ReverseProxy<HttpConnector<GaiResolver>> = {
        ReverseProxy::new(
            Client::new()
        )
    };
}

pub async fn proxy_handler(
    Extension(configmap): Extension<Arc<ConfigMap>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Host(hostname): Host,
    mut req: Request<Body>,
) -> Response<Body> {
    *Request::version_mut(&mut req) = Version::HTTP_11;
    let hostname = hostname.split(":").next().unwrap();

    // Work out where to proxy to
    let target = match configmap.get(hostname) {
        Some(HostType::App(app)) => app,
        _ => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }
    };

    match PROXY_CLIENT
        .call(
            addr.ip(),
            format!("http://{}", target.forward_to).as_str(),
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
