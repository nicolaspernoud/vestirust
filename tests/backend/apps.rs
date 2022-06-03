use crate::helpers::{create_apps_file, spawn_app};
use std::fs;

#[tokio::test]
async fn proxy_test() {
    // Arrange
    let port = 8080;
    let app = spawn_app(port).await;

    // Act
    let response = app
        .client
        .get(format!("http://vestibule.io:{port}"))
        .send()
        .await
        .expect("Failed to execute request.");

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
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from mock server 1"));

    // Tidy
    fs::remove_file(app.config_file).unwrap();
}

#[tokio::test]
async fn reload_test() {
    // Arrange
    let port = 8090;
    let app = spawn_app(port).await;

    create_apps_file(&app.config_file, &port, true);

    app.client
        .get(format!("http://vestibule.io:{port}/reload"))
        .send()
        .await
        .expect("Failed to execute request.");

    // Act
    let response = app
        .client
        .get(format!("http://app2.vestibule.io:{port}"))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert_eq!(response.status().as_u16(), 404);

    // Act
    let response = app
        .client
        .get(format!("http://app2-altered.vestibule.io:{port}"))
        .send()
        .await
        .expect("Failed to execute request.");

    // Assert
    assert!(response.status().is_success());
    assert!(response
        .text()
        .await
        .unwrap()
        .contains("Hello world from mock server 2"));

    // Tidy
    fs::remove_file(app.config_file).unwrap();
}
