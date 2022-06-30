use reqwest::Client;
use std::net::SocketAddr;
use tokio::sync::broadcast;

use vestibule::{
    apps::App, configuration::Config, davs::model::Dav, mocks::mock_proxied_server, server::Server,
    utils::random_string,
};

pub struct TestApp {
    pub client: Client,
    pub config_file: String,
    pub port: u16,
    pub server_started: tokio::sync::broadcast::Receiver<()>,
}

impl TestApp {
    pub async fn is_ready(&mut self) {
        self.server_started
            .recv()
            .await
            .expect("could not start server");
    }

    pub async fn spawn() -> Self {
        let main_listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");

        let main_addr = (&main_listener).local_addr().unwrap();
        let main_port = main_addr.port();
        let mock1_listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
        let mock1_port = mock1_listener.local_addr().unwrap().port();
        let mock2_listener =
            std::net::TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
        let mock2_port = mock2_listener.local_addr().unwrap().port();

        let filepath = format!("{}.yaml", random_string());
        create_apps_file(&filepath, &main_port, &mock1_port, &mock2_port);

        tokio::spawn(mock_proxied_server(mock1_listener));
        tokio::spawn(mock_proxied_server(mock2_listener));

        let (tx, _) = broadcast::channel(16);
        let fp = filepath.clone();

        let (server_status, server_started) = broadcast::channel(16);

        let _ = tokio::spawn(async move {
            drop(main_listener);
            loop {
                info!("Configuration read !");
                let mut rx = tx.subscribe();
                let app = Server::build(&fp, tx.clone())
                    .await
                    .expect("could not build server from configuration");
                let server = axum::Server::bind(&main_addr)
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
            .resolve("vestibule.io", main_addr)
            .resolve("app1.vestibule.io", main_addr)
            .resolve("app2.vestibule.io", main_addr)
            .resolve("app2-altered.vestibule.io", main_addr)
            .resolve("files1.vestibule.io", main_addr)
            .resolve("files2.vestibule.io", main_addr)
            .cookie_store(true)
            .build()
            .unwrap();

        let mut test_app = TestApp {
            client: client,
            config_file: filepath,
            port: main_port,
            server_started: server_started,
        };

        test_app.is_ready().await;

        test_app
    }
}

pub fn create_apps_file(filepath: &str, main_port: &u16, mock1_port: &u16, mock2_port: &u16) {
    let apps = vec![
        App {
            id: 1,
            name: "App 1".to_owned(),
            icon: "app_1_icon".to_owned(),
            color: "#010101".to_owned(),
            is_proxy: true,
            host: "app1.vestibule.io".to_owned(),
            forward_to: format!("localhost:{mock1_port}"),
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
            host: "app2.vestibule.io".to_owned(),
            forward_to: format!("localhost:{mock2_port}"),
            secured: true,
            login: "admin".to_owned(),
            password: "ff54fds6f".to_owned(),
            openpath: "/javascript_simple.html".to_owned(),
            roles: vec!["ADMINS".to_owned()],
        },
    ];

    let davs = vec![
        Dav {
            id: 1,
            host: "files1.vestibule.io".to_owned(),
            directory: "./data/dir1".to_owned(),
            writable: true,
            name: "Files 1".to_owned(),
            icon: "file-invoice".to_owned(),
            color: "#2ce027".to_owned(),
            secured: true,
            roles: vec!["ADMINS".to_owned(), "USERS".to_owned()],
            passphrase: "".to_owned(),
        },
        Dav {
            id: 2,
            host: "files2.vestibule.io".to_owned(),
            directory: "./data/dir2".to_owned(),
            writable: true,
            name: "Files 2".to_owned(),
            icon: "file-invoice".to_owned(),
            color: "#2ce027".to_owned(),
            secured: true,
            roles: vec!["ADMINS".to_owned()],
            passphrase: "ABCD123".to_owned(),
        },
    ];

    let config = Config {
        debug_mode: true,
        http_port: *main_port,
        apps: apps,
        davs: davs,
    };

    // Act
    config.to_file(filepath).unwrap();
}
