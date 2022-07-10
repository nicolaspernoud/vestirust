use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dav {
    pub id: i64,
    pub host: String,
    pub directory: String,
    pub writable: bool,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub secured: bool,
    #[serde(default)]
    pub allow_symlinks: bool,
    pub roles: Vec<String>,
    pub passphrase: String,
}
