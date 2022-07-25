use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use axum::async_trait;
use axum::extract::rejection::TypedHeaderRejectionReason;
use axum::extract::FromRequest;
use axum::extract::RequestParts;
use axum::Extension;
use axum::TypedHeader;
use hyper::header;
use hyper::StatusCode;
use serde::Deserialize;
use serde::Serialize;

use crate::apps::App;
use crate::davs::model::Dav;
use crate::users::User;
use sha2::{Digest, Sha256};

fn debug_mode() -> bool {
    false
}

fn http_port() -> u16 {
    8080
}

fn auto_tls() -> bool {
    false
}

fn hostname() -> String {
    "vestibule.io".to_owned()
}

#[derive(Deserialize, Serialize, Debug, Default, PartialEq)]
pub struct Config {
    #[serde(default = "hostname")]
    pub hostname: String,
    #[serde(default = "debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "http_port")]
    pub http_port: u16,
    #[serde(default = "auto_tls")]
    pub auto_tls: bool,
    pub letsencrypt_email: String,
    pub apps: Vec<App>,
    pub davs: Vec<Dav>,
    pub users: Vec<User>,
}

pub type ConfigMap = HashMap<String, HostType>;

pub type ConfigFile = String;

impl Config {
    pub async fn from_file(filepath: &str) -> Result<Self> {
        let data = tokio::fs::read_to_string(filepath).await?;
        let config = serde_yaml::from_str::<Config>(&data)?;
        Ok(config)
    }

    pub async fn to_file(&self, filepath: &str) -> Result<()> {
        let contents = serde_yaml::to_string::<Config>(self)?;
        tokio::fs::write(filepath, contents).await?;
        Ok(())
    }

    pub async fn to_file_or_internal_server_error(
        self,
        filepath: &str,
    ) -> Result<(), (StatusCode, &'static str)> {
        self.to_file(filepath).await.map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "could not save configuration",
            )
        })?;
        Ok(())
    }
}

#[async_trait]
impl<B> FromRequest<B> for Config
where
    B: Send,
{
    type Rejection = StatusCode;
    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(config_file) = Extension::<ConfigFile>::from_request(req)
            .await
            .expect("`Config file` extension is missing");
        // Load configuration
        let config = Config::from_file(&config_file)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(config)
    }
}

pub async fn load_config(config_file: &str) -> Result<(Config, Arc<ConfigMap>), anyhow::Error> {
    let config = Config::from_file(config_file).await?;
    let hashmap = config
        .apps
        .iter()
        .map(|app| {
            (
                format!("{}.{}", app.host.to_owned(), config.hostname),
                HostType::App(app.clone()),
            )
        })
        .chain(config.davs.iter().map(|dav| {
            let mut dav = dav.clone();
            if dav.passphrase != "" {
                let mut hasher = Sha256::new();
                hasher.update(&dav.passphrase);
                let result: [u8; 32] = hasher.finalize().into();
                dav.key = Some(result);
            }
            (
                format!("{}.{}", dav.host.to_owned(), config.hostname),
                HostType::Dav(dav),
            )
        }))
        .collect();
    Ok((config, Arc::new(hashmap)))
}

#[derive(PartialEq, Debug, Clone)]
pub enum HostType {
    App(App),
    Dav(Dav),
}

impl HostType {
    pub fn roles(&self) -> &Vec<String> {
        match self {
            HostType::App(app) => &app.roles,
            HostType::Dav(dav) => &dav.roles,
        }
    }

    pub fn secured(&self) -> bool {
        match self {
            HostType::App(app) => app.secured,
            HostType::Dav(dav) => dav.secured,
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for HostType
where
    B: Send,
{
    type Rejection = StatusCode;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let Extension(configmap) = Extension::<Arc<HashMap<String, HostType>>>::from_request(req)
            .await
            .expect("`Config` extension is missing");

        let host = TypedHeader::<headers::Host>::from_request(req)
            .await
            .map_err(|e| match *e.name() {
                header::HOST => match e.reason() {
                    TypedHeaderRejectionReason::Missing => StatusCode::NOT_FOUND,
                    _ => panic!("unexpected error getting Host header(s): {}", e),
                },
                _ => panic!("unexpected error getting Host header(s): {}", e),
            })?;

        let host = host.hostname();

        // Work out where to target to
        let target = configmap
            .get(host)
            .ok_or(())
            .map_err(|_| StatusCode::NOT_FOUND)?;
        let target = (*target).clone();

        Ok(target)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{apps::App, configuration::Config, davs::model::Dav, users::User};

    lazy_static::lazy_static! {
        static ref APPS: Vec<App> = {
            vec![
                App {
                    id: 1,
                    name: "App 1".to_owned(),
                    icon: "app_1_icon".to_owned(),
                    color: "#010101".to_owned(),
                    is_proxy: true,
                    host: "app1".to_owned(),
                    forward_to: "192.168.1.8".to_owned(),
                    secured: true,
                    login: "admin".to_owned(),
                    password: "ff54fds6f".to_owned(),
                    openpath: "".to_owned(),
                    roles: vec!["ADMINS".to_owned(), "USERS".to_owned()],
                },
                App {
                    id: 2,
                    name: "App 2".to_owned(),
                    icon: "app_2_icon".to_owned(),
                    color: "#020202".to_owned(),
                    is_proxy: false,
                    host: "app2".to_owned(),
                    forward_to: "localhost:8081".to_owned(),
                    secured: true,
                    login: "admin".to_owned(),
                    password: "ff54fds6f".to_owned(),
                    openpath: "/javascript_simple.html".to_owned(),
                    roles: vec!["ADMINS".to_owned()],
                },
            ]
        };

        static ref DAVS: Vec<Dav> = {
            vec![
                    Dav {
                    id: 1,
                    host: "files1".to_owned(),
                    directory: "/data/file1".to_owned(),
                    writable: true,
                    name: "Files 1".to_owned(),
                    icon: "file-invoice".to_owned(),
                    color: "#2ce027".to_owned(),
                    secured: true,
                    allow_symlinks: false,
                    roles: vec!["ADMINS".to_owned(),"USERS".to_owned()],
                    passphrase: "ABCD123".to_owned(),
                    key: None
                },
                Dav {
                    id: 2,
                    host: "files2".to_owned(),
                    directory: "/data/file2".to_owned(),
                    writable: true,
                    name: "Files 2".to_owned(),
                    icon: "file-invoice".to_owned(),
                    color: "#2ce027".to_owned(),
                    secured: true,
                    allow_symlinks: true,
                    roles: vec!["USERS".to_owned()],
                    passphrase: "".to_owned(),
                    key: None
                },
            ]
        };

        static ref USERS: Vec<User> = {
            vec![
                User {
                    login: "admin".to_owned(),
                    password: "password".to_owned(),
                    roles: vec!["ADMINS".to_owned()],
                },
                User {
                    login: "user".to_owned(),
                    password: "password".to_owned(),
                    roles: vec!["USERS".to_owned()],
                },
            ]
        };
    }

    #[tokio::test]
    async fn test_config_to_file_and_back() {
        // Arrange
        let config = Config {
            hostname: "vestibule.io".to_owned(),
            debug_mode: false,
            http_port: 8080,
            auto_tls: false,
            letsencrypt_email: "foo@bar.com".to_owned(),
            apps: APPS.clone(),
            davs: DAVS.clone(),
            users: USERS.clone(),
        };

        // Act
        let filepath = "config_test.yaml";
        config.to_file(filepath).await.unwrap();
        let new_config = Config::from_file(filepath).await.unwrap();

        // Assert
        assert_eq!(new_config, config);

        // Tidy
        fs::remove_file(filepath).unwrap();
    }
}
