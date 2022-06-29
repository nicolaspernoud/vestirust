use reqwest::Client;
use std::net::SocketAddr;
use tokio::sync::broadcast;

use vestibule::{
    apps::App, configuration::Config, mocks::mock_proxied_server, server::Server,
    utils::random_string,
};

pub struct TestApp {
    pub client: Client,
    pub config_file: String,
    pub server_started: tokio::sync::broadcast::Receiver<()>,
}

impl TestApp {
    pub async fn is_ready(&mut self) {
        self.server_started
            .recv()
            .await
            .expect("could not start server");
    }

    pub async fn spawn(port: u16) -> Self {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let filepath = format!("{}.yaml", random_string());
        create_apps_file(&filepath, &port, false);

        tokio::spawn(mock_proxied_server(port, 1));
        tokio::spawn(mock_proxied_server(port, 2));

        let (tx, _) = broadcast::channel(16);
        let fp = filepath.clone();

        let (server_status, server_started) = broadcast::channel(16);

        let _ = tokio::spawn(async move {
            loop {
                info!("Configuration read !");
                let mut rx = tx.subscribe();
                let app = Server::build(&fp, tx.clone())
                    .await
                    .expect("could not build server from configuration");
                let addr = SocketAddr::from(([127, 0, 0, 1], app.port));
                let server = axum::Server::bind(&addr)
                    .serve(
                        app.router
                            .into_make_service_with_connect_info::<SocketAddr>(),
                    )
                    .with_graceful_shutdown(async move {
                        rx.recv().await.expect("Could not receive reload command!");
                    });
                server_status.send(()).expect("could not send message");
                server.await.expect("could not start server");
            }
        });

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .resolve("vestibule.io", addr)
            .resolve("app1.vestibule.io", addr)
            .resolve("app2.vestibule.io", addr)
            .resolve("app2-altered.vestibule.io", addr)
            .cookie_store(true)
            .build()
            .unwrap();

        let mut test_app = TestApp {
            client: client,
            config_file: filepath,
            server_started: server_started,
        };

        test_app.is_ready().await;

        test_app
    }
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
        debug_mode: true,
        http_port: *port,
        apps: apps,
        davs: vec![],
    };

    // Act
    config.to_file(filepath).unwrap();
}
