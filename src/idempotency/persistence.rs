use actix_web::body::to_bytes;
use actix_web::HttpResponse;
use reqwest::StatusCode;
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::PgPool;
use uuid::Uuid;

use crate::idempotency::IdempotencyKey;

#[derive(Debug, sqlx::Type)]
#[sqlx(type_name = "header_pair")]
struct HeaderPairRecord {
    name: String,
    value: Vec<u8>,
}

pub async fn get_saved_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<Option<HttpResponse>, anyhow::Error> {
    let query = sqlx::query!(
        r#"
        SELECT 
        response_status_code,
        response_headers as "response_headers: Vec<HeaderPairRecord>",
        response_body
        FROM idempotency
        WHERE user_id = $1 AND 
        idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref()
    );

    let maybe_row = query.fetch_optional(pool).await?;
    match maybe_row {
        None => Ok(None),
        Some(response) => {
            let status_code = StatusCode::from_u16(response.response_status_code.try_into()?)?;
            let mut builder = HttpResponse::build(status_code);
            for header in response.response_headers {
                builder.append_header((header.name, header.value));
            }
            let http_response = builder.body(response.response_body);
            Ok(Some(http_response))
        }
    }
}

impl PgHasArrayType for HeaderPairRecord {
    fn array_type_info() -> PgTypeInfo {
        // Postgres implicitly creates an array type with a type name with a leading underscore
        // when we run like `CREATE TYPE header_pair`, which leads to a type name like `_header_pair`.
        PgTypeInfo::with_name("_header_pair")
    }
}

pub async fn save_response(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
    response: HttpResponse,
) -> Result<HttpResponse, anyhow::Error> {
    let (response_head, body) = response.into_parts();
    let body_b = to_bytes(body).await.map_err(|e| anyhow::anyhow!("{}", e))?;
    let headers = response_head
        .headers()
        .into_iter()
        .map(|(name, value)| HeaderPairRecord {
            name: name.as_str().to_string(),
            value: value.as_bytes().to_vec(),
        })
        .collect::<Vec<_>>();
    let status_code = response_head.status().as_u16() as i16;
    let query = sqlx::query_unchecked!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            response_status_code,
            response_headers,
            response_body,
            created_at
        )
        VALUES ($1, $2, $3, $4, $5, NOW())
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body_b.as_ref()
    );
    query.execute(pool).await?;

    let http_response = response_head.set_body(body_b).map_into_boxed_body();
    Ok(http_response)
}
