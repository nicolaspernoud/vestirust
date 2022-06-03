use axum::{routing::get, Router};

use std::net::SocketAddr;

pub async fn mock_proxied_server(http_port: u16, server_id: u16) {
    let port = http_port + server_id;
    let message = format!("Hello world from mock server {server_id}!");
    let app = Router::new().route("/", get(move || async { message }));

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
