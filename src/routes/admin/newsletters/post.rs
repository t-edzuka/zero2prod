use crate::authentication::UserId;
use crate::email_client::EmailClient;
use crate::idempotency::{save_response, try_processing, IdempotencyKey, NextAction};
use crate::utils::{e400, e500, see_other};
use actix_web::web::{Form, ReqData};
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::Context;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct FormData {
    title: String,
    text_content: String,
    html_content: String,
    idempotency_key: String,
}

#[tracing::instrument(
name = "Publishing newsletter",
skip(form, pool),
fields(user_id = % * user_id),
)]
pub async fn publish_newsletter(
    form: Form<FormData>,
    user_id: ReqData<UserId>,
    pool: web::Data<PgPool>,
    _email_client: web::Data<EmailClient>,
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
    let idempotency_key = IdempotencyKey::try_from(idempotency_key).map_err(e400)?;

    let mut transaction = match try_processing(&pool, &idempotency_key, *user_id)
        .await
        .map_err(e500)?
    {
        NextAction::StartProcessing(t) => t,
        NextAction::ReturnSavedResponse(http_response) => {
            success_message().send();
            return Ok(http_response);
        }
    };

    let issue_id = insert_news_letter_issue(&mut transaction, &title, &text_content, &html_content)
        .await
        .context("Failed to store newsletter issue details")
        .map_err(e500)?;

    enqueue_delivery_tasks(&mut transaction, issue_id)
        .await
        .context("Failed to enqueue delivery tasks")
        .map_err(e500)?;

    let response = see_other("/admin/newsletters");
    /* 5. Save the response */
    let response = save_response(transaction, &idempotency_key, *user_id, response)
        .await
        .map_err(e500)?;
    success_message().send();
    Ok(response)
}

fn success_message() -> FlashMessage {
    FlashMessage::info("The newsletter has been published.")
}

#[tracing::instrument(skip_all)]
async fn insert_news_letter_issue(
    transaction: &mut Transaction<'_, Postgres>,
    title: &str,
    text_content: &str,
    html_content: &str,
) -> Result<Uuid, sqlx::Error> {
    let newsletter_issue_id = Uuid::new_v4();
    let q = sqlx::query!(
        r#"
        INSERT INTO newsletter_issues (newsletter_issue_id, title, text_content, html_content, published_at)
        VALUES ($1, $2, $3, $4, NOW())
        "#,
        newsletter_issue_id,
        title,
        text_content,
        html_content,
    );

    transaction.execute(q).await?;
    Ok(newsletter_issue_id)
}

#[tracing::instrument(skip_all)]
async fn enqueue_delivery_tasks(
    transaction: &mut Transaction<'_, Postgres>,
    newsletter_issue_id: Uuid,
) -> Result<(), sqlx::Error> {
    let q = sqlx::query!(
        r#"
        INSERT INTO issue_delivery_queue (newsletter_issue_id, subscriber_email)
        SELECT $1, email FROM subscriptions WHERE status='confirmed'
        "#,
        newsletter_issue_id,
    );

    transaction.execute(q).await?;
    Ok(())
}
