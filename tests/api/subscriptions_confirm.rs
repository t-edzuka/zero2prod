use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::spawn_app;

#[tokio::test]
async fn confirmations_without_token_are_rejected_with_a_400_bad_request() {
    // Arrange
    let test_app = spawn_app().await;

    // Assert
    let response = reqwest::get(&format!("{}/subscriptions/confirm", test_app.address))
        .await
        .expect("Failed to execute request.");
    assert_eq!(400, response.status().as_u16());
}

#[tokio::test]
async fn the_link_returned_by_subscribe_returns_a_200_if_called() {
    // Arrange
    let test_app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    // Act
    let _response = test_app.post_subscriptions(body.into()).await;
    // Receive request at mock server
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    // Parse body as JSON to get the link
    let confirmation_links = test_app.get_confirmation_links(email_request);
    let response = reqwest::get(confirmation_links.html).await.unwrap();
    assert_eq!(200, response.status().as_u16());
}
