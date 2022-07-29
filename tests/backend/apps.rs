use axum::{response::Redirect, routing::get, Router};
use hyper::header::LOCATION;
use vestibule::apps::App;

use crate::helpers::TestApp;
use std::{fs, net::TcpListener};

#[tokio::test]
async fn secured_proxy_test() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act : try to access app as unlogged user
    let response = app
        .client
        .get(format!("http://secured-app.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");

    // Assert that is impossible
    assert!(response.status() == 403);
    assert_eq!(response.text().await.unwrap(), "");

    // Log as normal user
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/auth/local", app.port))
        .body(r#"{"login":"user","password":"password"}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert!(response.status().is_success());
    // Act : try to access app as logged user
    let response = app
        .client
        .get(format!("http://secured-app.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");
    // Assert that is impossible
    assert!(response.status() == 403);
    assert_eq!(response.text().await.unwrap(), "");

    // Log as admin
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/auth/local", app.port))
        .body(r#"{"login":"admin","password":"password"}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert!(response.status().is_success());
    // Act : try to access app as admin
    let response = app
        .client
        .get(format!("http://secured-app.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");
    // Assert that is possible
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from mock server"));
}

#[tokio::test]
async fn proxy_test() {
    // Arrange
    let app = TestApp::spawn().await;

    // Act
    let response = app
        .client
        .get(format!("http://vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from main server !"));

    // Act
    let response = app
        .client
        .get(format!("http://app1.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from mock server"));
}

#[tokio::test]
async fn reload_test() {
    // Arrange
    let mut app = TestApp::spawn().await;
    // alter the configuration file
    let fp = format!("{}.yaml", &app.id);
    let mut src = fs::File::open(&fp).expect("failed to open config file");
    let mut data = String::new();
    std::io::Read::read_to_string(&mut src, &mut data).expect("failed to read config file");
    drop(src);
    let new_data = data.replace("app2", "app2-altered");
    let mut dst = fs::File::create(&fp).expect("could not create file");
    std::io::Write::write(&mut dst, new_data.as_bytes()).expect("failed to write to file");

    app.client
        .get(format!("http://vestibule.io:{}/reload", app.port))
        .send()
        .await
        .expect("failed to execute request");

    app.is_ready().await;

    // Act
    let response = app
        .client
        .get(format!("http://app2.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from main server !"));

    // Act
    let response = app
        .client
        .get(format!("http://app2-altered.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from mock server"));
}

#[tokio::test]
async fn redirect_test() {
    // ARRANGE
    // Create base test app
    let mut app = TestApp::spawn().await;
    // Spawn 3 targets with different redirect behaviors
    let fwdtoredirect_listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
    let fwdtoredirect_port = fwdtoredirect_listener.local_addr().unwrap().port();
    let relativeredirect_listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
    let relativeredirect_port = relativeredirect_listener.local_addr().unwrap().port();
    let absoluteredirect_listener =
        std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
    let absoluteredirect_port = absoluteredirect_listener.local_addr().unwrap().port();
    tokio::spawn(fwdtoredirect_server(fwdtoredirect_listener));
    tokio::spawn(relativeredirect_server(relativeredirect_listener, app.port));
    tokio::spawn(absoluteredirect_server(absoluteredirect_listener));
    // Alter apps to proxify to those targets
    let filepath = format!("{}.yaml", &app.id);
    let apps = vec![
        App {
            id: 1,
            name: "fwdtoredirect".to_owned(),
            icon: "fwdtoredirect_icon".to_owned(),
            color: "#010101".to_owned(),
            is_proxy: false,
            host: "fwdtoredirect".to_owned(),
            forward_to: format!("localhost:{fwdtoredirect_port}"),
            secured: false,
            login: "".to_owned(),
            password: "".to_owned(),
            openpath: "".to_owned(),
            roles: vec![],
        },
        App {
            id: 1,
            name: "relativeredirect".to_owned(),
            icon: "relativeredirect_icon".to_owned(),
            color: "#010101".to_owned(),
            is_proxy: false,
            host: "relativeredirect".to_owned(),
            forward_to: format!("localhost:{relativeredirect_port}"),
            secured: false,
            login: "".to_owned(),
            password: "".to_owned(),
            openpath: "".to_owned(),
            roles: vec![],
        },
        App {
            id: 1,
            name: "absoluteredirect".to_owned(),
            icon: "absoluteredirect_icon".to_owned(),
            color: "#010101".to_owned(),
            is_proxy: false,
            host: "absoluteredirect".to_owned(),
            forward_to: format!("localhost:{absoluteredirect_port}"),
            secured: false,
            login: "".to_owned(),
            password: "".to_owned(),
            openpath: "".to_owned(),
            roles: vec![],
        },
    ];

    let config = vestibule::configuration::Config {
        hostname: "vestibule.io".to_owned(),
        debug_mode: true,
        auto_tls: false,
        letsencrypt_email: "foo@bar.com".to_owned(),
        http_port: app.port,
        apps: apps,
        davs: vec![],
        users: vec![],
    };
    config.to_file(&filepath).await.unwrap();
    app.client
        .get(format!("http://vestibule.io:{}/reload", app.port))
        .send()
        .await
        .expect("failed to execute request");
    app.is_ready().await;

    // ACT and ASSERT
    // Make requests to those apps and test the results

    // Redirect must be altered when is related to the proxied host and not to the exposed host
    let response = app
        .client
        .get(format!("http://fwdtoredirect.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert!(response.status().is_redirection());
    info!("Location Header : {:?}", response.headers()[LOCATION]);
    assert_eq!(
        response.headers()[LOCATION],
        format!("http://fwdtoredirect.vestibule.io:{}/some/path", app.port)
    );

    // Redirect must be kept intact when is made to the exposed host
    let response = app
        .client
        .get(format!("http://relativeredirect.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert!(response.status().is_redirection());
    info!("Location Header : {:?}", response.headers()[LOCATION]);
    assert_eq!(
        response.headers()[LOCATION],
        format!(
            "http://relative.redirect.relativeredirect.vestibule.io:{}",
            app.port
        )
    );

    // Redirect must be kept intact when is to another website
    let response = app
        .client
        .get(format!("http://absoluteredirect.vestibule.io:{}", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert!(response.status().is_redirection());
    info!("Location Header : {:?}", response.headers()[LOCATION]);
    assert_eq!(response.headers()[LOCATION], "http://absolute.redirect");
}

pub async fn fwdtoredirect_server(listener: TcpListener) {
    let port = listener.local_addr().unwrap().port();
    let app = Router::new().route(
        "/",
        get(move || async move {
            Redirect::permanent(
                format!("http://fwdto.redirect.bad.localhost:{}/some/path", port).as_str(),
            )
        }),
    );
    axum::Server::from_tcp(listener)
        .expect("failed to build mock server")
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn relativeredirect_server(listener: TcpListener, app_port: u16) {
    let app = Router::new().route(
        "/",
        get(move || async move {
            Redirect::permanent(
                format!("http://relative.redirect.relativeredirect.vestibule.io:{app_port}")
                    .as_str(),
            )
        }),
    );
    axum::Server::from_tcp(listener)
        .expect("failed to build mock server")
        .serve(app.into_make_service())
        .await
        .unwrap();
}

pub async fn absoluteredirect_server(listener: TcpListener) {
    let app = Router::new().route(
        "/",
        get(|| async { Redirect::permanent("http://absolute.redirect") }),
    );
    axum::Server::from_tcp(listener)
        .expect("failed to build mock server")
        .serve(app.into_make_service())
        .await
        .unwrap();
}
