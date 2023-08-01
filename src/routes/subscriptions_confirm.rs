use actix_web::web::Query;
use actix_web::HttpResponse;

use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(name = "Confirm a pending subscriber", skip(_parameters))]
pub async fn confirm(_parameters: Query<Parameters>) -> HttpResponse {
    // If the Query extraction fails,
    // this function returns automatically a 400 Bad Request error.
    HttpResponse::Ok().finish()
}
