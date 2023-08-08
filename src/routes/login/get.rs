use actix_web::cookie::Cookie;
use actix_web::http::header::ContentType;
use actix_web::{HttpRequest, HttpResponse};

///  # Used as login page or redirect page, when the login failed (controlled by cookie `_flash` key).  
/// When the user first vist login page(GET `/login`), just return login page
/// The user will be redirected to this page via POST `/login` with authentication failed, return login page with the error message injected.
/// by `Set-Cookie: _flash={{an error message}}`.  
/// After the error message is injected to html page, **the cookie value is removed** in the response.
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

    let mut response = HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body);
    response // Remove cookie immediately after the error message injected in the html page.
        .add_removal_cookie(&Cookie::new("_flash", ""))
        .unwrap();
    response
}
