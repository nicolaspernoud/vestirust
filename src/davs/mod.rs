pub(crate) mod encrypted_streamer;
pub mod model;
pub(crate) mod streamer;
pub(crate) mod webdav_server;

use std::sync::Arc;

use axum::{
    extract::ConnectInfo,
    http::{Request, Response},
};

use crate::users::check_authorization;
use crate::{configuration::HostType, users::User};
use hyper::{Body, StatusCode};
use std::net::SocketAddr;

lazy_static::lazy_static! {
    static ref  WEBDAV_SERVER: Arc<webdav_server::WebdavServer> = {
        Arc::new(webdav_server::WebdavServer::new(

        ))
    };
}

pub async fn webdav_handler(
    user: User,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    dav: HostType,
    req: Request<Body>,
) -> Response<Body> {
    if let Some(value) = check_authorization(&dav, user) {
        return value;
    }

    let dav = match dav {
        HostType::Dav(app) => app,
        _ => panic!("Service is not a dav !"),
    };

    match WEBDAV_SERVER.clone().call(req, addr, &dav).await {
        Ok(response) => response,
        Err(_) => Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::empty())
            .unwrap(),
    }
}
