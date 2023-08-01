use crate::helpers::{spawn_app, TestApp};

#[tokio::test]
async fn test_health_check_work() {
    let TestApp {
        port: _,
        address,
        db_pool: _,
        email_server: _,
    } = spawn_app().await;
    let client = reqwest::Client::new();
    let endpoint = format!("{}/health_check", address);
    let response = client
        .get(endpoint)
        .send()
        .await
        .expect("Failed to execute request");
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
