use actix_web::http::header::ContentType;
use actix_web::web;
use actix_web::HttpResponse;
use hmac::{Hmac, Mac};
use secrecy::ExposeSecret;
use serde::Deserialize;

use crate::startup::HmacSecret;

#[derive(Deserialize)]
pub struct QueryParams {
    error: String,
    tag: String,
}

impl QueryParams {
    fn verify(self, secret: &HmacSecret) -> Result<String, anyhow::Error> {
        let tag = hex::decode(self.tag)?;
        let query_string = format!("error={}", urlencoding::Encoded::new(&self.error));
        let mut mac =
            Hmac::<sha2::Sha256>::new_from_slice(secret.0.expose_secret().as_bytes()).unwrap();
        mac.update(query_string.as_bytes());
        mac.verify_slice(&tag)?;
        Ok(self.error)
    }
}

pub async fn login_form(
    query: Option<web::Query<QueryParams>>,
    secret: web::Data<HmacSecret>,
) -> HttpResponse {
    let error_html = match query {
        Some(query) => match query.0.verify(&secret) {
            Ok(error) => {
                format!("<p><i>{}</i></p>", htmlescape::encode_minimal(&error))
            }
            Err(e) => {
                tracing::warn!(
                    error.message = %e,
                    error.cause_chain = %e,
                    "Failed to verify query parameters using the HMAC tag"
                );
                "".into()
            }
        },
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
