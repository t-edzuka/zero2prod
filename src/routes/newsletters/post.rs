use crate::authentication::UserId;
use actix_web::web::{Form, ReqData};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;

use crate::utils::{e500, see_other};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
}

#[tracing::instrument(
name = "Publishing newsletter",
skip(body, pool, email_client),
fields(user_id = % * _user_id),
)]
pub async fn publish_newsletter(
    body: Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    _user_id: ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
    // 1. Authenticate the request
    // ... depends on ReqData<UserId> middleware.
    // 2. Get all confirmed subscribers
    let confirmed_subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;

    // 3. Send newsletter to all confirmed subscribers
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(
                        &subscriber.email,
                        &body.title,
                        &body.html_content,
                        &body.text_content,
                    )
                    .await
                    .with_context(|| {
                        format!("Failed to send newsletter issue to {}", subscriber.email)
                    })
                    .map_err(e500)?;
            }
            Err(error) => {
                tracing::warn!(
                    error.cause_chain = ?error,
                    "Failed to notify subscriber, skipping",
                );
            }
        }
    }
    FlashMessage::info("The newsletter has been published.").send();
    Ok(see_other("/admin/newsletters"))
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
