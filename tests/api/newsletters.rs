use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

#[tokio::test]
async fn news_letters_are_not_unconfirmed_subscribers() {
    let app = spawn_app().await;
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
        "content": {
            "text": "Newsletter body",
            "html": "<p>Newsletter body</p>"
        }
    });
    // A blog author post a newsletter to notify subscribers by email.
    let response_body = app.post_newsletters(news_request_body).await;
    assert_eq!(200, response_body.status().as_u16());
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
    create_confirmed_subscriber(&app).await; // Create a confirmed subscriber simulating a user clicking the confirmation link in the email.
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let news_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body",
            "html": "<p>Newsletter body</p>"
        }
    });

    let response_body = app.post_newsletters(news_request_body).await;

    assert_eq!(200, response_body.status().as_u16());
}

#[tokio::test]
async fn newsletters_returns_400_when_data_is_missing() {
    let app = spawn_app().await;
    let invalid_test_case_with_missing_title = (
        serde_json::json!({
            "body": {
                "text": "Newsletter body",
                "html": "<p>Newsletter body</p>"
            }
        }),
        "missing title",
    );

    let invalid_test_case_with_missing_body = (
        serde_json::json!({
            "title": "Newsletter title",
        }),
        "missing body",
    );

    let invalid_bodies = vec![
        invalid_test_case_with_missing_title,
        invalid_test_case_with_missing_body,
    ];

    for (invalid_body, error_message) in invalid_bodies.into_iter() {
        let response = app.post_newsletters(invalid_body).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;
    let news_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body",
            "html": "<p>Newsletter body</p>"
        }
    });

    let response_in_no_authentication_headers = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .json(&news_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    let www_authenticate = &response_in_no_authentication_headers
        .headers()
        .get("WWW-Authenticate");
    assert_eq!(401, response_in_no_authentication_headers.status().as_u16()); // 401 Unauthorized
    assert_eq!(r#"Basic realm="publish""#, www_authenticate.unwrap())
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    let app = spawn_app().await;
    let news_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body",
            "html": "<p>Newsletter body</p>"
        }
    });

    // Generate random username and password to simulate a non-existing user.
    let username = Uuid::new_v4().to_string();
    let password = Uuid::new_v4().to_string();

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(&username, Some(&password))
        .json(&news_request_body)
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers().get("WWW-Authenticate").unwrap()
    )
}

#[tokio::test]
async fn invalid_password_with_existing_user_is_rejected() {
    let app = spawn_app().await;
    let username = &app.test_user.username;
    // Generate random password to simulate an invalid password.
    let password = Uuid::new_v4().to_string();
    let news_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter body",
            "html": "<p>Newsletter body</p>"
        }
    });

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletters", &app.address))
        .basic_auth(username, Some(&password))
        .json(&news_request_body)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers().get("WWW-Authenticate").unwrap()
    )
}
