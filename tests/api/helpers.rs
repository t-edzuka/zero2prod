use once_cell::sync::Lazy;
use sqlx::{ConnectOptions, Executor, PgPool};

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
    pub address: String,
    pub db_pool: PgPool,
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
    let configuration = {
        let mut c = get_configuration().expect("Failed to read and set configuration");
        c.application.port = 0_u16;
        // Use random database name for each test cases.
        c.database.database_name = uuid::Uuid::new_v4().to_string();
        c
    };
    let app = Application::build(configuration.clone())
        .await
        .expect("Failed to build application");

    let addr = format!("http://127.0.0.1:{}", app.port()); // Note: Cause reqwest::Error if you forget "http://" prefix
    tokio::spawn(app.run_until_stopped());
    let db_pool = configure_database(&configuration.database).await;
    TestApp {
        address: addr,
        db_pool,
    }
}
