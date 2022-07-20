use argon2::password_hash::SaltString;
use argon2::Argon2;
use argon2::PasswordHash;
use argon2::PasswordHasher;
use argon2::PasswordVerifier;
use axum::async_trait;
use axum::Json;

use axum::extract::FromRequest;
use axum::extract::Host;
use axum::extract::Path;
use axum::extract::RequestParts;

use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::SignedCookieJar;
use hyper::Body;
use hyper::StatusCode;

use rand::rngs::OsRng;
use serde::Deserialize;
use serde::Serialize;

use crate::configuration::Config;
use crate::configuration::HostType;
use crate::configuration::CONFIG_FILE;

static COOKIE_NAME: &str = "VESTIBULE_AUTH";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub password: String,
    pub roles: Vec<String>,
}

#[async_trait]
impl<B> FromRequest<B> for User
where
    B: Send,
{
    type Rejection = StatusCode;
    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let jar: SignedCookieJar = SignedCookieJar::from_request(req)
            .await
            .expect("Could not find cookie jar");

        // Get the serialized user from the cookie jar
        if let Some(cookie) = jar.get(COOKIE_NAME) {
            // Deserialize the user and return him/her
            let serialized_user = cookie.value();
            let user: User = serde_json::from_str(serialized_user)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(user)
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Admin(User);

#[async_trait]
impl<B> FromRequest<B> for Admin
where
    B: Send,
{
    type Rejection = StatusCode;
    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let user = User::from_request(req).await?;
        if !user.roles.contains(&"ADMINS".to_owned()) {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(Admin(user))
    }
}

#[derive(Deserialize)]
pub struct LocalAuth {
    login: String,
    password: String,
}

#[axum_macros::debug_handler]
pub async fn local_auth(
    jar: SignedCookieJar,
    Host(hostname): Host,
    Json(payload): Json<LocalAuth>,
) -> Result<(SignedCookieJar, StatusCode), StatusCode> {
    // Load configuration
    let mut config = Config::from_file(CONFIG_FILE)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Find the user in configuration
    let mut user = config
        .users
        .iter_mut()
        .find(|u| u.login == payload.login)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Check if the given password is correct
    let parsed_hash =
        PasswordHash::new(&user.password).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Clean the password from the cookie
    user.password = "".to_string();

    // Serialize him/her as a cookie value
    let encoded = serde_json::to_string(&user).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let domain = hostname
        .split(":")
        .next()
        .expect("No hostname found")
        .to_owned();

    // Store the user into the cookie
    let cookie = Cookie::build(COOKIE_NAME, encoded)
        .domain(domain)
        .path("/")
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .secure(false)
        .http_only(false)
        .finish();

    Ok((jar.add(cookie), StatusCode::OK))
}

pub async fn get_users(_admin: Admin) -> Result<(StatusCode, String), (StatusCode, String)> {
    // Load the configuration
    let config = Config::from_file(CONFIG_FILE).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not load configuration".to_owned(),
        )
    })?;
    // Return all the users as Json
    let encoded = serde_json::to_string(&config.users).map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not encode users".to_owned(),
        )
    })?;
    Ok((StatusCode::OK, encoded))
}

pub async fn delete_user(
    _admin: Admin,
    Path(user_login): Path<(String, String)>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Load the configuration
    let mut config = Config::from_file(CONFIG_FILE).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not load configuration",
        )
    })?;
    // Find the user
    if let Some(pos) = config.users.iter().position(|u| u.login == user_login.1) {
        // It is an existing user, delete it
        config.users.remove(pos);
    } else {
        // If the user doesn't exist, respond with an error
        return Err((StatusCode::BAD_REQUEST, "user doesn't exist"));
    }

    // Save the configuration
    config.to_file(CONFIG_FILE).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not save configuration",
        )
    })?;

    Ok((StatusCode::OK, "user deleted successfully"))
}

pub async fn add_user(
    _admin: Admin,
    Json(mut payload): Json<User>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // Load the configuration
    let mut config = Config::from_file(CONFIG_FILE).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not load configuration",
        )
    })?;
    // Find the user
    if let Some(user) = config.users.iter_mut().find(|u| u.login == payload.login) {
        // It is an existing user, we only hash the password if it is not empty
        if !payload.password.is_empty() {
            hash_password(&mut payload)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "password hash failed"))?;
        } else {
            payload.password = user.password.clone();
        }
        *user = payload;
    } else {
        // It is a new user, we need to hash the password
        if payload.password.is_empty() {
            return Err((StatusCode::NOT_ACCEPTABLE, "password is required"));
        }
        hash_password(&mut payload)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "password hash failed"))?;
        config.users.push(payload);
    }

    // Save the configuration
    config.to_file(CONFIG_FILE).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "could not save configuration",
        )
    })?;

    Ok((StatusCode::CREATED, "user created or updated successfully"))
}

fn hash_password(payload: &mut User) -> Result<(), argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    payload.password = argon2
        .hash_password(payload.password.trim().as_bytes(), &salt)?
        .to_string();
    Ok(())
}

pub fn check_user_has_role_or_forbid(
    user: &Option<User>,
    target: &HostType,
) -> Option<Response<Body>> {
    if let Some(user) = user {
        for user_role in user.roles.iter() {
            for role in target.roles().iter() {
                if user_role == role {
                    return None;
                }
            }
        }
    }
    Some(
        Response::builder()
            .status(StatusCode::FORBIDDEN)
            .body(Body::empty())
            .unwrap(),
    )
}

pub fn check_authorization(app: &HostType, user: &Option<User>) -> Option<Response<Body>> {
    if app.secured() {
        if let Some(response) = check_user_has_role_or_forbid(user, app) {
            return Some(response);
        }
    }
    None
}

#[cfg(test)]
mod check_user_has_role_or_forbid_tests {
    use crate::{
        apps::App,
        configuration::HostType,
        users::{check_user_has_role_or_forbid, User},
    };

    #[test]
    fn test_no_user() {
        let user = &None;
        let mut app: App = App::default();
        app.roles = vec!["role1".to_string(), "role2".to_string()];
        let target = HostType::App(app);
        assert!(check_user_has_role_or_forbid(user, &target).is_some());
    }

    #[test]
    fn test_user_has_all_roles() {
        let mut user = User::default();
        user.roles = vec!["role1".to_string(), "role2".to_string()];
        let mut app: App = App::default();
        app.roles = vec!["role1".to_string(), "role2".to_string()];
        let target = HostType::App(app);
        assert!(check_user_has_role_or_forbid(&Some(user), &target).is_none());
    }

    #[test]
    fn test_user_has_one_role() {
        let mut user = User::default();
        user.roles = vec!["role1".to_string()];
        let mut app: App = App::default();
        app.roles = vec!["role1".to_string(), "role2".to_string()];
        let target = HostType::App(app);
        assert!(check_user_has_role_or_forbid(&Some(user), &target).is_none());
    }

    #[test]
    fn test_user_has_no_role() {
        let mut user = User::default();
        user.roles = vec!["role3".to_string(), "role4".to_string()];
        let mut app: App = App::default();
        app.roles = vec!["role1".to_string(), "role2".to_string()];
        let target = HostType::App(app);
        assert!(check_user_has_role_or_forbid(&Some(user), &target).is_some());
    }

    #[test]
    fn test_user_roles_are_empty() {
        let user = User::default();
        let mut app: App = App::default();
        app.roles = vec!["role1".to_string(), "role2".to_string()];
        let target = HostType::App(app);
        assert!(check_user_has_role_or_forbid(&Some(user), &target).is_some());
    }

    #[test]
    fn test_allowed_roles_are_empty() {
        let mut user = User::default();
        user.roles = vec!["role1".to_string(), "role2".to_string()];
        let app: App = App::default();
        let target = HostType::App(app);
        assert!(check_user_has_role_or_forbid(&Some(user), &target).is_some());
    }

    #[test]
    fn test_all_roles_are_empty() {
        let user = User::default();
        let app: App = App::default();
        let target = HostType::App(app);
        assert!(check_user_has_role_or_forbid(&Some(user), &target).is_some());
    }
}
