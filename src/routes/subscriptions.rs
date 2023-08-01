use actix_web::web::Form;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};
use crate::email_client::EmailClient;
use crate::startup::ApplicationBaseUrl;

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(form_data: FormData) -> Result<Self, Self::Error> {
        let name = SubscriberName::parse(form_data.name)?;
        let email = SubscriberEmail::parse(form_data.email)?;
        Ok(NewSubscriber { name, email })
    }
}

#[derive(Deserialize)]
pub struct FormData {
    pub email: String,
    pub name: String,
}

#[tracing::instrument(
name = "Adding a new subscriber",
skip(form, pool, email_client, base_url),
fields(
subscriber_email = % form.email,
subscriber_name = % form.name
)
)]
pub async fn subscribe(
    form: Form<FormData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    base_url: web::Data<ApplicationBaseUrl>,
) -> HttpResponse {
    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };

    let subscriber_id = match insert_subscriber(&pool, &new_subscriber).await {
        Ok(subscription_id) => subscription_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let subscription_token = generate_subscription_token();
    // Store token
    if store_token(&pool, subscriber_id, &subscription_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirmation_email(
        &email_client,
        new_subscriber,
        &base_url.0,
        subscription_token.as_str(),
    )
    .await
    .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }
    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let subscriber_id = Uuid::new_v4();
    let _insert_query = sqlx::query!(
        r#"
        insert into subscriptions (id, email, name, subscribed_at, status)
        values ($1, $2, $3, $4, 'pending_confirmation')
         "#,
        subscriber_id,
        new_subscriber.email.as_ref(),
        new_subscriber.name.as_ref(),
        Utc::now(),
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;

    Ok(subscriber_id)
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, subscription_token)
)]
async fn store_token(
    pool: &PgPool,
    subscriber_id: Uuid,
    subscription_token: &str,
) -> Result<(), sqlx::Error> {
    let insert_query = sqlx::query!(
        r#"
        insert into subscription_tokens (subscription_token, subscriber_id)
        values ($1, $2)
        "#,
        subscription_token,
        subscriber_id,
    );
    insert_query.execute(pool).await.map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Sending confirmation email to subscriber",
    skip(email_client, new_subscriber)
)]
pub async fn send_confirmation_email(
    email_client: &EmailClient,
    new_subscriber: NewSubscriber,
    base_url: &str,
    subscription_token: &str,
) -> Result<(), reqwest::Error> {
    // 仮実装 TODO: Replace with a real implementation
    let confirmation_link = format!(
        "{}/subscriptions/confirm?subscription_token={}",
        base_url, subscription_token
    );
    let temp_var = {
        let html = format!(
            r#"Welcomt to our newsletter!<br/>
            Click <a href \{}>here</a> to confirm your subscription."#,
            confirmation_link
        );
        let plain_text = format!(
            "Welcome to our newsletter!\nVisit {} to confirm your subscription.",
            confirmation_link
        );
        (html, plain_text)
    };

    let (html, plain_text) = temp_var;
    email_client
        .send_email(
            new_subscriber.email,
            "Email title",
            html.as_ref(),
            plain_text.as_ref(),
        )
        .await
}

pub fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
<<<<<<< HEAD
        .map(char::from)
=======
        .map(|random_byte: u8| char::from(random_byte))
>>>>>>> cb90b82 (:white_check_mark: chapter-7.6.2 Integrate confirmation subscriber token store)
        .take(25)
        .collect()
}
