# *Zero To Production in Rust* Book

I've successfully walked through the entire content of the book!✨

- I basically followed the [original repository](https://github.com/LukeMathWalker/zero-to-production).
- I've set up `renovate` to automatically update to the latest dependencies.
- For exercises, I referred to the [damccull](https://github.com/damccull/zero2prod) repository for implementation.

To pass the `sqlx` compilation in CI, it's necessary to generate a json file related to the query locally before
committing.

```bash
export DATABASE_URL="postgres://postgres:password@127.0.0.1:5432/newsletter"
cargo sqlx prepare --database-url $DATABASE_URL -- --all-targets --all-features
```

The necessary changes when updating to sqlx=0.71 are as follows:

In the original code (`sqlx="0.6"`), something like `some_query.execute(&mut transaction).await?`.

The following code (from `issue_delivery_worker.rs`) does not compile in `sqlx=0.71`.

```rust
// In sqlx v0.6, this code compiles.
// However, in sqlx v0.71, this code does not.
type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE 
            newsletter_issue_id = $1 AND
            subscriber_email = $2 
        "#,
        issue_id,
        email
    )
    .execute(&mut transaction) // This line does not compile.
    .await?;
    transaction.commit().await?;
    Ok(())
}
```

For the
correction, [this Reddit post](https://www.reddit.com/r/rust/comments/14pw35f/sqlx_07_released_offline_mode_usability/jqmczb1/?utm_source=share&utm_medium=web3x&utm_name=web3xcss&utm_term=1&utm_content=share_button)
was helpful.

```rust
type PgTransaction = Transaction<'static, Postgres>;

#[tracing::instrument(skip_all)]
async fn delete_task(
    mut transaction: PgTransaction,
    issue_id: Uuid,
    email: &str,
) -> Result<(), anyhow::Error> {
    let query = sqlx::query!(
        r#"
        DELETE FROM issue_delivery_queue
        WHERE newsletter_issue_id = $1 AND subscriber_email = $2
        "#,
        issue_id,
        email,
    );

    transaction.execute(query).await?; // Not query.execute(&mut transaction).await?;
    // Alternatively, the lines below is also acceptable.
    // query.execute(transaction.deref_mut()).await?;
    // query.execute(&mut *transaction).await?;

    transaction.commit().await?;
    Ok(())
}
```