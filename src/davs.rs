use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dav {
    pub id: i64,
    pub host: String,
    pub root: String,
    pub writable: bool,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub secured: bool,
    pub roles: Vec<String>,
    pub passphrase: String,
}
