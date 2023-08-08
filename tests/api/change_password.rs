use crate::helpers::{assert_is_redirect_to, spawn_app};
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    // Arrange
    let app = spawn_app().await;
    // Act
    let response = app.get_change_password().await;
    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_the_password() {
    // Arrange
    let app = spawn_app().await;
    // password change form
    let new_password = Uuid::new_v4().to_string();
    let password_change_form = serde_json::json!(
        {
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password
        }
    );
    // Act
    let response = app.post_change_password(&password_change_form).await;
    // Assert
    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;
    // Act -Part 1 - Login correctly.
    let valid_login_form = serde_json::json!(
        {
            "username": app.test_user.username,
            "password": app.test_user.password.clone()
        }
    );
    app.post_login(&valid_login_form).await;

    // Act - Part 2 -
    let new_password = Uuid::new_v4().to_string();
    let new_password_diff = Uuid::new_v4().to_string();

    let password_change_form = serde_json::json!(
        {
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_check": &new_password_diff
        }
    );
    // Act
    let response = app.post_change_password(&password_change_form).await;
    // Assert
    assert_is_redirect_to(&response, "/admin/password");

    // Act 3 Follow the redirect, which contain the expected error message
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - the field values must match.</i></p>"
    ));
}

#[tokio::test]
async fn current_password_must_be_valid() {
    // Arrange
    let app = spawn_app().await;

    // Act -Part 1 - Login correctly.
    let valid_login_form = serde_json::json!(
        {
            "username": app.test_user.username,
            "password": app.test_user.password
        }
    );
    app.post_login(&valid_login_form).await;

    // Act - Part 2 - Try to change password with the current password wrong.
    let wrong_password = Uuid::new_v4().to_string();
    let new_password = Uuid::new_v4().to_string();

    let password_change_form = serde_json::json!(
        {
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_check": &new_password
        }
    );
    // Act
    let response = app.post_change_password(&password_change_form).await;
    // Assert
    assert_is_redirect_to(&response, "/admin/password");

    // Act 3 Follow the redirect, which contain the expected error message
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"));
}
