use actix_web::web::Form;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::{NewSubscriber, SubscriberEmail, SubscriberName};

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
skip(form, pool),
fields(
subscriber_email = % form.email,
subscriber_name = % form.name
)
)]
pub async fn subscribe(form: Form<FormData>, pool: web::Data<PgPool>) -> HttpResponse {
    let new_subscriber = match NewSubscriber::try_from(form.0) {
        Ok(subscriber) => subscriber,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    match insert_subscriber(&pool, &new_subscriber).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "Saving new subscriber details in the database",
    skip(pool, new_subscriber)
)]
pub async fn insert_subscriber(
    pool: &PgPool,
    new_subscriber: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    let _insert_query = sqlx::query!(
        r#"
        insert into subscriptions (id, email, name, subscribed_at)
        values ($1, $2, $3, $4)
         "#,
        Uuid::new_v4(),
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

    Ok(())
}
