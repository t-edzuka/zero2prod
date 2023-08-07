use std::fmt::Formatter;

use crate::authentication;
use crate::authentication::{AuthError, Credentials};
use actix_web::body::BoxBody;
use actix_web::http::header::{HeaderMap, HeaderValue};
use actix_web::http::{header, StatusCode};
use actix_web::web::Json;
use actix_web::{web, HttpResponse, ResponseError};
use anyhow::Context;
use base64::Engine;
use secrecy::Secret;
use sqlx::PgPool;
use thiserror;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    text: String,
    html: String,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            PublishError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                response
            }
            PublishError::UnexpectedError(_) => HttpResponse::InternalServerError().finish(),
        }
    }

    // `status_code` is invoked by the default `error_response`
    // implementation. We are providing a bespoke `error_response` implementation
    // therefore there is no need to maintain a `status_code` implementation anymore.
}

#[tracing::instrument(
name = "Publishing newsletter",
skip(body, pool, email_client),
fields(username = tracing::field::Empty, user_id = tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: actix_web::HttpRequest,
) -> Result<HttpResponse, PublishError> {
    // 1. Authenticate the request
    let headers = request.headers();
    let credentials = basic_authentication(headers).map_err(PublishError::AuthError)?;
    tracing::Span::current().record("username", &tracing::field::display(&credentials.username));
    let user_id = authentication::validate_credentials(credentials, &pool)
        .await
        .map_err(|e| match e {
            AuthError::InvalidCredentials(e) => PublishError::AuthError(e),
            AuthError::UnexpectedError(e) => PublishError::UnexpectedError(e),
        })?;
    tracing::Span::current().record("user_id", &tracing::field::display(&user_id));

    // 2. Get all confirmed subscribers
    let confirmed_subscribers = get_confirmed_subscribers(&pool).await?;

    // 3. Send newsletter to all confirmed subscribers
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.content.html,
                        &body.content.text,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Failed to notify subscriber, skipping",
                );
            }
        }
    }
    Ok(HttpResponse::Ok().finish())
}

pub struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[tracing::instrument(name = "Get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email
        FROM subscriptions
        WHERE status = 'confirmed'
        "#,
    )
    .fetch_all(pool)
    .await?;

    let confirmed_subscribers = rows
        .into_iter()
        .map(|row| match SubscriberEmail::parse(row.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect::<Vec<_>>();
    Ok(confirmed_subscribers)
}

fn basic_authentication(headers: &HeaderMap) -> Result<Credentials, anyhow::Error> {
    let authorization = headers
        .get("Authorization")
        .context("Missing \"Authorization\" header.")?
        .to_str()
        .context("Failed to parse authorization header")?;
    let authorization = strip_prefix_case_insensitive(authorization, "Basic ")
        .context("Invalid authorization header")?;
    let decoded_bytes = base64::engine::general_purpose::STANDARD.decode(authorization)?;
    let decoded_authorization = String::from_utf8(decoded_bytes)?;
    let mut credentials = decoded_authorization.splitn(2, ':');
    let username = credentials.next().ok_or_else(|| {
        anyhow::anyhow!("username must be provide in Basic authentication headers.")
    })?;
    let password = credentials.next().ok_or_else(|| {
        anyhow::anyhow!("password must be provided in basic authorization headers.")
    })?;
    Ok(Credentials {
        username: username.to_owned(),
        password: Secret::new(password.to_owned()),
    })
}

fn strip_prefix_case_insensitive<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let s = s.trim_start();
    if s.len() >= prefix.len() && s[..prefix.len()].eq_ignore_ascii_case(prefix) {
        Some(&s[prefix.len()..])
    } else {
        None
    }
}

#[test]
fn basic_authentication_valid_case() {
    use secrecy::ExposeSecret;
    let mut headers_cases = vec![];
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization".parse().unwrap(),
        "Basic QWxhZGRpbjpPcGVuU2VzYW1l".parse().unwrap(),
    );
    headers_cases.push(headers);
    let mut headers = HeaderMap::new();
    headers.insert(
        "Authorization".parse().unwrap(),
        " Basic QWxhZGRpbjpPcGVuU2VzYW1l".parse().unwrap(), // Leading space allowed in RFC 7230 section 3.2.4
    );
    headers_cases.push(headers);

    for headers in headers_cases {
        let credentials = basic_authentication(&headers).unwrap();
        assert_eq!(credentials.username, "Aladdin");
        assert_eq!(credentials.password.expose_secret(), "OpenSesame");
    }
}

#[test]
fn basic_prefix_remove_case_insensitively() {
    let s = "Basic QWxhZGRpbjpPcGVuU2VzYW1l";
    assert_eq!(
        strip_prefix_case_insensitive(s, "Basic "),
        Some("QWxhZGRpbjpPcGVuU2VzYW1l")
    );
    assert_eq!(
        strip_prefix_case_insensitive(s, "BASIC "),
        Some("QWxhZGRpbjpPcGVuU2VzYW1l")
    );
    assert_eq!(
        strip_prefix_case_insensitive(s, "basic "),
        Some("QWxhZGRpbjpPcGVuU2VzYW1l")
    );
    assert_eq!(
        strip_prefix_case_insensitive(s, "baSic "),
        Some("QWxhZGRpbjpPcGVuU2VzYW1l")
    );
    assert_eq!(strip_prefix_case_insensitive(s, "Bearer "), None);
}
