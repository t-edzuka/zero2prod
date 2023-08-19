use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use linkify::Link;
use once_cell::sync::Lazy;
use sqlx::{ConnectOptions, Executor, PgPool};
use uuid::Uuid;
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
    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

// A set of API client implementations for testing.
impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_publish_newsletter<Body: serde::Serialize>(
        &self,
        body: &Body,
    ) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/admin/newsletters", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_publish_newsletter(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/newsletters", &self.address))
            .send()
            .await
            .expect("Failed to fetch GET /newsletters/publish response")
    }

    #[allow(unused)]
    pub async fn get_publish_newsletter_html(&self) -> String {
        self.get_publish_newsletter().await.text().await.expect(
            "Failed to fetch GET /newsletters/publish response body, expected body is a HTML text.",
        )
    }

    pub async fn post_login<Body: serde::Serialize>(&self, body: &Body) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to send request to `/login` endpoint.")
    }

    pub async fn get_login_html(&self) -> String {
        let response = self
            .api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to GET `/login` endpoint.");
        response.text().await.expect("Failed get the login page.")
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", &self.address))
            .send()
            .await
            .expect("Failed to GET `/admin/dashboard` endpoint.")
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard()
            .await
            .text()
            .await
            .expect("Failed get the dashboard page.")
    }

    pub async fn get_change_password(&self) -> reqwest::Response {
        self.api_client
            .get(format!("{}/admin/password", &self.address))
            .send()
            .await
            .expect("Failed to fetch GET /admin/password response")
    }

    pub async fn post_change_password<Body: serde::Serialize>(
        &self,
        body: &Body,
    ) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/password", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to fetch POST /admin/password response")
    }

    pub async fn get_change_password_html(&self) -> String {
        self.get_change_password()
            .await
            .text()
            .await
            .expect("Failed to fetch the change password html page")
    }

    pub async fn post_logout(&self) -> reqwest::Response {
        self.api_client
            .post(format!("{}/admin/logout", &self.address))
            .send()
            .await
            .expect("Failed to fetch POST /logout response")
    }
}

pub fn assert_is_redirect_to(response: &reqwest::Response, redirect_endpoint: &str) {
    assert_eq!(303, response.status().as_u16());
    assert_eq!(
        response.headers().get("Location").unwrap(),
        redirect_endpoint
    )
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

pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
            // password: "everything-has-to-start-somewhere".to_string(), // For just seeding.
        }
    }
    pub async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();
        let q = sqlx::query!(
            "INSERT INTO USERS (user_id, username, password_hash) values ($1, $2, $3)",
            self.user_id,
            self.username,
            password_hash
        );

        q.execute(pool).await.expect("Failed to insert test user");
    }

    pub async fn login(&self, app: &TestApp) -> reqwest::Response {
        let body = serde_json::json!({
            "username": self.username,
            "password": self.password,
        });
        app.post_login(&body).await
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
        c.database.database_name = Uuid::new_v4().to_string();
        c.email_client.base_url = email_server.uri();
        c
    };
    let app = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");
    let port = app.port();

    let addr = format!("http://127.0.0.1:{}", port); // Note: Cause reqwest::Error if you forget "http://" prefix
    let db_pool = configure_database(&configuration.database).await;
    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none()) // Turn off the default redirect behaviour, which is specific for reqwest library.
        .cookie_store(true)
        .build()
        .expect("Failed to build a API client for testing.");
    tokio::spawn(app.run_until_stopped());
    let test_app = TestApp {
        port,
        address: addr,
        db_pool,
        email_server,
        test_user: TestUser::generate(),
        api_client,
    };

    // Create a test user
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}
