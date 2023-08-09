use crate::authentication::{validate_credentials, Credentials};
use crate::routes::admin::dashboard::get_username;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use argon2::password_hash::SaltString;
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::session_state::TypedSession;
use crate::telemetry::spawn_blocking_with_tracing;
use crate::utils::{e500, see_other};

#[derive(Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

#[derive(Clone)]
pub struct Password(Secret<String>);

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

        Ok(Password(Secret::new(s)))
    }

    pub fn expose_secret(&self) -> String {
        self.0.expose_secret().clone()
    }
}

struct ValidPasswords {
    current_password: Password,
    new_password: Password,
    new_password_check: Password,
}

impl ValidPasswords {
    pub fn parse(form: FormData) -> Result<ValidPasswords, anyhow::Error> {
        let current_password = Password::parse(form.current_password.expose_secret())?;
        let new_password = Password::parse(form.new_password.expose_secret())?;
        let new_password_check = Password::parse(form.new_password_check.expose_secret())?;

        Ok(ValidPasswords {
            current_password,
            new_password,
            new_password_check,
        })
    }
}

pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    // Password length check.
    let form = match ValidPasswords::parse(form.into_inner()) {
        Ok(form) => form,
        Err(_) => {
            FlashMessage::error("The password must be between 12 and 128 characters long.").send();
            return Ok(see_other("/admin/password"));
        }
    };
    let is_entered_password_the_same =
        form.new_password.expose_secret() == form.new_password_check.expose_secret();

    if !is_entered_password_the_same {
        FlashMessage::error(
            "You entered two different new passwords - the field values must match.",
        )
        .send();
        return Ok(see_other("/admin/password"));
    }
    let option_user_id = session.get_user_id().map_err(e500)?;
    match option_user_id {
        None => Ok(see_other("/login")),
        Some(user_id) => {
            let username = get_username(user_id, &pool).await.map_err(e500)?;
            let credentials = Credentials {
                username,
                password: form.current_password.0.clone(), // MAYBE: Change the type signature to `Password` instead of Secret<String>?
            };
            let res_uuid = validate_credentials(credentials, &pool).await.map_err(e500);
            match res_uuid {
                Ok(user_id) => {
                    change_password_in_db(user_id, form.new_password, &pool)
                        .await
                        .map_err(e500)?;
                    FlashMessage::info("Your password has been changed.").send();
                    Ok(see_other("/admin/password"))
                } // TODO: Change response with password changed success message page.
                Err(_) => {
                    FlashMessage::error("The current password is incorrect.").send();
                    Ok(see_other("/admin/password"))
                }
            }
        }
    }
}

#[tracing::instrument(name = "Change password", skip(password, pool))]
async fn change_password_in_db(
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

fn compute_password_hash(password: Password) -> Result<Secret<String>, anyhow::Error> {
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

    Ok(Secret::new(password_hash))
}
