use crate::authentication::{validate_credentials, Credentials};
use crate::routes::admin::dashboard::get_username;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use sqlx::PgPool;

use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};

#[derive(Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_check: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let is_entered_password_the_same =
        form.new_password.expose_secret() == form.new_password_check.expose_secret();
    let current_password = form.current_password.clone();

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
                password: current_password,
            };
            let res_uuid = validate_credentials(credentials, &pool).await.map_err(e500);
            match res_uuid {
                Ok(_) => Ok(HttpResponse::Ok().finish()), // TODO: Change response with password changed success message page.
                Err(_) => {
                    FlashMessage::error("The current password is incorrect.").send();
                    Ok(see_other("/admin/password"))
                }
            }
        }
    }
}
