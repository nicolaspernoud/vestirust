use axum::extract::{ConnectInfo, Path};
use axum::http::uri::{Authority, Scheme};
use axum::http::{Request, Response};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use headers::HeaderValue;
use hyper::header::{HOST, LOCATION};
use hyper::Uri;
use hyper_trust_dns::RustlsHttpsConnector;
use hyper_trust_dns::TrustDnsResolver;
use log::error;
use serde::Deserialize;
use serde::Serialize;

use hyper::{Body, StatusCode};

use hyper_reverse_proxy::ReverseProxy;
use std::net::SocketAddr;

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

#[derive(PartialEq, Debug, Clone)]
pub struct AppWithUri {
    pub inner: App,
    pub app_scheme: Scheme,
    pub app_authority: Authority,
    pub forward_uri: Uri,
    pub forward_scheme: Scheme,
    pub forward_authority: Authority,
    pub forward_host: String,
}

impl AppWithUri {
    pub fn from_app_domain_and_http_port(inner: App, domain: &str, port: Option<u16>) -> Self {
        let app_scheme = if port.is_some() {
            Scheme::HTTP
        } else {
            Scheme::HTTPS
        };
        let app_authority = if let Some(port) = port {
            format!("{}.{}:{}", inner.host, domain, port)
                .parse()
                .expect("could not work out authority from app configuration")
        } else {
            format!("{}.{}", inner.host, domain)
                .parse()
                .expect("could not work out authority from app configuration")
        };
        let forward_scheme = if inner.forward_to.starts_with("https://") {
            Scheme::HTTPS
        } else {
            Scheme::HTTP
        };
        let forward_uri: Uri = inner
            .forward_to
            .parse()
            .expect("could not parse app target service");
        let mut forward_parts = forward_uri.into_parts();
        let forward_authority = forward_parts
            .authority
            .clone()
            .expect("could not parse app target service host");

        let forward_host = forward_authority.host().to_owned();
        forward_parts.scheme = Some(forward_scheme.clone());
        forward_parts.path_and_query = Some("/".parse().unwrap());
        let forward_uri = Uri::from_parts(forward_parts).unwrap();
        Self {
            inner,
            app_scheme,
            app_authority,
            forward_uri,
            forward_scheme,
            forward_authority,
            forward_host,
        }
    }
}

lazy_static::lazy_static! {
    static ref  PROXY_CLIENT: ReverseProxy<RustlsHttpsConnector> = {
        ReverseProxy::new(
            hyper::Client::builder().build::<_, hyper::Body>(TrustDnsResolver::default().into_rustls_webpki_https_connector()),
        )
    };
}

pub async fn proxy_handler(
    user: Option<User>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    app: HostType,
    mut req: Request<Body>,
) -> Response<Body> {
    if let Some(value) = check_authorization(&app, &user) {
        return value;
    }

    let app = match app {
        HostType::App(app) => app,
        _ => panic!("Service is not an app !"),
    };

    // Alter request
    let uri = req.uri_mut();
    let mut parts = uri.clone().into_parts();
    parts.scheme = Some(app.forward_scheme);
    if let Some(port) = &app.forward_authority.port() {
        parts.authority = Some(format!("{}:{}", app.forward_host, port).parse().unwrap());
    } else {
        parts.authority = Some(app.forward_host.parse().unwrap());
    }

    *uri = Uri::from_parts(parts).unwrap();

    // If the target service contains no port, is to an external service and we need to rewrite the host header to fool the target site
    if app.forward_authority.port().is_none() {
        req.headers_mut().insert(
            HOST,
            HeaderValue::from_str(&app.forward_authority.to_string()).unwrap(),
        );
    }

    // TODO : If the app contains basic auth information, forge a basic auth header

    match PROXY_CLIENT
        .call(addr.ip(), &app.forward_uri.to_string(), req)
        .await
    {
        Ok(mut response) => {
            // If the response contains a location, alter the redirect location if the redirection is relative to the proxied host

            if let Some(location) = response.headers().get("location") {
                // parse location as an url
                let location_uri: Uri = match location.to_str().unwrap().parse() {
                    Ok(uri) => uri,
                    Err(e) => {
                        error!("Proxy uri parse error : {:?}", e);
                        return Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body(Body::empty())
                            .unwrap();
                    }
                };
                // test if the host of this url contains the target service host
                if location_uri.host().is_some()
                    && location_uri.host().unwrap().contains(&app.forward_host)
                {
                    // if so, replace the target service host with the front service host
                    let mut parts = location_uri.into_parts();
                    parts.scheme = Some(app.app_scheme);
                    parts.authority = Some(app.app_authority);
                    let uri = Uri::from_parts(parts).unwrap();

                    response
                        .headers_mut()
                        .insert(LOCATION, HeaderValue::from_str(&uri.to_string()).unwrap());
                }
            }
            response
        }
        Err(e) => {
            error!("Proxy error: {:?}", e);
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
