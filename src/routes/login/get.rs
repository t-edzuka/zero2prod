use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

///  # Used as login page or redirect page, when the login failed (controlled by cookie `_flash` key).  
/// When the user first vist login page(GET `/login`), just return login page.    
/// The user with authentication failed will be redirected to this page via POST `/login`, return login page with the error message injected.  
/// Error message is sent by `FlashMessageFrameWork`.    
/// After the error message is injected to html page, **the cookie value is removed** in the new login page response.  
///
pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();
    for m in flash_messages.iter().filter(|m| m.level() == Level::Error) {
        let _ = writeln!(error_html, "<p><i>{}</i></p>", m.content());
    }
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
