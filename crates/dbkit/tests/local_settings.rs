use dbkit::sqlx::postgres::PgArguments;
use dbkit::sqlx::Row;
use dbkit::{Database, Executor};

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn current_test_setting<E: Executor + Send + Sync>(ex: &E) -> Result<Option<String>, dbkit::Error> {
    let rows = ex
        .fetch_rows(
            "SELECT current_setting('dbkit.test_setting', true) AS value",
            PgArguments::default(),
        )
        .await?;
    let value = rows
        .first()
        .expect("current_setting returns one row")
        .try_get::<Option<String>, _>("value")?;
    Ok(value)
}

#[tokio::test]
async fn set_local_sets_transaction_scoped_postgres_setting() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;

    tx.set_local("dbkit.test_setting", 1000).await?;
    assert_eq!(current_test_setting(&tx).await?, Some("1000".to_string()));

    tx.commit().await?;
    assert_ne!(current_test_setting(&db).await?, Some("1000".to_string()));

    Ok(())
}
