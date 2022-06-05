use reqwest::Client;
use std::{collections::HashMap, net::SocketAddr};

use vestibule::{apps::App, configuration::Config, server::Server, utils::random_string};

pub struct TestApp {
    pub client: Client,
    pub config_file: String,
}

pub async fn spawn_app(port: u16) -> TestApp {
    // Launch the application as a background task
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let filepath = format!("{}.yaml", random_string());
    create_apps_file(&filepath, &port, false);

    let app = Server::build(&filepath)
        .await
        .expect("Could not create app");

    let _ = tokio::spawn(
        axum::Server::bind(&addr).serve(
            app.router
                .into_make_service_with_connect_info::<SocketAddr>(),
        ),
    );

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .resolve("vestibule.io", addr)
        .resolve("app1.vestibule.io", addr)
        .resolve("app2.vestibule.io", addr)
        .resolve("app2-altered.vestibule.io", addr)
        .cookie_store(true)
        .build()
        .unwrap();

    let test_app = TestApp {
        client: client,
        config_file: filepath,
    };

    test_app
}

pub fn create_apps_file(filepath: &str, port: &u16, altered: bool) {
    let app2host = if altered {
        "app2-altered.vestibule.io"
    } else {
        "app2.vestibule.io"
    };
    let apps = vec![
        App {
            id: 1,
            name: "App 1".to_owned(),
            icon: "app_1_icon".to_owned(),
            color: "#010101".to_owned(),
            is_proxy: true,
            host: "app1.vestibule.io".to_owned(),
            forward_to: format!("localhost:{}", port + 1),
            secured: true,
            login: "admin".to_owned(),
            password: "ff54fds6f".to_owned(),
            openpath: "".to_owned(),
            roles: vec!["ADMINS".to_owned(), "USERS".to_owned()],
        },
        App {
            id: 2,
            name: "App 2".to_owned(),
            icon: "app_2_icon".to_owned(),
            color: "#020202".to_owned(),
            is_proxy: false,
            host: app2host.to_owned(),
            forward_to: format!("localhost:{}", port + 2),
            secured: true,
            login: "admin".to_owned(),
            password: "ff54fds6f".to_owned(),
            openpath: "/javascript_simple.html".to_owned(),
            roles: vec!["ADMINS".to_owned()],
        },
    ];

    let config = Config {
        hosts_map: HashMap::new(),
        config_file: filepath.to_owned(),
        debug_mode: true,
        http_port: *port,
        apps: apps,
        davs: vec![],
    };

    // Act
    config.to_file(filepath).unwrap();
}
