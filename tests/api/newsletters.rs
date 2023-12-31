use chrono::Utc;
use fake::faker::internet::en::SafeEmail;
use fake::faker::name::en::Name;
use fake::Fake;
use serde_json::Value;
use sqlx::PgPool;
use std::ops::Sub;
use std::time::Duration;
use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};
use zero2prod::idempotency_expiring_worker::delete_expired_idempotency_key;
use zero2prod::routes::SUCCESS_MESSAGE;

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};

fn sample_newsletter_form() -> Value {
    let idempotency_key = Uuid::new_v4().to_string();
    serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body",
        "html_content": "<p>Newsletter body</p>",
        "idempotency_key": idempotency_key,
    })
}

#[tokio::test]
async fn news_letters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    app.test_user.login(&app).await;
    create_unconfirmed_subscriber(&app).await;

    // Mock postmark server email service will not be called,
    // because the subscriber is not confirmed.
    // In this context, "call" implies that the email is not sent to the unconfirmed subscriber.
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    // Create a newsletter

    // A blog author post a newsletter to notify subscribers by email.
    let response_body = app.post_publish_newsletter(&sample_newsletter_form()).await;
    assert_is_redirect_to(&response_body, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains(SUCCESS_MESSAGE));
    app.dispatch_all_pending_emails().await;
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let name: String = Name().fake();
    let email: String = SafeEmail().fake();
    let body = serde_urlencoded::to_string(serde_json::json!({
        "name": name,
        "email": email,
    }))
    .expect("Failed to serialize the subscriber name and email data.");

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    // Create a subscriber, but not confirmed.
    let _api_response = app
        .post_subscriptions(body)
        .await
        .error_for_status()
        .unwrap();
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(email_request)
}

async fn create_confirmed_subscriber(app: &TestApp) {
    // Start from unconfirmed subscriber
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    // Arrange
    let app = spawn_app().await;
    app.test_user.login(&app).await;

    create_confirmed_subscriber(&app).await; // Create a confirmed subscriber simulating a user clicking the confirmation link in the email.
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response_body = app.post_publish_newsletter(&sample_newsletter_form()).await;
    assert_is_redirect_to(&response_body, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains(SUCCESS_MESSAGE));
    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn you_must_be_logged_in_to_see_newsletter_form() {
    let app = spawn_app().await;
    // Act
    let response = app.post_publish_newsletter(&sample_newsletter_form()).await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_a_newsletter() {
    let app = spawn_app().await;
    let response = app.post_publish_newsletter(&sample_newsletter_form()).await;
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    // Arrange:
    // 1. Start the app.
    let app = spawn_app().await;
    // 2.Create confirmed subscriber. & login as an admin.
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;
    // 3. Mount mockserver,
    // which expects to be called with a POST /email endpoint, only once with response 200.
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    // 4. Create a newsletter as form data with a idempotent key, which is a POST request to /admin/newsletters,
    // Act:
    // Post the newsletter.
    let newsletter_form = sample_newsletter_form();
    let response = app.post_publish_newsletter(&newsletter_form).await;

    // Assert:
    // 1. The response is a redirect to /admin/newsletters.
    assert_is_redirect_to(&response, "/admin/newsletters");
    // 2. The "published" message will be shown in the page.
    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains(SUCCESS_MESSAGE));

    // Act2:
    // Call again with the same idempotency key.
    let response = app.post_publish_newsletter(&newsletter_form).await;
    let html_page = app.get_publish_newsletter_html().await;
    // Assert2:
    // 1. The response is a redirect to /admin/newsletters.
    assert_is_redirect_to(&response, "/admin/newsletters");
    // 2. The same "published" message will be shown in the page.
    assert!(html_page.contains(SUCCESS_MESSAGE));
    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        // Setting a long delay to ensure that the second request
        // arrives before the first one completes
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        // .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Submit two newsletter forms concurrently
    let newsletter_request_body = sample_newsletter_form();
    let response1 = app.post_publish_newsletter(&newsletter_request_body);
    let response2 = app.post_publish_newsletter(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);

    assert_eq!(response1.status(), response2.status());
    assert_eq!(
        response1.text().await.unwrap(),
        response2.text().await.unwrap()
    );
    // Mock verifies on Drop that we have sent the newsletter email **once**
    app.dispatch_all_pending_emails().await;
}

#[tokio::test]
async fn transient_errors_get_retried() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    // First failure response from external external service.
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Second time, success.
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;

    // Act - Part 1 - Login
    app.test_user.login(&app).await;

    // Act - Part 2 - Send Newsletter
    let newsletter_request_body = sample_newsletter_form();

    app.post_publish_newsletter(&newsletter_request_body).await;

    app.dispatch_all_pending_emails().await;

    // Mock verifies on Drop that we have attempted to send the email twice.
    // and the second time should have been successful.
}

#[tokio::test]
async fn old_idempotency_key_is_cleaned_up() {
    // Arrange
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;
    let user_id = app.test_user.user_id;
    let pool = &app.db_pool;

    // When 2 idempotency keys are expired and 1 is valid.
    create_expired_idempotency_key(pool, user_id).await;
    create_expired_idempotency_key(pool, user_id).await;
    create_valid_idempotency_key(pool, user_id).await;
    // Act
    let deleted_count = delete_expired_idempotency_key(pool, 48)
        .await
        .expect("Failed to delete expired idempotency key.");
    assert_eq!(deleted_count, 2);

    let the_rest_count = sqlx::query!(
        r#"
        SELECT COUNT(*) AS count FROM idempotency
        "#,
    )
    .fetch_one(pool)
    .await
    .expect("Failed to count the rows in idempotency table.");
    assert_eq!(the_rest_count.count, Some(1));
}

async fn create_expired_idempotency_key(pool: &PgPool, user_id: Uuid) {
    let now = Utc::now();
    let before_49_hours = now.sub(chrono::Duration::hours(49));

    sqlx::query!(
        r#"
        INSERT INTO idempotency (user_id, idempotency_key, created_at)
        VALUES ($1, $2, $3)
        "#,
        user_id,
        Uuid::new_v4().to_string(),
        before_49_hours,
    )
    .execute(pool)
    .await
    .expect("Failed to insert expired idempotency key.");
}

async fn create_valid_idempotency_key(pool: &PgPool, user_id: Uuid) {
    let now = Utc::now();

    sqlx::query!(
        r#"
        INSERT INTO idempotency (user_id, idempotency_key, created_at)
        VALUES ($1, $2, $3)
        "#,
        user_id,
        Uuid::new_v4().to_string(),
        now,
    )
    .execute(pool)
    .await
    .expect("Failed to insert a new idempotency key.");
}
