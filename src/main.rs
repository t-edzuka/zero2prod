use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;

use zero2prod::configuration::{get_configuration, EmailClientSettings};
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Telemetry setup
    let subscriber = get_subscriber("rs_z2p", "info", std::io::stdout);
    init_subscriber(subscriber);

    // Database and app connection setup
    let conf = get_configuration().expect("Failed to read configurations.");
    let pg_opt = conf.database.with_db();
    let pg_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(pg_opt);

    // Application setup
    let app_addr = conf.application.addr();
    let email_client_settings = conf.email_client.clone();

    // Email client setup
    let EmailClientSettings {
        base_url,
        sender_email: _,
        authorization_token,
        timeout_milliseconds: _,
    } = email_client_settings.clone();

    let email_client = EmailClient::new(
        base_url,
        email_client_settings
            .sender()
            .expect("Failed to parse email sender address"),
        authorization_token,
        email_client_settings.timeout(),
    );

    tracing::info!("Starting server at {}", app_addr);
    let listener = TcpListener::bind(app_addr).expect("Failed to bind port");

    run(listener, pg_pool, email_client)?.await
}
