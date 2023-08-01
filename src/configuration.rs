use secrecy::{ExposeSecret, Secret};

use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::ConnectOptions;

use crate::domain::SubscriberEmail;

// `Settings` struct is basically coming from `configurations/base.toml`, `local.toml`, or `production.toml`
// In production environment, the values may be overridden by environment variables. See `get_configuration()`.
// Settings will be deserialized from toml files. the struct field name is equivalent to the toml key name.
// For which toml file to use, see `get_configuration()`.
#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
}

impl ApplicationSettings {
    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    pub host: String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .port(self.port)
            .username(&self.username)
            .password(self.password.expose_secret())
            .ssl_mode(ssl_mode)
    }

    pub fn with_db(&self) -> PgConnectOptions {
        let opts = self.without_db().database(&self.database_name);
        opts.log_statements(tracing_log::log::LevelFilter::Trace)
    }
}

#[derive(Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine current directory");
    let configuration_directory = base_path.join("configurations");

    // APP_ENVIRONMENT is set in Dockerfile, like APP_ENVIRONMENT="production" but by default set to "local"
    // This env variable is used to determine which toml file to use in configurations directory.
    let app_env_string = std::env::var("APP_ENVIRONMENT").unwrap_or("local".into());
    let environment =
        Environment::try_from(app_env_string).expect("Failed to parse APP_ENVIRONMENT");
    let environment_filename = format!("{}.toml", environment.as_str());

    let settings = config::Config::builder()
        .add_source(config::File::from(
            configuration_directory.join("base.toml"),
        ))
        .add_source(config::File::from(
            // Control configuration by APP_ENVIRONMENT value and its toml file name.
            // like "production.toml", "local.toml", etc.
            configuration_directory.join(environment_filename),
        ))
        .add_source(
            // This section is for deployment in Digital Ocean. See spec.yaml
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;
    settings.try_deserialize::<Settings>()
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            other => Err(format!(
                "{} is not valid env variable, \n Help: Use either `local` or `production`",
                other
            )),
        }
    }
}
