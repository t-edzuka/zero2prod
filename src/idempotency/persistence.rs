use actix_web::body::to_bytes;
use actix_web::HttpResponse;
use reqwest::StatusCode;
use sqlx::postgres::{PgHasArrayType, PgTypeInfo};
use sqlx::{Executor, PgPool, Postgres, Transaction};
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
    // `! as {column_name}` is required to avoid the error:
    // `!` mark forcefully assume to tell the compiler that the column will not be null, while the column type is nullable.
    // So if we mistakenly insert null value into the column, we will get an "runtime" error.
    let query = sqlx::query!(
        r#"
        SELECT 
        response_status_code as "response_status_code!",
        response_headers as "response_headers!: Vec<HeaderPairRecord>",
        response_body as "response_body!"
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
    mut transaction: Transaction<'static, Postgres>,
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
        UPDATE idempotency 
        SET
            response_status_code = $3,
            response_headers = $4,
            response_body = $5
        WHERE
            user_id = $1 AND 
            idempotency_key = $2
        "#,
        user_id,
        idempotency_key.as_ref(),
        status_code,
        headers,
        body_b.as_ref()
    );
    transaction.execute(query).await?;
    transaction.commit().await?;

    let http_response = response_head.set_body(body_b).map_into_boxed_body();
    Ok(http_response)
}

pub enum NextAction {
    StartProcessing(Transaction<'static, Postgres>),
    ReturnSavedResponse(HttpResponse),
}

pub async fn try_processing(
    pool: &PgPool,
    idempotency_key: &IdempotencyKey,
    user_id: Uuid,
) -> Result<NextAction, anyhow::Error> {
    // The isolation level in postgres is "read committed" by default.
    // In this isolation level, a SELECT query sees only data committed before the query began,
    // while NEVER see data that is still uncommitted, or committed during query execution by concurrent transactions.

    let mut transaction = pool.begin().await?;

    // This query is for the case that the "concurrent" requests are sent to this server.
    // The first request will insert a row into the table,
    // and the second request will try to insert a row into the table,
    // but it will do nothing because of the conflict.

    let query = sqlx::query!(
        r#"
        INSERT INTO idempotency (
            user_id,
            idempotency_key,
            created_at
        )
        VALUES ($1, $2, NOW())
        ON CONFLICT (user_id, idempotency_key) DO NOTHING
        "#,
        user_id,
        idempotency_key.as_ref(),
    );

    let n_inserted_rows = transaction.execute(query).await?.rows_affected();

    if n_inserted_rows > 0 {
        // This means that the request is the first request.
        // This returning transaction will be used in the next `UPDATE` query.
        // In postgres, the latter updater will *wait*
        // for the first updater to commit or rollback.
        Ok(NextAction::StartProcessing(transaction))
    } else {
        // This means that the request is the second or later request.
        let saved_response = get_saved_response(pool, idempotency_key, user_id)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("We expected a saved response, actually, we didn't find it.")
            })?;
        Ok(NextAction::ReturnSavedResponse(saved_response))
    }
}
