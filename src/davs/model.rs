use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use hyper::StatusCode;
use serde::Deserialize;
use serde::Serialize;

use crate::configuration::Config;
use crate::configuration::ConfigFile;
use crate::users::Admin;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dav {
    pub id: usize,
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
    #[serde(skip)]
    pub key: Option<[u8; 32]>,
}

pub async fn get_davs(
    config: Config,
    _admin: Admin,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // Return all the davs as Json
    let encoded = serde_json::to_string(&config.davs).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not encode davs".to_owned(),
        )
    })?;
    Ok((StatusCode::OK, encoded))
}

pub async fn delete_dav(
    config_file: Extension<ConfigFile>,
    mut config: Config,
    _admin: Admin,
    Path(dav_id): Path<(String, usize)>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Find the dav
    if let Some(pos) = config.davs.iter().position(|d| d.id == dav_id.1) {
        // It is an existing dav, delete it
        config.davs.remove(pos);
    } else {
        // If the dav doesn't exist, respond with an error
        return Err((StatusCode::BAD_REQUEST, "dav doesn't exist"));
    }

    config
        .to_file_or_internal_server_error(&config_file)
        .await?;

    Ok((StatusCode::OK, "dav deleted successfully"))
}

pub async fn add_dav(
    config_file: Extension<ConfigFile>,
    mut config: Config,
    _admin: Admin,
    Json(payload): Json<Dav>,
) -> Result<(StatusCode, &'static str), (StatusCode, &'static str)> {
    // Find the dav
    if let Some(dav) = config.davs.iter_mut().find(|d| d.id == payload.id) {
        *dav = payload;
    } else {
        config.davs.push(payload);
    }

    config
        .to_file_or_internal_server_error(&config_file)
        .await?;

    Ok((StatusCode::CREATED, "dav created or updated successfully"))
}
