use linkify::Link;
use once_cell::sync::Lazy;
use sqlx::{ConnectOptions, Executor, PgPool};
use wiremock::MockServer;

use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::Application;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

pub static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber_name = "test";
    let default_filter_level = "debug";
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(subscriber_name, "debug", std::io::sink);
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub port: u16,
    pub address: String,
    pub db_pool: PgPool,
    pub email_server: MockServer,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/newsletters", &self.address))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();
        let get_link = |s: &str| {
            let links: Vec<Link> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(1, links.len());
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link
                .set_port(Some(self.port))
                .expect("Failed to set port");
            confirmation_link
        };
        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = config
        .without_db()
        .connect()
        .await
        .expect("Failed to connect to Postgres");
    let random_db_name = config.database_name.clone();
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, random_db_name).as_str())
        .await
        .expect("Failed to create database.");

    // Migrate database
    let new_pg_opt = config.with_db();
    let connection_pool = PgPool::connect_with(new_pg_opt)
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}

pub async fn spawn_app() -> TestApp {
    // The first time we call Lazy::force(&TRACING) the subscriber is initialized and
    // all subsequent calls will instead skip execution.
    Lazy::force(&TRACING);
    let email_server = MockServer::start().await;
    let configuration = {
        let mut c = get_configuration().expect("Failed to read and set configuration");
        c.application.port = 0_u16;
        // Use random database name for each test cases.
        c.database.database_name = uuid::Uuid::new_v4().to_string();
        c.email_client.base_url = email_server.uri();
        c
    };
    let app = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();

    let addr = format!("http://127.0.0.1:{}", port); // Note: Cause reqwest::Error if you forget "http://" prefix
    let db_pool = configure_database(&configuration.database).await;
    tokio::spawn(app.run_until_stopped());
    TestApp {
        port,
        address: addr,
        db_pool,
        email_server,
    }
}
