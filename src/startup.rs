use actix_session::storage::RedisSessionStore;
use actix_session::SessionMiddleware;
use std::net::TcpListener;

use crate::authentication::reject_anonymous_user;
use actix_web::cookie::Key;
use actix_web::dev::Server;
use actix_web::middleware::from_fn;
use actix_web::{web, App, HttpServer};
use actix_web_flash_messages::storage::CookieMessageStore;
use actix_web_flash_messages::FlashMessagesFramework;
use secrecy::{ExposeSecret, SecretString};

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    admin_dashboard, change_password, change_password_form, confirm, health_check, log_out, login,
    publish_newsletter, publish_newsletter_form, subscribe,
};
use crate::routes::{home, login_form};

pub struct Application {
    server: Server,
    port: u16,
}

impl Application {
    pub async fn build(configuration: Settings) -> Result<Self, anyhow::Error> {
        let connection_pool = get_connection_pool(&configuration.database);

        let email_client = configuration.email_client.client();
        let listener = TcpListener::bind(configuration.application.addr())?;
        let port = listener
            .local_addr()
            .expect("Failed to get local address")
            .port();
        let server = run(
            listener,
            connection_pool,
            email_client,
            configuration.application.base_url,
            configuration.application.hmac_secret,
            configuration.redis_uri,
        )
        .await?;
        Ok(Self { server, port })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), anyhow::Error> {
        self.server.await?;
        Ok(())
    }
}

// We need to define a wrapper type in order to retrieve the base url from the configuration file.
pub struct ApplicationBaseUrl(pub String);

#[derive(Clone)]
pub struct HmacSecret(pub SecretString);

pub fn get_connection_pool(database_settings: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(database_settings.with_db())
}

pub async fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
    hmac_secret: SecretString,
    redis_uri: SecretString,
) -> Result<Server, anyhow::Error> {
    let connection = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));

    let secret_key = Key::from(hmac_secret.expose_secret().as_bytes());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let flash_message_framework = FlashMessagesFramework::builder(message_store).build();

    let redis_store = RedisSessionStore::new(redis_uri.expose_secret()).await?;

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .wrap(flash_message_framework.clone())
            .wrap(SessionMiddleware::new(
                redis_store.clone(),
                secret_key.clone(),
            ))
            .route("/", web::get().to(home))
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .service(
                web::scope("/admin")
                    .wrap(from_fn(reject_anonymous_user))
                    .route("/dashboard", web::get().to(admin_dashboard))
                    .route("/password", web::get().to(change_password_form))
                    .route("/password", web::post().to(change_password))
                    .route("/logout", web::post().to(log_out))
                    .route("/newsletters", web::get().to(publish_newsletter_form))
                    .route("/newsletters", web::post().to(publish_newsletter)),
            )
            .app_data(connection.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
            .app_data(web::Data::new(HmacSecret(hmac_secret.clone())))
    })
    .listen(listener)?
    .run();
    Ok(server)
}
