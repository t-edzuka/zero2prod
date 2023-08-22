//  1. async fn try_execute_task -> Result<(), anyhow::Error>

use crate::configuration::Settings;
use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::startup::get_connection_pool;
use sqlx::{Executor, PgPool, Postgres, Transaction};
use std::ops::DerefMut;
use tracing::field::display;
use tracing::Span;
use uuid::Uuid;

#[tracing::instrument(skip_all, fields(newsletter_issue_id = tracing::field::Empty, email = tracing::field::Empty), err)]
pub async fn try_execute_task(
    pool: &PgPool,
    email_client: &EmailClient,
) -> Result<ExecutionOutcome, anyhow::Error> {
    let maybe_task = dequeue_task(pool).await?;
    if maybe_task.is_none() {
        return Ok(ExecutionOutcome::EmptyQueue);
    }
    let (transaction, newsletter_issue_id, email) = maybe_task.unwrap();
    Span::current()
        .record("newsletter_issue_id", &display(newsletter_issue_id))
        .record("email", &display(&email));

    match SubscriberEmail::parse(email.clone()) {
        Ok(email) => {
            let issue: NewsletterIssue = get_issue(pool, newsletter_issue_id).await?;
            let send_result = email_client
                .send_email(
                    &email,
                    &issue.title,
                    &issue.html_content,
                    &issue.text_content,
                )
                .await;
            match send_result {
                Ok(_) => {
                    delete_task(transaction, newsletter_issue_id, email.as_ref()).await?;
                }
                Err(e) => {
                    tracing::error!(
                        error.cause_chain = ?e,
                        error.message = %e,
                        "Failed to deliver issue to a confirmed subscriber\
                        Skipping.",
                    );
                }
            }
        }
        Err(email_parse_error) => {
            tracing::error!(
                error.cause_chain = ?email_parse_error,
                error.message = %email_parse_error,
                "Failed to parse a stored subscriber email address. Skipping this email.",
            );
        }
    }

    Ok(ExecutionOutcome::TaskCompleted)
}

//  2. type = PgTransaction = Transaction<'static, Postgres>
type PgTransaction = Transaction<'static, Postgres>;

//  3. async fn dequeue_task -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error>
#[tracing::instrument(skip_all)]
async fn dequeue_task(
    pool: &PgPool,
) -> Result<Option<(PgTransaction, Uuid, String)>, anyhow::Error> {
    // Use transaction
    let mut transaction = pool.begin().await?;
    let q = sqlx::query!(
        r#"
        SELECT newsletter_issue_id, subscriber_email
        FROM issue_delivery_queue
        LIMIT 1
        FOR UPDATE SKIP LOCKED
        "#,
    );

    let result = q.fetch_optional(transaction.deref_mut()).await?;
    match result {
        Some(rec) => Ok(Some((
            transaction,
            rec.newsletter_issue_id,
            rec.subscriber_email,
        ))),
        None => Ok(None),
    }
}

//  4. async fn delete_task(mut transaction: PgTransaction, issue_id: Uuid, email: &str) -> Result<(), anyhow::Error>
#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    let q = sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email,
    );

    transaction.execute(q).await?;
    transaction.commit().await?;
    Ok(())
}

//  5. struct NewsletterIssue, title text_content, html_content
struct NewsletterIssue {
    title: String,
    text_content: String,
    html_content: String,
}

async fn get_issue(pool: &PgPool, issue_id: Uuid) -> Result<NewsletterIssue, anyhow::Error> {
    let q = sqlx::query_as!(
        NewsletterIssue,
        r#"
        SELECT title, text_content, html_content
        FROM newsletter_issues
        WHERE newsletter_issue_id = $1
        "#,
        issue_id,
    );

    let result = q.fetch_one(pool).await?;
    Ok(result)
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}

async fn worker_loop(pool: PgPool, email_client: EmailClient) -> Result<(), anyhow::Error> {
    loop {
        let outcome_result = try_execute_task(&pool, &email_client).await;
        let outcome = match outcome_result {
            Ok(outcome) => outcome,
            Err(_) => {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        match outcome {
            ExecutionOutcome::TaskCompleted => {
                continue;
            }
            ExecutionOutcome::EmptyQueue => {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                continue;
            }
        }
    }
}

// Launching Background Workers
pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);

    let sender_email = configuration
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let timeout = configuration.email_client.timeout();
    let email_client = EmailClient::new(
        configuration.email_client.base_url,
        sender_email,
        configuration.email_client.authorization_token,
        timeout,
    );
    worker_loop(connection_pool, email_client).await
}
