use hyper::StatusCode;

use crate::helpers::TestApp;

#[tokio::test]
async fn list_services_api_for_unlogged_user_test() {
    // Arrange
    let app = TestApp::spawn().await;
    // Do not log

    // Act and Assert : Get the services (must fail)
    let response = app
        .client
        .get(format!(
            "http://vestibule.io:{}/api/user/list_services",
            app.port
        ))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response.text().await.unwrap(), "");
}

#[tokio::test]
async fn list_services_api_for_normal_user_test() {
    // Arrange
    let app = TestApp::spawn().await;
    // Log as user
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/auth/local", app.port))
        .body(r#"{"login":"user","password":"password"}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Act and Assert : Get the services (must fail)
    let response = app
        .client
        .get(format!(
            "http://vestibule.io:{}/api/user/list_services",
            app.port
        ))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    let response_content = response.text().await.unwrap();
    // Assert that apps and davs for users are present
    println!("Response content is: {}", response_content);
    assert!(response_content.contains("app1"));
    assert!(response_content.contains("files1"));
    // Assert that apps and davs for admins are not present
    assert!(!response_content.contains("secured-app"));
    assert!(!response_content.contains("secured-files"));
    assert!(!response_content.contains("ff54fds6f"));
    assert!(!response_content.contains("ABCD123"));
    assert!(response_content.contains(r#"login":"REDACTED"#));
    assert!(response_content.contains(r#"password":"REDACTED"#));
    assert!(response_content.contains(r#"passphrase":"REDACTED"#));
}
