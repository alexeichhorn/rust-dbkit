#![allow(non_upper_case_globals)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use dbkit::prelude::*;
use dbkit::sqlx::postgres::{PgArguments, PgPoolOptions, PgRow};
use dbkit::sqlx::{FromRow, Row};
use dbkit::{func, model, Database, Error, Executor, Order};
use futures_util::{FutureExt, StreamExt, TryStreamExt};
use tokio::time::timeout;

#[model(table = "stream_groups")]
pub struct StreamGroup {
    #[key]
    pub id: i64,
    pub name: String,
}

#[model(table = "stream_items")]
pub struct StreamItem {
    #[key]
    pub id: i64,
    pub group_id: Option<i64>,
    pub name: String,
    pub note: Option<String>,
    #[belongs_to(key = group_id, references = id)]
    pub group: dbkit::BelongsTo<StreamGroup>,
}

#[model(table = "stream_probe")]
pub struct StreamProbe {
    #[key]
    pub id: i64,
}

#[model(table = "stream_failures")]
pub struct StreamFailure {
    #[key]
    pub id: i64,
    pub payload: String,
    pub value: i32,
}

#[derive(Debug, PartialEq, Eq, FromRow)]
struct ItemSummary {
    item_id: i64,
    note: Option<String>,
}

static DECODED_ROWS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug)]
struct DecodeProbe(i64);

impl FromRow<'_, PgRow> for DecodeProbe {
    fn from_row(row: &PgRow) -> Result<Self, sqlx::Error> {
        DECODED_ROWS.fetch_add(1, Ordering::SeqCst);
        let id = row.try_get("id")?;
        if id == 3 {
            return Err(sqlx::Error::ColumnNotFound("deliberate decode failure".into()));
        }
        Ok(Self(id))
    }
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn database() -> Result<Database, Error> {
    // One connection keeps temporary tables in scope and exposes leaked stream leases.
    let db = Database::connect_with_options(
        &db_url(),
        PgPoolOptions::new().max_connections(1).acquire_timeout(Duration::from_secs(2)),
    )
    .await?;
    for sql in [
        "SET statement_timeout = '5s'",
        "CREATE TEMP TABLE stream_groups (id BIGINT PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TEMP TABLE stream_items (id BIGINT PRIMARY KEY, group_id BIGINT, name TEXT NOT NULL, note TEXT)",
        "INSERT INTO stream_groups VALUES (1, 'north'), (2, 'south')",
        "INSERT INTO stream_items VALUES
            (1, 1, 'alpha', NULL), (2, 2, 'beta', ''),
            (3, 1, '猫 '' OR true --', 'memo'), (4, NULL, 'delta', NULL)",
    ] {
        db.execute(sql, PgArguments::default()).await?;
    }
    Ok(db)
}

async fn assert_pool_usable(db: &Database) -> Result<(), Error> {
    let (value,): (i32,) = timeout(Duration::from_secs(6), sqlx::query_as("SELECT 42").fetch_one(db.pool()))
        .await
        .expect("stream must release its connection")?;
    assert_eq!(value, 42);
    Ok(())
}

#[tokio::test]
async fn stream_preserves_filters_order_limit_offset_and_unloaded_relations() -> Result<(), Error> {
    let db = database().await?;
    let mut rows = StreamItem::query()
        .filter(StreamItem::id.gt(1_i64))
        .order_by(Order::desc(StreamItem::id))
        .offset(1)
        .limit(2)
        .stream(&db);

    let first = rows.try_next().await?.expect("first row");
    assert_eq!(first.id, 3);
    assert_eq!(first.note.as_deref(), Some("memo"));
    let _: dbkit::NotLoaded = first.group;
    let second = rows.try_next().await?.expect("second row");
    assert_eq!(second.id, 2);
    assert_eq!(second.note.as_deref(), Some(""));
    assert!(rows.try_next().await?.is_none());
    assert!(rows.try_next().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn stream_owns_temporary_query_and_bound_values() -> Result<(), Error> {
    let db = database().await?;
    let mut rows = {
        let name = String::from("猫 ' OR true --");
        StreamItem::query().filter(StreamItem::name.eq(name.as_str())).stream(&db)
    };
    assert_eq!(rows.try_next().await?.expect("bound name matches").id, 3);
    assert!(rows.try_next().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn query_runs_on_first_poll_rather_than_stream_creation() -> Result<(), Error> {
    let db = database().await?;
    let mut rows = StreamItem::query().filter(StreamItem::id.eq(1_i64)).stream(&db);
    db.execute("UPDATE stream_items SET name = 'changed' WHERE id = 1", PgArguments::default())
        .await?;
    assert_eq!(rows.try_next().await?.expect("updated row").name, "changed");
    assert!(rows.try_next().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn large_stream_visits_every_row_once_without_collecting() -> Result<(), Error> {
    let db = database().await?;
    db.execute("TRUNCATE stream_items", PgArguments::default()).await?;
    db.execute(
        "INSERT INTO stream_items SELECT n, NULL, repeat('猫', 100), NULL FROM generate_series(1, 10003) n",
        PgArguments::default(),
    )
    .await?;
    let mut rows = StreamItem::query().order_by(Order::asc(StreamItem::id)).stream(&db);
    let mut expected_id = 1;
    while let Some(row) = rows.try_next().await? {
        assert_eq!(row.id, expected_id);
        assert_eq!(row.name.chars().count(), 100);
        expected_id += 1;
    }
    assert_eq!(expected_id, 10004);
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn empty_results_zero_limit_and_offset_past_end_finish_cleanly() -> Result<(), Error> {
    let db = database().await?;
    for query in [
        StreamItem::query().filter(StreamItem::id.lt(0_i64)),
        StreamItem::query().limit(0),
        StreamItem::query().order_by(Order::asc(StreamItem::id)).offset(100),
    ] {
        let mut rows = query.stream(&db);
        assert!(rows.try_next().await?.is_none());
        assert!(rows.try_next().await?.is_none());
        assert_pool_usable(&db).await?;
    }
    Ok(())
}

#[tokio::test]
async fn stream_supports_named_projections_and_preserves_nulls() -> Result<(), Error> {
    let db = database().await?;
    let summaries: Vec<ItemSummary> = StreamItem::query()
        .select_only()
        .column_as(StreamItem::id, "item_id")
        .column(StreamItem::note)
        .order_by(Order::asc(StreamItem::id))
        .into_model::<ItemSummary>()
        .stream(db.pool())
        .try_collect()
        .await?;
    assert_eq!(
        summaries,
        vec![
            ItemSummary { item_id: 1, note: None },
            ItemSummary {
                item_id: 2,
                note: Some(String::new())
            },
            ItemSummary {
                item_id: 3,
                note: Some("memo".into())
            },
            ItemSummary { item_id: 4, note: None },
        ]
    );
    Ok(())
}

#[tokio::test]
async fn stream_supports_distinct_grouping_having_and_aggregate_expressions() -> Result<(), Error> {
    let db = database().await?;
    let groups: Vec<(Option<i64>,)> = StreamItem::query()
        .select_only()
        .column(StreamItem::group_id)
        .distinct()
        .order_by(Order::asc(StreamItem::group_id))
        .into_model::<(Option<i64>,)>()
        .stream(&db)
        .try_collect()
        .await?;
    assert_eq!(groups, vec![(Some(1),), (Some(2),), (None,)]);

    let totals: Vec<(Option<i64>, i64)> = StreamItem::query()
        .select_only()
        .column(StreamItem::group_id)
        .column(func::count(StreamItem::id))
        .group_by(StreamItem::group_id)
        .having(func::count(StreamItem::id).gt(1_i64))
        .into_model::<(Option<i64>, i64)>()
        .stream(&db)
        .try_collect()
        .await?;
    assert_eq!(totals, vec![(Some(1), 2)]);
    Ok(())
}

#[tokio::test]
async fn stream_supports_relation_path_joins_without_eager_loading() -> Result<(), Error> {
    let db = database().await?;
    let rows: Vec<(i64, Option<String>)> = StreamItem::query()
        .select_only()
        .column(StreamItem::id)
        .column(StreamItem::group.name)
        .order_by(Order::asc(StreamItem::id))
        .into_model::<(i64, Option<String>)>()
        .stream(&db)
        .try_collect()
        .await?;
    assert_eq!(
        rows,
        vec![
            (1, Some("north".into())),
            (2, Some("south".into())),
            (3, Some("north".into())),
            (4, None)
        ]
    );

    let ids: Vec<(i64,)> = StreamItem::query()
        .select_only()
        .column(StreamItem::id)
        .filter(StreamItem::group.name.eq("north"))
        .order_by(Order::asc(StreamItem::id))
        .into_model::<(i64,)>()
        .stream(&db)
        .try_collect()
        .await?;
    assert_eq!(ids, vec![(1,), (3,)]);
    Ok(())
}

#[tokio::test]
async fn unpolled_stream_does_not_execute_and_streaming_does_not_count_or_requery() -> Result<(), Error> {
    let db = database().await?;
    db.execute("CREATE TEMP SEQUENCE stream_calls", PgArguments::default()).await?;
    db.execute(
        "CREATE TEMP VIEW stream_probe AS SELECT nextval('stream_calls') AS id FROM generate_series(1, 4)",
        PgArguments::default(),
    )
    .await?;

    drop(StreamProbe::query().stream(&db));
    let (called,): (bool,) = sqlx::query_as("SELECT is_called FROM stream_calls").fetch_one(db.pool()).await?;
    assert!(!called, "creating and dropping an unpolled stream must not execute SQL");

    let rows: Vec<StreamProbe> = StreamProbe::query().stream(&db).try_collect().await?;
    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    let (last_value,): (i64,) = sqlx::query_as("SELECT last_value FROM stream_calls").fetch_one(db.pool()).await?;
    assert_eq!(last_value, 4, "no hidden count query or replay");
    Ok(())
}

#[tokio::test]
async fn stream_decodes_on_demand_and_stops_after_first_decode_error() -> Result<(), Error> {
    let db = database().await?;
    DECODED_ROWS.store(0, Ordering::SeqCst);
    let mut rows = StreamItem::query()
        .order_by(Order::asc(StreamItem::id))
        .into_model::<DecodeProbe>()
        .stream(&db);
    assert_eq!(DECODED_ROWS.load(Ordering::SeqCst), 0);
    assert_eq!(rows.try_next().await?.expect("first row").0, 1);
    assert_eq!(DECODED_ROWS.load(Ordering::SeqCst), 1);
    assert_eq!(rows.try_next().await?.expect("second row").0, 2);
    assert_eq!(DECODED_ROWS.load(Ordering::SeqCst), 2);
    assert!(matches!(
        rows.try_next().await,
        Err(Error::Sqlx(sqlx::Error::ColumnNotFound(ref name))) if name == "deliberate decode failure"
    ));
    assert_eq!(DECODED_ROWS.load(Ordering::SeqCst), 3);
    assert!(rows.try_next().await?.is_none());
    assert!(rows.try_next().await?.is_none());
    assert_eq!(DECODED_ROWS.load(Ordering::SeqCst), 3, "must not decode row four after an error");
    drop(rows);
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn stream_yields_rows_before_a_later_database_error() -> Result<(), Error> {
    let db = database().await?;
    // Wide rows force PostgreSQL to flush output before the third row divides by zero.
    // Collecting the whole result before yielding would lose the two successful rows.
    db.execute(
        "CREATE TEMP VIEW stream_failures AS
            SELECT n::BIGINT AS id, repeat('x', 65536) AS payload, 1 / (3 - n) AS value
            FROM generate_series(1, 3) AS n",
        PgArguments::default(),
    )
    .await?;
    let mut rows = StreamFailure::query().stream(&db);
    let first = rows.try_next().await?.expect("row before server failure");
    assert_eq!(first.id, 1);
    assert_eq!(first.payload.len(), 65536);
    assert_eq!(rows.try_next().await?.expect("second row before server failure").id, 2);
    assert!(matches!(
        rows.try_next().await,
        Err(Error::Sqlx(sqlx::Error::Database(ref error))) if error.code().as_deref() == Some("22012")
    ));
    assert!(rows.try_next().await?.is_none());
    drop(rows);
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn initial_query_error_is_an_item_then_the_stream_ends() -> Result<(), Error> {
    let db = database().await?;
    let mut rows = StreamProbe::query().stream(&db);
    assert!(matches!(
        rows.try_next().await,
        Err(Error::Sqlx(sqlx::Error::Database(ref error))) if error.code().as_deref() == Some("42P01")
    ));
    assert!(rows.try_next().await?.is_none());
    assert!(rows.try_next().await?.is_none());
    drop(rows);
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn closed_pool_error_is_reported_during_iteration() -> Result<(), Error> {
    let db = database().await?;
    let mut rows = StreamItem::query().stream(db.pool());
    timeout(Duration::from_secs(6), db.pool().close())
        .await
        .expect("unpolled stream must not hold a connection");
    assert!(matches!(rows.try_next().await, Err(Error::Sqlx(sqlx::Error::PoolClosed))));
    assert!(rows.try_next().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn exhausted_pool_error_is_reported_during_iteration() -> Result<(), Error> {
    let db = database().await?;
    let connection = db.pool().acquire().await?;
    let mut rows = StreamItem::query().stream(&db);
    assert!(matches!(
        timeout(Duration::from_secs(6), rows.try_next())
            .await
            .expect("acquire timeout must propagate"),
        Err(Error::Sqlx(sqlx::Error::PoolTimedOut))
    ));
    assert!(rows.try_next().await?.is_none());
    drop(rows);
    drop(connection);
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn dropping_stream_while_waiting_for_connection_does_not_leak_pool_capacity() -> Result<(), Error> {
    let db = database().await?;
    let connection = db.pool().acquire().await?;
    let mut rows = StreamItem::query().stream(&db);
    assert!(
        rows.try_next().now_or_never().is_none(),
        "must wait asynchronously for the connection"
    );
    drop(rows);
    drop(connection);
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn dropping_partially_consumed_stream_releases_its_connection() -> Result<(), Error> {
    let db = database().await?;
    let mut rows = StreamItem::query().order_by(Order::asc(StreamItem::id)).stream(&db);
    assert_eq!(rows.try_next().await?.expect("first row").id, 1);
    assert!(db.pool().try_acquire().is_none(), "active stream must retain its connection");
    drop(rows);
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn exhaustion_releases_connection_even_while_stream_variable_is_alive() -> Result<(), Error> {
    let db = database().await?;
    let mut rows = StreamItem::query().stream(&db);
    let mut count = 0;
    while rows.try_next().await?.is_some() {
        count += 1;
    }
    assert_eq!(count, 4);
    assert_pool_usable(&db).await?;
    assert!(rows.try_next().await?.is_none());
    Ok(())
}

#[tokio::test]
async fn standard_stream_combinators_preserve_errors_and_release_connections() -> Result<(), Error> {
    let db = database().await?;
    let ids: Vec<i64> = StreamItem::query()
        .order_by(Order::asc(StreamItem::id))
        .stream(&db)
        .take(2)
        .map_ok(|row| row.id)
        .try_collect()
        .await?;
    assert_eq!(ids, vec![1, 2]);
    assert_pool_usable(&db).await?;

    let result = StreamItem::query()
        .stream(&db)
        .try_for_each(|_| async { Err::<(), _>(Error::Decode("consumer stopped".into())) })
        .await;
    assert!(matches!(result, Err(Error::Decode(ref message)) if message == "consumer stopped"));
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn stream_reads_uncommitted_transaction_rows_and_can_be_dropped_before_rollback() -> Result<(), Error> {
    let db = database().await?;
    let tx = db.begin().await?;
    tx.execute(
        "INSERT INTO stream_items VALUES (5, 1, 'uncommitted', NULL)",
        PgArguments::default(),
    )
    .await?;
    let mut rows = StreamItem::query().order_by(Order::desc(StreamItem::id)).stream(&tx);
    assert_eq!(rows.try_next().await?.expect("transaction's own insert").id, 5);
    drop(rows);
    assert_eq!(
        timeout(Duration::from_secs(6), StreamItem::query().count(&tx))
            .await
            .expect("transaction mutex must be released")?,
        5
    );
    tx.rollback().await?;
    assert_eq!(StreamItem::query().count(&db).await?, 4);
    Ok(())
}

#[tokio::test]
async fn exhausted_transaction_stream_releases_mutex_before_commit() -> Result<(), Error> {
    let db = database().await?;
    let tx = db.begin().await?;
    tx.execute("INSERT INTO stream_items VALUES (5, 1, 'committed', NULL)", PgArguments::default())
        .await?;
    let mut rows = StreamItem::query().filter(StreamItem::id.eq(5_i64)).for_update().stream(&tx);
    assert_eq!(rows.try_next().await?.expect("inserted row").id, 5);
    assert!(rows.try_next().await?.is_none());
    assert_eq!(
        timeout(Duration::from_secs(6), StreamItem::query().count(&tx))
            .await
            .expect("exhaustion must release transaction mutex")?,
        5
    );
    drop(rows);
    tx.commit().await?;
    assert_eq!(StreamItem::query().count(&db).await?, 5);
    Ok(())
}

#[tokio::test]
async fn transaction_query_error_releases_mutex_for_rollback() -> Result<(), Error> {
    let db = database().await?;
    let tx = db.begin().await?;
    let mut rows = StreamProbe::query().stream(&tx);
    assert!(matches!(
        rows.try_next().await,
        Err(Error::Sqlx(sqlx::Error::Database(ref error))) if error.code().as_deref() == Some("42P01")
    ));
    assert!(rows.try_next().await?.is_none());
    drop(rows);
    timeout(Duration::from_secs(6), tx.rollback())
        .await
        .expect("failed stream must release transaction mutex")?;
    assert_pool_usable(&db).await?;
    Ok(())
}

#[tokio::test]
async fn transaction_remains_usable_after_client_side_decode_error() -> Result<(), Error> {
    let db = database().await?;
    let tx = db.begin().await?;
    let mut rows = StreamItem::query()
        .select_only()
        .column(StreamItem::id)
        .into_model::<(String,)>()
        .stream(&tx);
    assert!(matches!(rows.try_next().await, Err(Error::Sqlx(sqlx::Error::ColumnDecode { .. }))));
    assert!(rows.try_next().await?.is_none());
    drop(rows);
    assert_eq!(
        timeout(Duration::from_secs(6), StreamItem::query().count(&tx))
            .await
            .expect("decode error must release transaction mutex")?,
        4
    );
    tx.rollback().await?;
    Ok(())
}
