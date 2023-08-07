use actix_web::http::header::ContentType;
use actix_web::web;
use actix_web::HttpResponse;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct QueryParams {
    error: Option<String>,
}

pub async fn login_form(query: web::Query<QueryParams>) -> HttpResponse {
    let error_html = match query.0.error {
        Some(error_message) => format!("<p><i>{error_message}</i></p>"),
        None => "".into(),
    };

    let html_body = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8">
 <meta http-equiv="content-type" content="text/html">    
<title>Login</title>
</head>
<body>
  {error_html}
   <form action="/login" method="post">
        <label>Username
            <input type="text" name="username" placeholder="Enter Username">
        </label>
        <label>Password
            <input type="password" name="password" placeholder="Enter Password">
        </label>
        <button type="submit">Login</button>
   </form>
</body>
</html>

        "#
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body)
}
