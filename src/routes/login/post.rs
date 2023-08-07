use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;
use actix_web::body::BoxBody;
use actix_web::http::header::LOCATION;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use secrecy::Secret;
use serde::Deserialize;
use sqlx::PgPool;
use std::fmt::{Debug, Formatter};

#[derive(Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error("Login process failed at form submission")]
    UnexpectedError(#[from] anyhow::Error),
}

impl Debug for LoginError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn status_code(&self) -> StatusCode {
        StatusCode::SEE_OTHER
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let encoded_error = urlencoding::Encoded::new(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((LOCATION, format!("/login?error={}", encoded_error)))
            .finish()
    }
}

#[tracing::instrument(skip(form, pool), fields(username = tracing::field::Empty, user_id = tracing::field::Empty))]
pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, LoginError> {
    let credentials = Credentials {
        username: form.0.username,
        password: form.0.password,
    };
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(e) => LoginError::AuthError(e),
            AuthError::UnexpectedError(e) => LoginError::UnexpectedError(e),
        })?;

    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));
    Ok(HttpResponse::SeeOther()
        .insert_header((LOCATION, "/"))
        .finish())
}
