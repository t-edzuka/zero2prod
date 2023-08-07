use actix_web::http::header::ContentType;
use actix_web::{HttpRequest, HttpResponse};

// pub struct QueryParams {
//     error: String,
// }

pub async fn login_form(request: HttpRequest) -> HttpResponse {
    let error_html = match request.cookie("_flash") {
        Some(cookie) => {
            format!("<p><i>{}</i></p>", cookie.value())
        }
        None => "".to_string(),
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
