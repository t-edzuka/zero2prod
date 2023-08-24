# *Zero To Production in Rust* Book

- [オリジナルレポジトリ](https://github.com/LukeMathWalker/zero-to-production)に従って実装した.
- `renovate`を利用して最新の依存関係を自動で更新するようにした.
- エクササイズも[damccull](https://github.com/damccull/zero2prod)のレポジトリを参考に実装した.

CIで`sqlx`のコンパイルを通すために、コミット前にローカルにクエリに関するjsonファイルを生成する必要がある.

```bash
export DATABASE_URL="postgres://postgres:password@127.0.0.1:5432/newsletter"
cargo sqlx prepare --database-url $DATABASE_URL -- --all-targets --all-features
```

`sqlx=0.71` にupdateする際に伴う必要な変更は以下の通り.

オリジナルコード (`sqlx="0.6"`)における``some_query.execute(&mut transaction).await?;``のような
以下のコード (from ``issue_delivery_worker.rs``)のコンパイルが`sqlx=0.71`では失敗する.

```rust
// In sqlx v0.6, this code compiles.
// but in sqlx v0.71, this code does not compile.
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

修正する際には[このRedditの投稿](https://www.reddit.com/r/rust/comments/14pw35f/sqlx_07_released_offline_mode_usability/jqmczb1/?utm_source=share&utm_medium=web3x&utm_name=web3xcss&utm_term=1&utm_content=share_button)
が参考になった.

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

    transaction.execute(query).await?; //Not query.execute(&mut transaction).await?;
    // Or the below is also OK.
    // query.execute(transaction.deref_mut()).await?;
    transaction.commit().await?;
    Ok(())
}
```
