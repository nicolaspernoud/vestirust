use hyper::StatusCode;

use crate::helpers::TestApp;

#[tokio::test]
async fn users_api_for_unlogged_user_test() {
    // Arrange
    let app = TestApp::spawn().await;
    // Do not log

    // Get the existing users
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Add an user and assert that he is here
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .body(r#"{"login":"nicolas","password":"verystrongpassword","roles":["ADMINS"]}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Remove an user and assert that he is not here anymore
    let response = app
        .client
        .delete(format!(
            "http://vestibule.io:{}/api/admin/users/nicolas",
            app.port
        ))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn users_api_for_normal_user_test() {
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

    // Get the existing users
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Add an user and assert that he is here
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .body(r#"{"login":"nicolas","password":"verystrongpassword","roles":["ADMINS"]}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Remove an user and assert that he is not here anymore
    let response = app
        .client
        .delete(format!(
            "http://vestibule.io:{}/api/admin/users/nicolas",
            app.port
        ))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn users_api_for_admin_user_test() {
    // Arrange
    let app = TestApp::spawn().await;
    // Log as admin
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/auth/local", app.port))
        .body(r#"{"login":"admin","password":"password"}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Get the existing users
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().starts_with(r#"[{"login":"#));

    // Add an user and assert that he is here
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .body(r#"{"login":"nicolas","password":"verystrongpassword","roles":["ADMINS"]}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().contains(r#"nicolas"#));

    // Remove an user and assert that he is not here anymore
    let response = app
        .client
        .delete(format!(
            "http://vestibule.io:{}/api/admin/users/nicolas",
            app.port
        ))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.text().await.unwrap().contains(r#"nicolas"#));
}
