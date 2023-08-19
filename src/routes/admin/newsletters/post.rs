use actix_web::web::{Form, ReqData};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::PgPool;

use crate::authentication::UserId;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::idempotency::{get_saved_response, save_response, IdempotencyKey};
use crate::utils::{e400, e500, see_other};

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[tracing::instrument(
name = "Publishing newsletter",
skip(form, pool, email_client),
fields(user_id = % * user_id),
)]
pub async fn publish_newsletter(
    form: Form<FormData>,
    user_id: ReqData<UserId>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = user_id.into_inner();
    // 1. Authenticate the request
    // ... depends on ReqData<UserId> middleware.

    // Destructure the form data
    let FormData {
        title,
        text_content,
        html_content,
        idempotency_key,
    } = form.into_inner();
    // 2. Parse the idempotency_key from the form data
    let idempotency_key = // TODO: Process this idempotency_key using the database.
        IdempotencyKey::try_from(idempotency_key).map_err(e400)?;

    // 2.5 Check if the newsletter has already been published
    let maybe_saved_response = get_saved_response(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?;
    if let Some(saved_response) = maybe_saved_response {
        FlashMessage::info("The newsletter has been published.").send();
        return Ok(saved_response);
    }

    // 3. Get all confirmed subscribers
    let confirmed_subscribers = get_confirmed_subscribers(&pool).await.map_err(e500)?;

    // 4. Send newsletter to all confirmed subscribers
    for subscriber in confirmed_subscribers {
        match subscriber {
            Ok(subscriber) => {
                email_client
                    .send_email(&subscriber.email, &title, &html_content, &text_content)
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
    let response = see_other("/admin/newsletters");
    // 5. Save the response
    let response = save_response(&pool, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    Ok(response)
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
