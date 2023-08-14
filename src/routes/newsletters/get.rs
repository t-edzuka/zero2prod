use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn publish_newsletter_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut message = String::new();
    for m in flash_messages.iter() {
        writeln!(message, "<p><i>{}</i></p>", m.content()).unwrap();
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
  {message}
   <form action="/admin/newsletters" method="post">
        <label>Title
            <input type="text" name="title" placeholder="Enter issue title">
        </label>
        <label>Plain text content:
        <br>
            <textarea name="text_content" cols="20" rows="50"></textarea>
        </label>
        <br>

        <label>Html content:
        <br>
            <textarea name="html_content" cols="20" rows="50"></textarea>
        </label>
        <br>
        <button type="submit">Publish</button>

   </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>

        "#
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body)
}
