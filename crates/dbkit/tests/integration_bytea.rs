#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor};

#[model(table = "blob_rows")]
pub struct BlobRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub data: Vec<u8>,
    pub data_optional: Option<Vec<u8>>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE blob_rows (\
            id BIGSERIAL PRIMARY KEY,\
            data BYTEA NOT NULL,\
            data_optional BYTEA NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn bytea_roundtrip_filter_and_active_model_nulling() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let data = vec![0, 1, 2, 255];
    let optional = vec![3, 4, 5];

    let inserted = BlobRow::insert(BlobRowInsert {
        data: data.clone(),
        data_optional: Some(optional.clone()),
    })
    .returning_all()
    .one(&tx)
    .await?
    .expect("inserted blob row");

    assert_eq!(inserted.data, data);
    assert_eq!(inserted.data_optional, Some(optional));

    let found = BlobRow::query()
        .filter(BlobRow::data.eq(data.clone()))
        .one(&tx)
        .await?
        .expect("row by bytea equality");
    assert_eq!(found.id, inserted.id);

    let mut active = found.into_active();
    active.data_optional = None::<Vec<u8>>.into();
    let cleared = active.update(&tx).await?;
    assert!(cleared.data_optional.is_none());

    let only_null_optional = BlobRow::query().filter(BlobRow::data_optional.eq(None::<Vec<u8>>)).all(&tx).await?;
    assert_eq!(only_null_optional.len(), 1);
    assert_eq!(only_null_optional[0].id, inserted.id);

    Ok(())
}
