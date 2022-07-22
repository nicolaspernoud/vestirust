use hyper::StatusCode;

use crate::helpers::TestApp;

#[tokio::test]
async fn users_api_for_unlogged_user_test() {
    // Arrange
    let app = TestApp::spawn().await;
    // Do not log

    // Get the existing users (must fail)
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Try to add an user (must fail)
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .body(r#"{"login":"nicolas","password":"verystrongpassword","roles":["ADMINS"]}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Try to remove an user (must fail)
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

    // Get the existing users (must fail)
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Try to add an user (must fail)
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/users", app.port))
        .body(r#"{"login":"nicolas","password":"verystrongpassword","roles":["ADMINS"]}"#)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Try to remove an user (must fail)
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

const NEW_APP: &'static str = r##"
{
    "id": 101,
    "name": "App101",
    "icon": "app_101_icon",
    "color": "#010101",
    "is_proxy": true,
    "host": "app101",
    "forward_to": "localhost:8081",
    "secured": false,
    "login": "admin",
    "password": "app101pwd",
    "openpath": "",
    "roles": ["ADMINS", "USERS"]
}
"##;

#[tokio::test]
async fn apps_api_for_unlogged_user_test() {
    // Arrange
    let app = TestApp::spawn().await;
    // Do not log

    // Get the existing apps (must fail)
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Add an app (must fail)
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .body(NEW_APP)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Remove an app (must fail)
    let response = app
        .client
        .delete(format!("http://vestibule.io:{}/api/admin/apps/1", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn apps_api_for_normal_user_test() {
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

    // Get the existing apps (must fail)
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Add an app (must fail)
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .body(NEW_APP)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Remove an app (must fail)
    let response = app
        .client
        .delete(format!("http://vestibule.io:{}/api/admin/apps/1", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn apps_api_for_admin_user_test() {
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

    // Get the existing apps
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().starts_with(r#"[{"id":"#));

    // Add an app and assert that it has been added
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .body(NEW_APP)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().contains(r#""id":101"#));

    // Remove an app and assert that it is not here anymore
    let response = app
        .client
        .delete(format!(
            "http://vestibule.io:{}/api/admin/apps/101",
            app.port
        ))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/apps", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.text().await.unwrap().contains(r#""id":101"#));
}

const NEW_DAV: &'static str = r##"
{
    "id": 201,
    "host": "files101",
    "directory": "./data/dir2",
    "writable": true,
    "name": "Files101",
    "icon": "file-invoice",
    "color": "#2ce027",
    "secured": false,
    "allow_symlinks": false,
    "roles": ["USERS"],
    "passphrase": "ABCD101"
}
"##;

#[tokio::test]
async fn davs_api_for_unlogged_user_test() {
    // Arrange
    let app = TestApp::spawn().await;
    // Do not log

    // Get the existing davs (must fail)
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Add a dav (must fail)
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .body(NEW_DAV)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Remove a dav (must fail)
    let response = app
        .client
        .delete(format!("http://vestibule.io:{}/api/admin/davs/1", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn davs_api_for_normal_user_test() {
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

    // Get the existing davs (must fail)
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Add a dav (must fail)
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .body(NEW_DAV)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Remove a dav (must fail)
    let response = app
        .client
        .delete(format!("http://vestibule.io:{}/api/admin/davs/1", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn davs_api_for_admin_user_test() {
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

    // Get the existing davs
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().starts_with(r#"[{"id":"#));

    // Add a dav and assert that it is here
    let response = app
        .client
        .post(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .body(NEW_DAV)
        .header("Content-Type", "application/json")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::CREATED);
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.text().await.unwrap().contains(r#""id":201"#));

    // Remove a dav and assert that it is not here anymore
    let response = app
        .client
        .delete(format!(
            "http://vestibule.io:{}/api/admin/davs/201",
            app.port
        ))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    let response = app
        .client
        .get(format!("http://vestibule.io:{}/api/admin/davs", app.port))
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.text().await.unwrap().contains(r#""id":201"#));
}
