use crate::session_state::TypedSession;
use crate::utils::{e500, see_other};
use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::IncomingFlashMessages;
use std::fmt::Write;

pub async fn change_password_form(
    session: TypedSession,
    flash_message: IncomingFlashMessages,
) -> Result<HttpResponse, actix_web::Error> {
    let mut error_messages = String::new();
    for m in flash_message.iter() {
        writeln!(error_messages, "<p><i>{}</i></p>", m.content()).unwrap();
    }
    if session.get_user_id().map_err(e500)?.is_none() {
        return Ok(see_other("/login"));
    }
    let html_body = format!(
        r#"
<!DOCTYPE html>
<html lang="en"> 
<head><meta http-equiv="Content-Type" content="text/html;charset=UTF-8">
<title>Change password</title>
</head>
<body>
    {error_messages}
    <form action="/admin/password" method="post">
        <label >Current password
            <input type="password" name="current_password" placeholder="Enter current password">
        </label>
        <br>

        <label >New password
            <input type="password" 
                   name="new_password" 
                   placeholder="Enter new password">
        </label>
        <br>

         <label >Confirm new password
            <input type="password" 
                   name="new_password_check" 
                   placeholder="Type the new password again">
        </label>
        <button type="submit">Change password</button>
        <p><a href="/admin/dashboard">&lt;- Back</a></p>
    </form>
</body>
</html>

    
    "#
    );
    Ok(HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_body))
}
