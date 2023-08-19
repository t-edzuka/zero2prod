use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;
use uuid::Uuid;

pub async fn publish_newsletter_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut message = String::new();
    for m in flash_messages.iter() {
        writeln!(message, "<p><i>{}</i></p>", m.content()).unwrap();
    }

    let idempotency_key = Uuid::new_v4().to_string();
    let html_body = format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8">
 <meta http-equiv="content-type" content="text/html">    
<title>Publish a Newsletter</title>
</head>
<body>
  {message}
   <form action="/admin/newsletters" method="post">
        <label>Title:<br>
            <input type="text" name="title" placeholder="Enter issue title">
        </label>
        <br>
        <label>Plain text content:
        <br>
            <textarea name="text_content" cols="50" rows="20"></textarea>
        </label>
        <br>

        <label>Html content:
        <br>
            <textarea name="html_content" cols="50" rows="20"></textarea>
        </label>
        <br>
            <input type="hidden" name="idempotency_key" value="{idempotency_key}">
        <button type="submit">Publish</button>

   </form>
    <p><a href="/admin/dashboard">&lt;- Back</a></p>
</body>
</html>

        "#,
    );

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body)
}
