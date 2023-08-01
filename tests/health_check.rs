use std::fmt::Debug;
use std::net::TcpListener;

use once_cell::sync::Lazy;
use sqlx::{ConnectOptions, Executor, PgPool};

use zero2prod::configuration::{get_configuration, DatabaseSettings, EmailClientSettings};
use zero2prod::domain::SubscriberEmail;
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
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

type LocalHttp = String;

struct TestApp {
    address: LocalHttp,
    db_pool: PgPool,
}

fn create_local_address(port: u16) -> LocalHttp {
    format!("http://127.0.0.1:{}", port)
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

pub async fn configure_email_client(config: &EmailClientSettings) -> EmailClient {
    let email_string = config.sender_email.clone();
    let sender = SubscriberEmail::parse(email_string.clone())
        .unwrap_or_else(|_| panic!("Failed to parse sender email string, got {}", &email_string));
    let base_url = config.base_url.clone();
    let authorization_token = config.authorization_token.clone();
    let timeout = config.timeout();
    EmailClient::new(base_url, sender, authorization_token, timeout)
}

async fn spawn_app() -> TestApp {
    // The first time we call Lazy::force(&TRACING) the subscriber is initialized and
    // all subsequent calls will instead skip execution.
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Error: address or port may be wrong.");
    let port = listener.local_addr().unwrap().port();
    let app_addr = create_local_address(port);

    let conf = {
        let mut c = get_configuration().expect("Failed to read configurations.");
        // Use random os port.
        c.application.port = 0_u16;
        // Use random database name for each test cases.
        c.database.database_name = uuid::Uuid::new_v4().to_string();
        c
    };

    let db_pool = configure_database(&conf.database).await;
    let email_client = configure_email_client(&conf.email_client).await;

    let server = run(listener, db_pool.clone(), email_client).expect("Failed to bind address");

    tokio::spawn(server);

    TestApp {
        address: app_addr,
        db_pool,
    }
}

#[tokio::test]
async fn test_health_check_work() {
    let TestApp {
        address,
        db_pool: _,
    } = spawn_app().await;
    let client = reqwest::Client::new();
    let endpoint = format!("{}/health_check", address);
    let response = client
        .get(endpoint)
        .send()
        .await
        .expect("Failed to execute request");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let TestApp { address, db_pool } = spawn_app().await;

    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    // Act
    let response = client
        .post(&format!("{}/subscriptions", address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16());

    // After act fetch saved data from db
    let saved = sqlx::query!("select email, name from subscriptions")
        .fetch_one(&db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    // Assert
    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let TestApp {
        address,
        db_pool: _,
    } = spawn_app().await;

    let client = reqwest::Client::new();
    //

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];
    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        )
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=ursula&email=", "empty email"),
        (
            "name=le%20guin&email=definitely-not-an-email",
            "invalid email",
        ),
    ];
    for (body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(
            400_u16,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when payload was {}.",
            error_message
        )
    }
}
