use crate::authentication::UserId;
use crate::authentication::{validate_credentials, Credentials};
use crate::authentication::{AuthError, Password};
use crate::routes::admin::dashboard::get_username;

use crate::utils::{e500, see_other};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
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
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
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

    let user_id = user_id.into_inner();

    let username = get_username(*user_id, &pool).await.map_err(e500)?;
    let credentials = Credentials {
        username,
        password: form.current_password.inner_ref().clone(),
    };
    if let Err(e) = validate_credentials(credentials, &pool).await {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("The current password is incorrect.").send();
                Ok(see_other("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }
    crate::authentication::change_password_in_db(*user_id, form.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::error("Your password has been changed.").send();
    Ok(see_other("/admin/password"))
}
