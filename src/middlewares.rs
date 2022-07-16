use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use headers::HeaderValue;

pub async fn strip_port_from_host_middleware<B>(
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode>
where
    B: Send,
{
    match request.headers().get("host") {
        Some(host) => {
            let host = HeaderValue::from_str(
                host.to_str()
                    .expect("Invalid hostname")
                    .split(":")
                    .next()
                    .expect("Invalid hostname"),
            )
            .expect("Invalid hostname");
            request.headers_mut().insert("host", host);
        }
        None => {}
    }

    Ok(next.run(request).await)
}
