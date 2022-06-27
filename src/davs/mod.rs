pub(crate) mod encrypted_streamer;
pub(crate) mod model;
pub(crate) mod streamer;
pub(crate) mod webdav_server;

use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Host},
    http::{Request, Response},
    Extension,
};

use crate::configuration::ConfigMap;
use crate::configuration::HostType;
use hyper::{Body, StatusCode};
use std::net::SocketAddr;

lazy_static::lazy_static! {
    static ref  WEBDAV_SERVER: Arc<webdav_server::WebdavServer> = {
        Arc::new(webdav_server::WebdavServer::new(

        ))
    };
}

pub async fn webdav_handler(
    Extension(configmap): Extension<Arc<ConfigMap>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Host(hostname): Host,
    req: Request<Body>,
) -> Response<Body> {
    let hostname = hostname.split(":").next().unwrap();

    // Work out where to proxy to
    let target = match configmap.get(hostname) {
        Some(HostType::Dav(dav)) => dav,
        _ => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        }
    };

    match WEBDAV_SERVER.clone().call(req, addr, target).await {
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
