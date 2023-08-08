use crate::session_state::TypedSession;
use crate::utils;
use actix_web::http::header::ContentType;
use actix_web::{web, HttpResponse};
use anyhow::Context;
use reqwest::header::LOCATION;
use sqlx::PgPool;
use uuid::Uuid;

#[tracing::instrument(name = "Get admin dashboard", skip(session, pool))]
pub async fn admin_dashboard(
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::error::Error> {
    let username = if let Some(user_id) = session.get_user_id().map_err(utils::e500)? {
        get_username(user_id, &pool).await.map_err(utils::e500)?
    } else {
        return Ok(HttpResponse::SeeOther()
            .insert_header((LOCATION, "/login"))
            .finish());
    };

    let body = format!(
        r#"<html lang="en">
<head>
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<title>Admin dashboard</title>
</head>
<body>
    <p>Welcome {username}</p>
    <p>Available actions:</p>
    <ol>
        <li><a href="/admin/password">Change password</a></li>
    </ol>
</body>
</html>

"#
    );
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(body))
}

#[tracing::instrument(name = "Fetch a username from database by user_id" skip(pool))]
pub async fn get_username(user_id: Uuid, pool: &PgPool) -> Result<String, anyhow::Error> {
    let q = sqlx::query!(
        r#"
    SELECT username FROM users WHERE user_id=$1
    "#,
        user_id
    );
    let row = q
        .fetch_one(pool)
        .await
        .context("Failed to perform query to retrieve a username.")?;
    Ok(row.username)
}
