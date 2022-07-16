use async_session::async_trait;
use async_session::MemoryStore;
use async_session::Session;
use async_session::SessionStore;
use axum::extract::rejection::TypedHeaderRejectionReason;
use axum::extract::FromRequest;
use axum::extract::Host;
use axum::extract::RequestParts;
use axum::extract::TypedHeader;

use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::response::Response;
use axum::Extension;
use headers::HeaderMap;
use hyper::header;
use hyper::header::SET_COOKIE;
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
        let Extension(store) = Extension::<MemoryStore>::from_request(req)
            .await
            .expect("`MemoryStore` extension is missing");

        let cookies = TypedHeader::<headers::Cookie>::from_request(req)
            .await
            .map_err(|e| match *e.name() {
                header::COOKIE => match e.reason() {
                    TypedHeaderRejectionReason::Missing => AuthRedirect,
                    _ => panic!("unexpected error getting Cookie header(s): {}", e),
                },
                _ => panic!("unexpected error getting cookies: {}", e),
            })?;
        let session_cookie = cookies.get(COOKIE_NAME).ok_or(AuthRedirect)?;

        let session = store
            .load_session(session_cookie.to_string())
            .await
            .unwrap()
            .ok_or(AuthRedirect)?;

        let user = session.get::<User>("user").ok_or(AuthRedirect)?;

        Ok(user)
    }
}

pub async fn local_auth(
    Extension(store): Extension<MemoryStore>,
    Host(hostname): Host,
) -> impl IntoResponse {
    let user = User {
        id: 1,
        login: "admin".to_owned(),
        password: "password".to_owned(),
        roles: vec!["ADMINS".to_owned()],
    };

    // Create a new session filled with user data
    let mut session = Session::new();
    session.insert("user", &user).unwrap();

    // Store session and get corresponding cookie
    let cookie = store.store_session(session).await.unwrap().unwrap();

    // Build the cookie
    let cookie = format!(
        "{}={}; SameSite=Lax; Domain={}; Path=/",
        COOKIE_NAME,
        cookie,
        hostname.split(":").next().expect("No hostname found")
    );

    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (headers, Redirect::to("/"))
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
