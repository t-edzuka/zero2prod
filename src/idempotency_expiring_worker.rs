use crate::configuration::Settings;
use crate::startup::get_connection_pool;
use sqlx::{Executor, PgPool};
use std::time::Duration;

// Expiring idempotency key
// Reference implementation.
// https://github.com/damccull/zero2prod/blob/main/zero2prod/src/idempotency_remover_worker.rs

// Every 24 hours, we will remove all the expired keys from the database.
// We define "Expired" as any key that has not been used in the last 48 hours.

#[tracing::instrument(skip_all)]
pub async fn delete_expired_idempotency_key(
    pool: &PgPool,
    expired_after_hours: u8,
) -> Result<u64, anyhow::Error> {
    let mut transaction = pool.begin().await?;
    let query_string = format!(
        r#"
        DELETE FROM idempotency
        WHERE created_at < NOW() - INTERVAL '{} hours'
        "#,
        expired_after_hours
    );
    let res = transaction.execute(sqlx::query(&query_string)).await?;

    transaction.commit().await?;
    Ok(res.rows_affected())
}

async fn worker_loop(pool: PgPool, expired_after_hours: u8) -> Result<(), anyhow::Error> {
    loop {
        let delete_execution_result =
            delete_expired_idempotency_key(&pool, expired_after_hours).await;
        match delete_execution_result {
            Ok(_) => tracing::info!("Successfully deleted expired idempotency keys."),
            Err(e) => tracing::error!(
                error.cause_chain = ?e,
                error.message = %e,
                "Failed to delete expired idempotency keys."
            ),
        }
        // Sleep for 24 hours.
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
    }
}

/// This is called in entry point of the application.
pub async fn run_until_worker_stopped(
    configuration: Settings,
    expired_after_hours: u8,
) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    worker_loop(connection_pool, expired_after_hours).await
}
