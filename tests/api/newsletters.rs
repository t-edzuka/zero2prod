use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};

#[tokio::test]
async fn news_letters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    app.login().await;
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
    let news_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body",
        "html_content": "<p>Newsletter body</p>"
    });
    // A blog author post a newsletter to notify subscribers by email.
    let response_body = app.post_newsletters(&news_request_body).await;
    assert_is_redirect_to(&response_body, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter has been published."));
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    // Create a subscriber, but not confirmed.
    let _api_response = app
        .post_subscriptions(body.into())
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
    app.login().await;

    create_confirmed_subscriber(&app).await; // Create a confirmed subscriber simulating a user clicking the confirmation link in the email.
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let news_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body",
        "html_content": "<p>Newsletter body</p>"
    });

    let response_body = app.post_newsletters(&news_request_body).await;
    assert_is_redirect_to(&response_body, "/admin/newsletters");

    let html_page = app.get_publish_newsletter_html().await;
    assert!(html_page.contains("The newsletter has been published."));
}

#[tokio::test]
async fn you_must_be_logged_in_to_see_newsletter_form() {
    let app = spawn_app().await;
    let response = app.get_publish_newsletter().await;
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn you_must_be_logged_in_to_publish_newsletter() {
    let app = spawn_app().await;
    let response = app
        .post_newsletters(&serde_json::json!({
            "title": "Newsletter title",
            "text_content": "Newsletter body",
            "html_content": "<p>Newsletter body</p>"
        }))
        .await;
    assert_is_redirect_to(&response, "/login")
}
