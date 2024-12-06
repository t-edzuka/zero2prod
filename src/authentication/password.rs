use crate::telemetry::spawn_blocking_with_tracing;
use anyhow::Context;

use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

use argon2::password_hash::SaltString;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

// This user input maps this struct, which is commonly called DTO: Data Transfer Object.
pub struct Credentials {
    pub username: String,
    pub password: SecretString,
}

#[tracing::instrument(name = "Validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<Uuid, AuthError> {
    // These two lines are for forcing calculating hash in blocking task.
    let mut user_id = None;
    let mut expected_password_hash = SecretString::new(Box::from(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTWllSAxT0zRno",
    ));

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
    expected_password_hash: SecretString,
    password_candidate: SecretString,
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
        .map_err(AuthError::InvalidCredentials)?;

    Ok(())
}

#[tracing::instrument(name = "Get stored credentials", skip(credentials, pool))]
pub async fn get_stored_credentials(
    credentials: &Credentials,
    pool: &PgPool,
) -> Result<Option<(Uuid, SecretString)>, anyhow::Error> {
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
        .map(|row| (row.user_id, SecretString::new(Box::from(row.password_hash))));
    Ok(row)
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
pub async fn change_password_in_db(
    user_id: Uuid,
    password: Password,
    pool: &PgPool,
) -> Result<(), anyhow::Error> {
    // Compute password_hash
    let password_hash = spawn_blocking_with_tracing(|| compute_password_hash(password))
        .await?
        .context("Failed to hash password")?;

    // Update users table, column: password_hash
    sqlx::query!(
        "UPDATE users SET password_hash=$1 WHERE user_id=$2",
        password_hash.expose_secret(),
        user_id
    )
    .execute(pool)
    .await
    .context("Failed to change user's password in the database.")?;
    Ok(())
}

fn compute_password_hash(password: Password) -> Result<SecretString, anyhow::Error> {
    // 1. Generate random salt
    let salt = SaltString::generate(&mut rand::thread_rng());
    // 2. Argon2 algorithm.
    let password_hash = Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15000, 2, 1, None).unwrap(),
    )
    .hash_password(password.expose_secret().as_bytes(), &salt)?
    .to_string();

    Ok(SecretString::new(Box::from(password_hash)))
}

#[derive(Clone)]
pub struct Password(SecretString);

impl Password {
    /// # OWASP’s a minimum set of requirements for password
    /// when it comes to password strength -
    /// passwords should be longer than 12 characters
    /// but shorter than 128 characters.
    pub fn parse(s: impl Into<String>) -> Result<Password, anyhow::Error> {
        use unicode_segmentation::UnicodeSegmentation;
        let s = s.into();

        let is_too_short = s.graphemes(true).count() < 12;
        if is_too_short {
            return Err(anyhow::anyhow!(
                "The password length must be at least 12 characters."
            ));
        }

        let is_too_long = s.graphemes(true).count() > 128;
        if is_too_long {
            return Err(anyhow::anyhow!(
                "The password length must be less than 128 characters."
            ));
        }

        Ok(Password(SecretString::new(Box::from(s))))
    }

    pub fn inner_ref(&self) -> &SecretString {
        &self.0
    }

    pub fn expose_secret(&self) -> &str {
        self.0.expose_secret()
    }
}
