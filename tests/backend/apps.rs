use crate::helpers::{create_apps_file, TestApp};
use std::fs;

#[tokio::test]
async fn proxy_test() {
    // Arrange
    let port = 8080;
    let app = TestApp::spawn(port).await;

    // Act
    let response = app
        .client
        .get(format!("http://vestibule.io:{port}"))
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
        .get(format!("http://app1.vestibule.io:{port}"))
        .send()
        .await
        .expect("failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from mock server 1"));

    // Tidy
    fs::remove_file(app.config_file).ok();
}

#[tokio::test]
async fn reload_test() {
    // Arrange
    let port = 8090;
    let mut app = TestApp::spawn(port).await;
    create_apps_file(&app.config_file, &port, true);

    app.client
        .get(format!("http://vestibule.io:{port}/reload"))
        .send()
        .await
        .expect("failed to execute request");

    app.is_ready().await;

    // Act
    let response = app
        .client
        .get(format!("http://app2.vestibule.io:{port}"))
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
        .get(format!("http://app2-altered.vestibule.io:{port}"))
        .send()
        .await
        .expect("failed to execute request");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from mock server 2"));

    // Tidy
    fs::remove_file(app.config_file).ok();
}
