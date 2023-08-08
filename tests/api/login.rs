use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;
    let body = serde_json::json!({
        "username": "random-user-name",
        "password": "random-password"
    });

    // Act - Part 1 - Try to login
    let response = app.post_login(&body).await;
    assert_is_redirect_to(&response, "/login");
    let flash_cookie = response
        .cookies()
        .find(|c| c.name() == "_flash")
        .expect("Cookie name: `_flash` is not set");
    assert_eq!(flash_cookie.value(), "Authentication failed");

    // Act - Part 2 - Follow the redirect
    let html_page = app.get_login_html().await;
    let html_auth_error_message = r#"<p><i>Authentication failed</i></p>"#;
    assert!(html_page.contains(html_auth_error_message));

    // Act - Part 3 - Reload the login page
    let html_page = app.get_login_html().await;
    assert!(!html_page.contains(html_auth_error_message));
}
