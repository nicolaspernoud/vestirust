use std::sync::Arc;

use anyhow::Result;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use tokio::sync::Mutex;

use crate::apps::App;

fn debug_mode() -> bool {
    false
}

fn main_hostname() -> String {
    "localhost".to_owned()
}

fn http_port() -> u16 {
    8080
}

#[derive(Deserialize, Serialize, Debug, Default, PartialEq)]
pub struct Config {
    // The config file is used for future reference, but it is not serialized
    #[serde(skip)]
    pub config_file: String,
    #[serde(default = "debug_mode")]
    pub debug_mode: bool,
    #[serde(default = "main_hostname")]
    pub main_hostname: String,
    #[serde(default = "http_port")]
    pub http_port: u16,
    pub apps: Vec<App>,
}

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

pub async fn load_config(config_file: &str) -> Result<Arc<Mutex<Config>>, anyhow::Error> {
    let mut config = Config::from_file(config_file)?;
    config.config_file = config_file.to_owned();
    Ok(Arc::new(Mutex::new(config)))
}

pub async fn reload_config(config: &Arc<Mutex<Config>>) -> Result<(), anyhow::Error> {
    let mut config = &mut *config.lock().await;
    let config_file: String = config.config_file.clone();
    *config = Config::from_file(config_file.as_str())?;
    config.config_file = config_file.to_owned();
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{
        apps::App,
        configuration::{load_config, reload_config, Config},
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
       }

    #[test]
    fn test_config_to_file_and_back() {
        // Arrange
        let config = Config {
            config_file: "".to_owned(),
            debug_mode: false,
            main_hostname: "localhost".to_owned(),
            http_port: 8080,
            apps: APPS.clone(),
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
            config_file: "".to_owned(),
            debug_mode: false,
            main_hostname: "localhost".to_owned(),
            http_port: 6666,
            apps: APPS.clone(),
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
