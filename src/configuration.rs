use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use tokio::sync::Mutex;

use crate::apps::App;
use crate::davs::Dav;

fn debug_mode() -> bool {
    false
}

fn http_port() -> u16 {
    8080
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    // The config file and the hashmap are generated dynamically, and stored for future processing, but not serialized.
    #[serde(skip)]
    pub config_file: String,
    #[serde(skip)]
    pub hosts_map: HashMap<String, HostType>,
    #[serde(default = "debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "http_port")]
    pub http_port: u16,
    pub apps: Vec<App>,
    pub davs: Vec<Dav>,
}

impl Config {
    pub fn from_file(filepath: &str) -> Result<Self> {
        let data = std::fs::read_to_string(filepath)?;
        let mut config = serde_yaml::from_str::<Config>(&data)?;
        config.config_file = filepath.to_owned();
        config.generate_hosts_map();
        Ok(config)
    }

    pub fn to_file(&self, filepath: &str) -> Result<()> {
        let contents = serde_yaml::to_string::<Config>(self)?;
        std::fs::write(filepath, contents)?;
        Ok(())
    }

    fn generate_hosts_map(&mut self) -> &Self {
        self.hosts_map = self
            .apps
            .iter()
            .map(|app| (app.host.to_owned(), HostType::App(app.clone())))
            .chain(
                self.davs
                    .iter()
                    .map(|dav| (dav.host.to_owned(), HostType::Dav(dav.clone()))),
            )
            .collect();
        self
    }
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.debug_mode == other.debug_mode
            && self.http_port == other.http_port
            && self.apps == other.apps
            && self.davs == other.davs
    }
}

pub async fn load_config(config_file: &str) -> Result<Arc<Mutex<Config>>, anyhow::Error> {
    let config = Config::from_file(config_file)?;
    Ok(Arc::new(Mutex::new(config)))
}

pub async fn reload_config(config: &Arc<Mutex<Config>>) -> Result<(), anyhow::Error> {
    let mut config = &mut *config.lock().await;
    let config_file: String = config.config_file.clone();
    *config = Config::from_file(config_file.as_str())?;
    config.config_file = config_file.to_owned();
    Ok(())
}

#[derive(PartialEq, Debug)]
pub enum HostType {
    App(App),
    Dav(Dav),
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};

    use crate::{
        apps::App,
        configuration::{load_config, reload_config, Config},
        davs::Dav,
    };

    lazy_static::lazy_static! {
        static ref APPS: Vec<App> = {
            vec![
                App {
                    id: 1,
                    name: "App 1".to_owned(),
                    icon: "app_1_icon".to_owned(),
                    color: "#010101".to_owned(),
                    is_proxy: true,
                    host: "app1.vestibule.io".to_owned(),
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
                    host: "app2.vestibule.io".to_owned(),
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
                    host: "files1.vestibule.io".to_owned(),
                    root: "/data/file1".to_owned(),
                    writable: true,
                    name: "Files 1".to_owned(),
                    icon: "file-invoice".to_owned(),
                    color: "#2ce027".to_owned(),
                    secured: true,
                    roles: vec!["ADMINS".to_owned(),"USERS".to_owned()],
                    passphrase: "ABCD123".to_owned()
                },
                Dav {
                    id: 2,
                    host: "files2.vestibule.io".to_owned(),
                    root: "/data/file2".to_owned(),
                    writable: true,
                    name: "Files 2".to_owned(),
                    icon: "file-invoice".to_owned(),
                    color: "#2ce027".to_owned(),
                    secured: true,
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
            hosts_map: HashMap::new(),
            config_file: "".to_owned(),
            debug_mode: false,
            http_port: 8080,
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

    #[tokio::test]
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
    }
}
