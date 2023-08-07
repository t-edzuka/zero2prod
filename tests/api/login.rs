use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;
    let body = serde_json::json!({
        "username": "random-user-name",
        "password": "random-password"
    });
    let response = app.post_login(&body).await;
    assert_is_redirect_to(&response, "/login");

    let flash_cookie = response
        .cookies()
        .find(|c| c.name() == "_flash")
        .expect("Cookie name: `_flash` is not set");

    assert_eq!(flash_cookie.value(), "Authentication failed");
}
