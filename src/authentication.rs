use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AuthError> {
    let mut user_id = None;
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno"
            .to_string(),
    );

    // Fetch user information from the database
    if let Some((stored_user_id, stored_hash_password)) =
        get_stored_credentials(&credentials, pool).await?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_hash_password;
    }

    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("Failed to spawn blocking task.")??; // Nested Error => Result<Result<(), PublishError>, Error>??;

    user_id
        .ok_or_else(|| anyhow::anyhow!("Unknown username."))
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(
    name = "Verify password hash",
    skip(expected_password_hash, password_candidate)
)]
fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    // Calculate the password hash by using the password_hash stored in the database, following PHC string format.
    let expected_password_hash = PasswordHash::new(expected_password_hash.expose_secret())
        .context("Failed to parse hash in PHC string format.")?;
    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("Failed to verify password hash.")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "Get stored credentials", skip(credentials, pool))]
async fn get_stored_credentials(
    credentials: &Credentials,
    pool: &PgPool,
) -> Result<Option<(Uuid, Secret<String>)>, anyhow::Error> {
    let q = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1
        "#,
        credentials.username,
    );

    let row = q
        .fetch_optional(pool)
        .await
        .context("Failed to perform query to validate auth credentials.")?
        .map(|row| (row.user_id, Secret::new(row.password_hash)));
    Ok(row)
}
