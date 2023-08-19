use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;

// Return a Bad Request (400) with the user-representation of the validation error as body.
// The error root cause is preserved for logging purposes.
pub fn e400<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorBadRequest(e)
}

// Return an opaque Internal Server Error (500),
// while preserving the error root's cause for logging.
pub fn e500<T>(e: T) -> actix_web::Error
where
    T: std::fmt::Debug + std::fmt::Display + 'static,
{
    actix_web::error::ErrorInternalServerError(e)
}

pub fn see_other(location: &str) -> HttpResponse {
    // Borrow checker is complaining with E505 for (LOCATION, location), the reason is unsure,
    // but the code can be compiled.
    HttpResponse::SeeOther()
        .insert_header((LOCATION, location))
        .finish()
}
