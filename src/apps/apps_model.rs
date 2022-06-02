use anyhow::Result;
use serde_derive::Deserialize;
use serde_derive::Serialize;

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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Apps(Vec<App>);

impl Apps {
    pub fn from_file(filepath: &str) -> Result<Self> {
        let data = std::fs::read_to_string(filepath)?;
        let apps = serde_json::from_str::<Apps>(&data)?;
        Ok(apps)
    }

    pub fn to_file(&self, filepath: &str) -> Result<()> {
        let contents = serde_json::to_string_pretty::<Apps>(self)?;
        std::fs::write(filepath, contents)?;
        Ok(())
    }
}

#[cfg(test)]
#[test]
fn test_apps_to_file_and_back() {
    // arrange
    let apps = Apps(vec![
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
    ]);

    // act
    apps.to_file("apps.json").unwrap();
    let new_apps = Apps::from_file("apps.json").unwrap();

    // assert
    assert_eq!(new_apps, apps);
}
