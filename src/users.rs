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

use serde::Deserialize;
use serde::Serialize;

static COOKIE_NAME: &str = "SESSION";

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
        COOKIE_NAME, cookie, hostname
    );

    // Set cookie
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());

    (headers, Redirect::to("/"))
}
