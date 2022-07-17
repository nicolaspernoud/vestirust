use axum::async_trait;

use axum::extract::FromRequest;
use axum::extract::Host;
use axum::extract::RequestParts;

use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::SignedCookieJar;
use hyper::Body;
use hyper::StatusCode;

use serde::Deserialize;
use serde::Serialize;

use crate::configuration::HostType;

static COOKIE_NAME: &str = "VESTIBULE_AUTH";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    pub id: usize,
    pub login: String,
    pub password: String,
    pub roles: Vec<String>,
}

pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        //Redirect::temporary("http://vestibule.127.0.0.1.nip.io:8080/auth/local").into_response()
        Redirect::temporary("/auth/local").into_response()
    }
}

#[async_trait]
impl<B> FromRequest<B> for User
where
    B: Send,
{
    // If anything goes wrong or no session is found, redirect to the auth page
    type Rejection = AuthRedirect;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let jar: SignedCookieJar = SignedCookieJar::from_request(req)
            .await
            .expect("Could not find cookie jar");

        if let Some(_cookie) = jar.get(COOKIE_NAME) {
            Ok(User {
                id: 1,
                login: "admin".to_owned(),
                password: "password".to_owned(),
                roles: vec!["ADMINS".to_owned()],
            })
        } else {
            Err(AuthRedirect)
        }
    }
}

pub async fn local_auth(
    jar: SignedCookieJar,
    Host(hostname): Host,
) -> Result<(SignedCookieJar, Redirect), StatusCode> {
    let domain = hostname
        .split(":")
        .next()
        .expect("No hostname found")
        .to_owned();

    let cookie = Cookie::build(COOKIE_NAME, "value")
        .domain(domain)
        .path("/")
        .secure(false)
        .http_only(false)
        .finish();

    Ok((jar.add(cookie), Redirect::to("/me")))
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
