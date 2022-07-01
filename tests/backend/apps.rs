use crate::helpers::TestApp;
use std::fs;

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
    let new_data = data.replace("app2.vestibule.io", "app2-altered.vestibule.io");
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
