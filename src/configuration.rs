use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::apps::App;
use crate::davs::model::Dav;

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
}

pub type ConfigMap = HashMap<String, HostType>;

impl Config {
    pub fn from_file(filepath: &str) -> Result<Self> {
        let data = std::fs::read_to_string(filepath)?;
        let config = serde_yaml::from_str::<Config>(&data)?;
        Ok(config)
    }

    pub fn to_file(&self, filepath: &str) -> Result<()> {
        let contents = serde_yaml::to_string::<Config>(self)?;
        std::fs::write(filepath, contents)?;
        Ok(())
    }
}

pub async fn load_config(config_file: &str) -> Result<(Config, Arc<ConfigMap>), anyhow::Error> {
    let config = Config::from_file(config_file)?;
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
            (
                format!("{}.{}", dav.host.to_owned(), config.hostname),
                HostType::Dav(dav.clone()),
            )
        }))
        .collect();
    Ok((config, Arc::new(hashmap)))
}

/*pub async fn reload_config(config: &Arc<Mutex<Config>>) -> Result<(), anyhow::Error> {
    let mut config = &mut *config.lock().await;
    let config_file: String = config.config_file.clone();
    *config = Config::from_file(config_file.as_str())?;
    config.config_file = config_file.to_owned();
    Ok(())
}*/

#[derive(PartialEq, Debug)]
pub enum HostType {
    App(App),
    Dav(Dav),
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{apps::App, configuration::Config, davs::model::Dav};

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
                    passphrase: "ABCD123".to_owned()
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
                    passphrase: "".to_owned()
                },
            ]
        };
    }

    #[test]
    fn test_config_to_file_and_back() {
        // Arrange
        let config = Config {
            hostname: "vestibule.io".to_owned(),
            debug_mode: false,
            http_port: 8080,
            auto_tls: false,
            letsencrypt_email: "foo@bar.com".to_owned(),
            apps: APPS.clone(),
            davs: DAVS.clone(),
        };

        // Act
        let filepath = "config_test.yaml";
        config.to_file(filepath).unwrap();
        let new_config = Config::from_file(filepath).unwrap();

        // Assert
        assert_eq!(new_config, config);

        // Tidy
        fs::remove_file(filepath).unwrap();
    }

    /*#[tokio::test]
    async fn test_reload_configuration() {
        // Arrange
        let config = Config {
            hosts_map: HashMap::new(),
            config_file: "".to_owned(),
            debug_mode: false,
            http_port: 6666,
            apps: APPS.clone(),
            davs: DAVS.clone(),
        };
        let filepath = "config_test_2.yaml";
        config.to_file(filepath).unwrap();

        // Act
        let shared_config = load_config("config_test_2.yaml")
            .await
            .expect("Failed to load configuration");
        reload_config(&shared_config)
            .await
            .expect("Failed to reload configuration");
        assert_eq!(shared_config.lock().await.http_port, 6666);

        // Tidy
        fs::remove_file(filepath).unwrap();
    }*/
}
