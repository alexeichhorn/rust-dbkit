#![allow(non_upper_case_globals)]

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::func::{RegexReplaceFlags, RegexSplitFlags};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, IntoExpr};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
    pub email: String,
    #[has_many]
    pub todos: dbkit::HasMany<Todo>,
}

#[model(table = "todos")]
pub struct Todo {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    #[belongs_to(key = user_id, references = id)]
    pub user: dbkit::BelongsTo<User>,
    #[many_to_many(through = TodoTag, left_key = todo_id, right_key = tag_id)]
    pub tags: dbkit::ManyToMany<Tag>,
}

#[model(table = "nullable_rows")]
pub struct NullableRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub note: Option<String>,
}

#[model(table = "events")]
pub struct Event {
    #[key]
    pub id: Uuid,
    pub name: String,
    pub starts_at: NaiveDateTime,
    pub day: NaiveDate,
    pub starts_at_time: NaiveTime,
}

#[model(table = "tags")]
pub struct Tag {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
    #[many_to_many(through = TodoTag, left_key = tag_id, right_key = todo_id)]
    pub todos: dbkit::ManyToMany<Todo>,
}

#[model(table = "profiles")]
pub struct Profile {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub tags: Vec<String>,
}

#[model(table = "json_rows")]
pub struct JsonRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub data: serde_json::Value,
}

#[model(table = "func_rows")]
pub struct FuncRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub email: Option<String>,
    pub backup_email: Option<String>,
    pub region: Option<String>,
    pub starts_at: NaiveDateTime,
}

#[model(table = "text_samples")]
pub struct TextSample {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub label: String,
    pub body: Option<String>,
}

#[model(table = "sales")]
pub struct Sale {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub region: String,
    pub amount: i64,
    pub created_at: NaiveDateTime,
}

#[model(table = "todo_tags")]
pub struct TodoTag {
    #[key]
    pub todo_id: i64,
    #[key]
    pub tag_id: i64,
}

#[model(table = "order_lines")]
pub struct OrderLine {
    #[key]
    pub order_id: i64,
    #[key]
    pub line_id: i64,
    pub note: String,
}

#[model(table = "run_payloads")]
pub struct RunPayload {
    #[key]
    pub target_id: i64,
    #[key]
    pub run_id: i64,
    pub payload: String,
    pub source: String,
    pub version: i64,
}

#[model(table = "renamed_parents")]
pub struct RenamedParent {
    #[key]
    #[autoincrement]
    pub id: i64,
    #[dbkit(column = "type")]
    pub type_: String,
    #[dbkit(column = "external_ref")]
    pub external_reference: String,
    pub label: String,
    #[has_many]
    pub children: dbkit::HasMany<RenamedChild>,
}

#[model(table = "renamed_children")]
pub struct RenamedChild {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub parent_id: i64,
    #[dbkit(column = "type")]
    pub type_: String,
    #[dbkit(column = "sort_key")]
    pub rank_key: i64,
    #[belongs_to(key = parent_id, references = id)]
    pub parent: dbkit::BelongsTo<RenamedParent>,
}

#[model(table = "dbkit_lock_rows")]
pub struct LockRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub token: Uuid,
    pub note: String,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    let statements = [
        "CREATE TEMP TABLE users (\
            id BIGSERIAL PRIMARY KEY,\
            name TEXT NOT NULL,\
            email TEXT NOT NULL\
        )",
        "CREATE TEMP TABLE todos (\
            id BIGSERIAL PRIMARY KEY,\
            user_id BIGINT NOT NULL,\
            title TEXT NOT NULL\
        )",
        "CREATE TEMP TABLE tags (\
            id BIGSERIAL PRIMARY KEY,\
            name TEXT NOT NULL\
        )",
        "CREATE TEMP TABLE profiles (\
            id BIGSERIAL PRIMARY KEY,\
            tags TEXT[] NOT NULL\
        )",
        "CREATE TEMP TABLE json_rows (\
            id BIGSERIAL PRIMARY KEY,\
            data JSONB NOT NULL\
        )",
        "CREATE TEMP TABLE func_rows (\
            id BIGSERIAL PRIMARY KEY,\
            email TEXT,\
            backup_email TEXT,\
            region TEXT,\
            starts_at TIMESTAMP NOT NULL\
        )",
        "CREATE TEMP TABLE text_samples (\
            id BIGSERIAL PRIMARY KEY,\
            label TEXT NOT NULL,\
            body TEXT\
        )",
        "CREATE TEMP TABLE sales (\
            id BIGSERIAL PRIMARY KEY,\
            region TEXT NOT NULL,\
            amount BIGINT NOT NULL,\
            created_at TIMESTAMP NOT NULL\
        )",
        "CREATE TEMP TABLE todo_tags (\
            todo_id BIGINT NOT NULL,\
            tag_id BIGINT NOT NULL,\
            PRIMARY KEY (todo_id, tag_id)\
        )",
        "CREATE TEMP TABLE events (\
            id UUID PRIMARY KEY,\
            name TEXT NOT NULL,\
            starts_at TIMESTAMP NOT NULL,\
            day DATE NOT NULL,\
            starts_at_time TIME NOT NULL\
        )",
        "CREATE TEMP TABLE nullable_rows (\
            id BIGSERIAL PRIMARY KEY,\
            note TEXT NULL\
        )",
        "CREATE TEMP TABLE order_lines (\
            order_id BIGINT NOT NULL,\
            line_id BIGINT NOT NULL,\
            note TEXT NOT NULL,\
            PRIMARY KEY (order_id, line_id)\
        )",
        "CREATE TEMP TABLE run_payloads (\
            target_id BIGINT NOT NULL,\
            run_id BIGINT NOT NULL,\
            payload TEXT NOT NULL,\
            source TEXT NOT NULL,\
            version BIGINT NOT NULL,\
            PRIMARY KEY (target_id, run_id)\
        )",
        "CREATE TEMP TABLE renamed_parents (\
            id BIGSERIAL PRIMARY KEY,\
            type TEXT NOT NULL,\
            external_ref TEXT NOT NULL,\
            label TEXT NOT NULL,\
            UNIQUE (type)\
        )",
        "CREATE TEMP TABLE renamed_children (\
            id BIGSERIAL PRIMARY KEY,\
            parent_id BIGINT NOT NULL,\
            type TEXT NOT NULL,\
            sort_key BIGINT NOT NULL\
        )",
    ];

    for statement in statements {
        ex.execute(statement, PgArguments::default()).await?;
    }

    Ok(())
}

async fn setup_locking_schema(db: &Database) -> Result<(), dbkit::Error> {
    // Serialize DDL across parallel test workers to avoid Postgres catalog races on CREATE TABLE IF NOT EXISTS.
    let tx = db.begin().await?;
    tx.execute("SELECT pg_advisory_xact_lock(816726, 1)", PgArguments::default())
        .await?;
    tx.execute(
        "CREATE TABLE IF NOT EXISTS dbkit_lock_rows (\
            id BIGSERIAL PRIMARY KEY,\
            token UUID NOT NULL,\
            note TEXT NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_dbkit_lock_rows_token ON dbkit_lock_rows(token)",
        PgArguments::default(),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn seed_lock_row<E: Executor + Send + Sync>(ex: &E, token: Uuid, note: &str) -> Result<LockRow, dbkit::Error> {
    let row = LockRow::insert(LockRowInsert {
        token,
        note: note.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted lock row");
    Ok(row)
}

async fn cleanup_lock_rows<E: Executor + Send + Sync>(ex: &E, token: Uuid) -> Result<(), dbkit::Error> {
    LockRow::delete().filter(LockRow::token.eq(token)).execute(ex).await?;
    Ok(())
}

fn is_lock_not_available(err: &dbkit::Error) -> bool {
    match err {
        dbkit::Error::Sqlx(sqlx_err) => sqlx_err
            .as_database_error()
            .and_then(|db_err| db_err.code())
            .map(|code| code.as_ref() == "55P03")
            .unwrap_or(false),
        _ => false,
    }
}

fn unique_lock_token() -> Uuid {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let pid = std::process::id() as u128;
    Uuid::from_u128((pid << 64) | seq)
}

async fn seed_user<E: Executor + Send + Sync>(ex: &E, name: &str, email: &str) -> Result<User, dbkit::Error> {
    let user = User::insert(UserInsert {
        name: name.to_string(),
        email: email.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted user");
    Ok(user)
}

async fn seed_todo<E: Executor + Send + Sync>(ex: &E, user_id: i64, title: &str) -> Result<Todo, dbkit::Error> {
    let todo = Todo::insert(TodoInsert {
        user_id,
        title: title.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted todo");
    Ok(todo)
}

async fn seed_tag<E: Executor + Send + Sync>(ex: &E, name: &str) -> Result<Tag, dbkit::Error> {
    let tag = Tag::insert(TagInsert { name: name.to_string() })
        .returning_all()
        .one(ex)
        .await?
        .expect("inserted tag");
    Ok(tag)
}

async fn seed_todo_tag<E: Executor + Send + Sync>(ex: &E, todo_id: i64, tag_id: i64) -> Result<TodoTag, dbkit::Error> {
    let row = TodoTag::insert(TodoTagInsert { todo_id, tag_id })
        .returning_all()
        .one(ex)
        .await?
        .expect("inserted todo_tag");
    Ok(row)
}

async fn seed_event<E: Executor + Send + Sync>(
    ex: &E,
    id: Uuid,
    name: &str,
    starts_at: NaiveDateTime,
    day: NaiveDate,
    starts_at_time: NaiveTime,
) -> Result<Event, dbkit::Error> {
    let event = Event::insert(EventInsert {
        id,
        name: name.to_string(),
        starts_at,
        day,
        starts_at_time,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted event");
    Ok(event)
}

async fn seed_nullable_row<E: Executor + Send + Sync>(ex: &E, note: Option<String>) -> Result<NullableRow, dbkit::Error> {
    let row = NullableRow::insert(NullableRowInsert { note })
        .returning_all()
        .one(ex)
        .await?
        .expect("inserted nullable row");
    Ok(row)
}

async fn seed_text_sample<E: Executor + Send + Sync>(ex: &E, label: &str, body: Option<&str>) -> Result<TextSample, dbkit::Error> {
    let row = TextSample::insert(TextSampleInsert {
        label: label.to_string(),
        body: body.map(str::to_string),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted text sample");
    Ok(row)
}

async fn seed_order_line<E: Executor + Send + Sync>(ex: &E, order_id: i64, line_id: i64, note: &str) -> Result<OrderLine, dbkit::Error> {
    let row = OrderLine::insert(OrderLineInsert {
        order_id,
        line_id,
        note: note.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted order line");
    Ok(row)
}

async fn seed_run_payload<E: Executor + Send + Sync>(
    ex: &E,
    target_id: i64,
    run_id: i64,
    payload: &str,
    source: &str,
    version: i64,
) -> Result<RunPayload, dbkit::Error> {
    let row = RunPayload::insert(RunPayloadInsert {
        target_id,
        run_id,
        payload: payload.to_string(),
        source: source.to_string(),
        version,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted run payload");
    Ok(row)
}

async fn seed_renamed_parent<E: Executor + Send + Sync>(
    ex: &E,
    type_: &str,
    external_reference: &str,
    label: &str,
) -> Result<RenamedParent, dbkit::Error> {
    let row = RenamedParent::insert(RenamedParentInsert {
        type_: type_.to_string(),
        external_reference: external_reference.to_string(),
        label: label.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted renamed parent");
    Ok(row)
}

async fn seed_renamed_child<E: Executor + Send + Sync>(
    ex: &E,
    parent_id: i64,
    type_: &str,
    rank_key: i64,
) -> Result<RenamedChild, dbkit::Error> {
    let row = RenamedChild::insert(RenamedChildInsert {
        parent_id,
        type_: type_.to_string(),
        rank_key,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted renamed child");
    Ok(row)
}

#[tokio::test]
async fn insert_update_delete_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Alex", "a@b.com").await?;
    assert!(user.id > 0);
    assert_eq!(user.name, "Alex");

    let updated = User::update()
        .set(User::name, "Updated")
        .filter(User::id.eq(user.id))
        .returning_all()
        .all(&tx)
        .await?;
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].name, "Updated");

    let deleted = User::delete().filter(User::id.eq(user.id)).execute(&tx).await?;
    assert_eq!(deleted, 1);

    let remaining = User::query().all(&tx).await?;
    assert!(remaining.is_empty());

    Ok(())
}

#[tokio::test]
async fn renamed_columns_roundtrip_through_crud_filters_and_joined_loads() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let parent = seed_renamed_parent(&tx, "primary", "ref-1", "Example").await?;
    assert_eq!(parent.type_, "primary");
    assert_eq!(parent.external_reference, "ref-1");
    assert_eq!(parent.label, "Example");

    let child = seed_renamed_child(&tx, parent.id, "child", 10).await?;
    assert_eq!(child.type_, "child");
    assert_eq!(child.rank_key, 10);

    let filtered = RenamedParent::query()
        .filter(RenamedParent::type_.eq("primary"))
        .filter(RenamedParent::external_reference.eq("ref-1"))
        .one(&tx)
        .await?
        .expect("filtered parent");
    assert_eq!(filtered.id, parent.id);
    assert_eq!(filtered.type_, "primary");

    let updated_rows = RenamedParent::update()
        .set(RenamedParent::type_, "secondary")
        .set(RenamedParent::external_reference, "ref-2")
        .filter(RenamedParent::id.eq(parent.id))
        .returning_all()
        .all(&tx)
        .await?;
    assert_eq!(updated_rows.len(), 1);
    let updated = &updated_rows[0];
    assert_eq!(updated.type_, "secondary");
    assert_eq!(updated.external_reference, "ref-2");

    let loaded = RenamedParent::query()
        .filter(RenamedParent::id.eq(parent.id))
        .with(RenamedParent::children.joined())
        .one(&tx)
        .await?
        .expect("joined parent");
    assert_eq!(loaded.type_, "secondary");
    assert_eq!(loaded.external_reference, "ref-2");
    assert_eq!(loaded.children_loaded().len(), 1);
    assert_eq!(loaded.children_loaded()[0].type_, "child");
    assert_eq!(loaded.children_loaded()[0].rank_key, 10);

    Ok(())
}

#[tokio::test]
async fn insert_many_inserts_multiple_rows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let inserted = User::insert_many(vec![
        UserInsert {
            name: "Alpha".to_string(),
            email: "alpha@db.com".to_string(),
        },
        UserInsert {
            name: "Beta".to_string(),
            email: "beta@db.com".to_string(),
        },
    ])
    .execute(&tx)
    .await?;
    assert_eq!(inserted, 2);

    let users = User::query().order_by(dbkit::Order::asc(User::id.as_ref())).all(&tx).await?;
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alpha");
    assert_eq!(users[1].name, "Beta");

    Ok(())
}

#[tokio::test]
async fn on_conflict_do_nothing_ignores_duplicate_composite_key() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let original = seed_run_payload(&tx, 100, 200, "payload-v1", "ingest-a", 1).await?;
    assert_eq!(original.payload, "payload-v1");

    let affected = RunPayload::insert(RunPayloadInsert {
        target_id: 100,
        run_id: 200,
        payload: "payload-v2".to_string(),
        source: "ingest-b".to_string(),
        version: 2,
    })
    .on_conflict_do_nothing((RunPayload::target_id, RunPayload::run_id))
    .execute(&tx)
    .await?;
    assert_eq!(affected, 0);

    let rows = RunPayload::query()
        .filter(RunPayload::target_id.eq(100))
        .filter(RunPayload::run_id.eq(200))
        .all(&tx)
        .await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].payload, "payload-v1");
    assert_eq!(rows[0].source, "ingest-a");
    assert_eq!(rows[0].version, 1);

    Ok(())
}

#[tokio::test]
async fn on_conflict_do_nothing_with_returning_all_returns_none_on_conflict() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let _original = seed_run_payload(&tx, 10, 20, "payload-v1", "ingest-a", 1).await?;

    let row: Option<RunPayload> = RunPayload::insert(RunPayloadInsert {
        target_id: 10,
        run_id: 20,
        payload: "payload-v2".to_string(),
        source: "ingest-b".to_string(),
        version: 2,
    })
    .on_conflict_do_nothing((RunPayload::target_id, RunPayload::run_id))
    .returning_all()
    .one(&tx)
    .await?;

    assert!(row.is_none());

    Ok(())
}

#[tokio::test]
async fn on_conflict_do_update_updates_only_selected_columns_on_conflict() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let _original = seed_run_payload(&tx, 500, 600, "payload-v1", "ingest-a", 1).await?;

    let affected = RunPayload::insert(RunPayloadInsert {
        target_id: 500,
        run_id: 600,
        payload: "payload-v2".to_string(),
        source: "ingest-b".to_string(),
        version: 2,
    })
    .on_conflict_do_update(
        (RunPayload::target_id, RunPayload::run_id),
        (RunPayload::payload, RunPayload::version),
    )
    .execute(&tx)
    .await?;
    assert_eq!(affected, 1);

    let rows = RunPayload::query()
        .filter(RunPayload::target_id.eq(500))
        .filter(RunPayload::run_id.eq(600))
        .all(&tx)
        .await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].payload, "payload-v2");
    assert_eq!(rows[0].version, 2);
    assert_eq!(rows[0].source, "ingest-a");

    Ok(())
}

#[tokio::test]
async fn on_conflict_do_update_inserts_when_no_conflict_exists() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let affected = RunPayload::insert(RunPayloadInsert {
        target_id: 700,
        run_id: 800,
        payload: "payload-v1".to_string(),
        source: "ingest-a".to_string(),
        version: 1,
    })
    .on_conflict_do_update(
        (RunPayload::target_id, RunPayload::run_id),
        (RunPayload::payload, RunPayload::version),
    )
    .execute(&tx)
    .await?;
    assert_eq!(affected, 1);

    let rows = RunPayload::query()
        .filter(RunPayload::target_id.eq(700))
        .filter(RunPayload::run_id.eq(800))
        .all(&tx)
        .await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].payload, "payload-v1");
    assert_eq!(rows[0].source, "ingest-a");
    assert_eq!(rows[0].version, 1);

    Ok(())
}

#[tokio::test]
async fn selectin_has_many_loads_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Riley", "r@b.com").await?;
    let _todo1 = seed_todo(&tx, user.id, "Write tests").await?;
    let _todo2 = seed_todo(&tx, user.id, "Ship code").await?;

    let users: Vec<User<Vec<Todo>>> = User::query()
        .filter(User::id.eq(user.id))
        .with(User::todos.selectin())
        .all(&tx)
        .await?;

    assert_eq!(users.len(), 1);
    let mut titles: Vec<String> = users[0].todos.iter().map(|todo| todo.title.clone()).collect();
    titles.sort();
    assert_eq!(titles, vec!["Ship code", "Write tests"]);

    Ok(())
}

#[tokio::test]
async fn selectin_belongs_to_loads_parent() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Dana", "d@b.com").await?;
    let todo = seed_todo(&tx, user.id, "Map relations").await?;

    let todos: Vec<Todo<Option<User>>> = Todo::query()
        .filter(Todo::id.eq(todo.id))
        .with(Todo::user.selectin())
        .all(&tx)
        .await?;

    assert_eq!(todos.len(), 1);
    let loaded = todos[0].user.as_ref().expect("loaded user");
    assert_eq!(loaded.id, user.id);
    assert_eq!(loaded.email, "d@b.com");

    Ok(())
}

#[tokio::test]
async fn joined_has_many_loads_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Joined", "joined@db.com").await?;
    let _todo1 = seed_todo(&tx, user.id, "Joined A").await?;
    let _todo2 = seed_todo(&tx, user.id, "Joined B").await?;

    let users: Vec<User<Vec<Todo>>> = User::query()
        .filter(User::id.eq(user.id))
        .with(User::todos.joined())
        .all(&tx)
        .await?;

    assert_eq!(users.len(), 1);
    let mut titles: Vec<String> = users[0].todos.iter().map(|todo| todo.title.clone()).collect();
    titles.sort();
    assert_eq!(titles, vec!["Joined A", "Joined B"]);

    Ok(())
}

#[tokio::test]
async fn joined_has_many_includes_empty_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Empty", "empty@db.com").await?;

    let users: Vec<User<Vec<Todo>>> = User::query()
        .filter(User::id.eq(user.id))
        .with(User::todos.joined())
        .all(&tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert!(users[0].todos.is_empty());

    Ok(())
}

#[tokio::test]
async fn joined_has_many_filters_children_when_join_filtered() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Filter", "filter@db.com").await?;
    let _todo_keep = seed_todo(&tx, user.id, "Keep").await?;
    let _todo_drop = seed_todo(&tx, user.id, "Drop").await?;

    let users: Vec<User<Vec<Todo>>> = User::query()
        .join(User::todos)
        .filter(Todo::title.eq("Keep"))
        .distinct()
        .with(User::todos.joined())
        .all(&tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].todos.len(), 1);
    assert_eq!(users[0].todos[0].title, "Keep");

    Ok(())
}

#[tokio::test]
async fn joined_belongs_to_loads_parent() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Joined Parent", "joined-parent@db.com").await?;
    let todo = seed_todo(&tx, user.id, "Joined child").await?;

    let todos: Vec<Todo<Option<User>>> = Todo::query()
        .filter(Todo::id.eq(todo.id))
        .with(Todo::user.joined())
        .all(&tx)
        .await?;

    assert_eq!(todos.len(), 1);
    let loaded = todos[0].user.as_ref().expect("loaded user");
    assert_eq!(loaded.id, user.id);
    assert_eq!(loaded.email, "joined-parent@db.com");

    Ok(())
}

#[tokio::test]
async fn joined_nested_filters_children_when_join_filtered() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Nested", "nested@db.com").await?;
    let todo_keep = seed_todo(&tx, user.id, "Keep").await?;
    let todo_drop = seed_todo(&tx, user.id, "Drop").await?;

    let tag_a = seed_tag(&tx, "A").await?;
    let tag_b = seed_tag(&tx, "B").await?;

    let _keep_a = seed_todo_tag(&tx, todo_keep.id, tag_a.id).await?;
    let _drop_b = seed_todo_tag(&tx, todo_drop.id, tag_b.id).await?;

    let users: Vec<User<Vec<Todo<dbkit::NotLoaded, Vec<Tag>>>>> = User::query()
        .join(User::todos)
        .filter(Todo::title.eq("Keep"))
        .distinct()
        .with(User::todos.joined().with(Todo::tags.joined()))
        .all(&tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].todos.len(), 1);
    assert_eq!(users[0].todos[0].title, "Keep");
    assert_eq!(users[0].todos[0].tags.len(), 1);
    assert_eq!(users[0].todos[0].tags[0].name, "A");

    Ok(())
}

#[tokio::test]
async fn joined_many_to_many_filters_children_when_join_filtered() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Tags", "tags@db.com").await?;
    let todo = seed_todo(&tx, user.id, "Tagged").await?;

    let tag_a = seed_tag(&tx, "A").await?;
    let tag_b = seed_tag(&tx, "B").await?;

    let _link_a = seed_todo_tag(&tx, todo.id, tag_a.id).await?;
    let _link_b = seed_todo_tag(&tx, todo.id, tag_b.id).await?;

    let todos: Vec<Todo<dbkit::NotLoaded, Vec<Tag>>> = Todo::query()
        .join(Todo::tags)
        .filter(Tag::name.eq("A"))
        .distinct()
        .with(Todo::tags.joined())
        .all(&tx)
        .await?;

    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].tags.len(), 1);
    assert_eq!(todos[0].tags[0].name, "A");

    Ok(())
}

#[tokio::test]
async fn nested_selectin_loads() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Jo", "jo@b.com").await?;
    let _todo = seed_todo(&tx, user.id, "Chain loads").await?;

    let users = User::query() // should be Vec<User<Vec<Todo<Option<User>>>>>
        .filter(User::id.eq(user.id))
        .with(User::todos.selectin().with(Todo::user.selectin()))
        .all(&tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].todos.len(), 1);
    let nested_user = users[0].todos[0].user.as_ref().expect("nested user");
    assert_eq!(nested_user.id, user.id);

    Ok(())
}

#[tokio::test]
async fn lazy_load_relation() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Ari", "ari@b.com").await?;
    let _todo = seed_todo(&tx, user.id, "Lazy load").await?;

    let loaded = user.load(User::todos, &tx).await?;
    assert_eq!(loaded.todos.len(), 1);
    assert_eq!(loaded.todos[0].title, "Lazy load");

    Ok(())
}

#[tokio::test]
async fn join_filter_on_child_table() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user_keep = seed_user(&tx, "Keep", "keep@db.com").await?;
    let user_drop = seed_user(&tx, "Drop", "drop@db.com").await?;
    let _todo_keep = seed_todo(&tx, user_keep.id, "Keep me").await?;
    let _todo_other = seed_todo(&tx, user_keep.id, "Also me").await?;
    let _todo_drop = seed_todo(&tx, user_drop.id, "Ignore me").await?;

    let users = User::query()
        .join(User::todos)
        .filter(Todo::title.eq("Keep me"))
        .distinct()
        .all(&tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].id, user_keep.id);

    Ok(())
}

#[tokio::test]
async fn uuid_date_time_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let date = NaiveDate::from_ymd_opt(2024, 1, 2).expect("date");
    let time = NaiveTime::from_hms_opt(3, 4, 5).expect("time");
    let starts_at = NaiveDateTime::new(date, time);
    let id = Uuid::nil();

    let inserted = seed_event(&tx, id, "Launch", starts_at, date, time).await?;
    assert_eq!(inserted.id, id);
    assert_eq!(inserted.starts_at, starts_at);
    assert_eq!(inserted.day, date);
    assert_eq!(inserted.starts_at_time, time);

    let found = Event::query()
        .filter(Event::id.eq(id))
        .filter(Event::day.eq(date))
        .filter(Event::starts_at.eq(starts_at))
        .filter(Event::starts_at_time.eq(time))
        .one(&tx)
        .await?
        .expect("event");

    assert_eq!(found.id, id);
    assert_eq!(found.name, "Launch");
    assert_eq!(found.starts_at, starts_at);
    assert_eq!(found.day, date);
    assert_eq!(found.starts_at_time, time);

    Ok(())
}

#[tokio::test]
async fn insert_update_and_filter_nulls() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let inserted = seed_nullable_row(&tx, None).await?;
    assert!(inserted.note.is_none());

    let some_row = seed_nullable_row(&tx, Some("hello".to_string())).await?;
    assert_eq!(some_row.note.as_deref(), Some("hello"));

    let updated = NullableRow::update()
        .set(NullableRow::note, "direct")
        .filter(NullableRow::id.eq(some_row.id))
        .returning_all()
        .all(&tx)
        .await?;
    assert_eq!(updated[0].note.as_deref(), Some("direct"));

    let updated = NullableRow::update()
        .set(NullableRow::note, "optional")
        .filter(NullableRow::id.eq(some_row.id))
        .returning_all()
        .all(&tx)
        .await?;
    assert_eq!(updated[0].note.as_deref(), Some("optional"));

    let direct_match = NullableRow::query()
        .filter(NullableRow::note.eq("optional"))
        .one(&tx)
        .await?
        .expect("direct nullable value match");
    assert_eq!(direct_match.id, some_row.id);

    let optional_match = NullableRow::query()
        .filter(NullableRow::note.eq("optional"))
        .one(&tx)
        .await?
        .expect("optional nullable value match");
    assert_eq!(optional_match.id, some_row.id);

    let updated = NullableRow::update()
        .set(NullableRow::note, None)
        .filter(NullableRow::id.eq(some_row.id))
        .returning_all()
        .all(&tx)
        .await?;
    assert_eq!(updated.len(), 1);
    assert!(updated[0].note.is_none());

    let null_rows = NullableRow::query().filter(NullableRow::note.eq(None)).all(&tx).await?;
    assert_eq!(null_rows.len(), 2);
    assert!(null_rows.iter().all(|row| row.note.is_none()));

    Ok(())
}

#[tokio::test]
async fn array_column_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let tags = vec!["alpha".to_string(), "beta".to_string()];
    let inserted = Profile::insert(ProfileInsert { tags: tags.clone() })
        .returning_all()
        .one(&tx)
        .await?
        .expect("inserted profile");
    assert_eq!(inserted.tags, tags);

    let matched = Profile::query().filter(Profile::tags.eq(tags.clone())).all(&tx).await?;
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id, inserted.id);

    let updated_tags = vec!["gamma".to_string(), "delta".to_string()];
    let mut active = inserted.into_active();
    active.tags = updated_tags.clone().into();
    let updated = active.update(&tx).await?;
    assert_eq!(updated.tags, updated_tags);

    let fetched = Profile::by_id(updated.id).one(&tx).await?.expect("updated profile");
    assert_eq!(fetched.tags, updated_tags);

    Ok(())
}

#[tokio::test]
async fn json_column_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let payload = json!({"name": "alpha", "active": true});
    let inserted = JsonRow::insert(JsonRowInsert { data: payload.clone() })
        .returning_all()
        .one(&tx)
        .await?
        .expect("inserted json row");
    assert_eq!(inserted.data, payload);

    let matched = JsonRow::query().filter(JsonRow::data.eq(payload.clone())).all(&tx).await?;
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id, inserted.id);

    let updated_payload = json!({"name": "beta", "active": false});
    let mut active = inserted.into_active();
    active.data = updated_payload.clone().into();
    let updated = active.update(&tx).await?;
    assert_eq!(updated.data, updated_payload);

    let fetched = JsonRow::by_id(updated.id).one(&tx).await?.expect("updated json row");
    assert_eq!(fetched.data, updated_payload);

    Ok(())
}

#[tokio::test]
async fn function_expressions_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 1, 2).expect("day");
    let day_start = NaiveDateTime::new(day, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));
    let later_day = NaiveDate::from_ymd_opt(2024, 1, 3).expect("day");
    let later_start = NaiveDateTime::new(later_day, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));

    let row1 = FuncRow::insert(FuncRowInsert {
        email: Some("alpha@ex.com".to_string()),
        backup_email: None,
        region: Some("us".to_string()),
        starts_at: NaiveDateTime::new(day, NaiveTime::from_hms_opt(10, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&tx)
    .await?
    .expect("row1");

    let row2 = FuncRow::insert(FuncRowInsert {
        email: None,
        backup_email: Some("beta@ex.com".to_string()),
        region: Some("eu".to_string()),
        starts_at: NaiveDateTime::new(day, NaiveTime::from_hms_opt(12, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&tx)
    .await?
    .expect("row2");

    let row3 = FuncRow::insert(FuncRowInsert {
        email: None,
        backup_email: None,
        region: Some("uk".to_string()),
        starts_at: NaiveDateTime::new(later_day, NaiveTime::from_hms_opt(9, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&tx)
    .await?
    .expect("row3");

    let row4 = FuncRow::insert(FuncRowInsert {
        email: Some("gamma@ex.com".to_string()),
        backup_email: Some("backup@ex.com".to_string()),
        region: None,
        starts_at: NaiveDateTime::new(later_day, NaiveTime::from_hms_opt(15, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&tx)
    .await?
    .expect("row4");

    let upper_match = FuncRow::query()
        .filter(dbkit::func::upper(dbkit::func::coalesce(FuncRow::email, FuncRow::backup_email)).eq("BETA@EX.COM"))
        .all(&tx)
        .await?;
    assert_eq!(upper_match.len(), 1);
    assert_eq!(upper_match[0].id, row2.id);

    let fallback_match = FuncRow::query()
        .filter(dbkit::func::coalesce(FuncRow::email, "fallback").eq("fallback"))
        .all(&tx)
        .await?;
    let mut fallback_ids: Vec<i64> = fallback_match.iter().map(|row| row.id).collect();
    fallback_ids.sort();
    assert_eq!(fallback_ids, vec![row2.id, row3.id]);

    let nested_match = FuncRow::query()
        .filter(dbkit::func::coalesce(dbkit::func::coalesce(FuncRow::email, FuncRow::backup_email), "none").eq("none"))
        .all(&tx)
        .await?;
    assert_eq!(nested_match.len(), 1);
    assert_eq!(nested_match[0].id, row3.id);

    let truncated_match = FuncRow::query()
        .filter(dbkit::func::date_trunc("day", FuncRow::starts_at).eq(day_start))
        .all(&tx)
        .await?;
    let mut day_ids: Vec<i64> = truncated_match.iter().map(|row| row.id).collect();
    day_ids.sort();
    assert_eq!(day_ids, vec![row1.id, row2.id]);

    let region_match = FuncRow::query()
        .filter(dbkit::func::upper(dbkit::func::coalesce(FuncRow::region, "unknown")).eq("UNKNOWN"))
        .all(&tx)
        .await?;
    assert_eq!(region_match.len(), 1);
    assert_eq!(region_match[0].id, row4.id);

    let combined_match = FuncRow::query()
        .filter(dbkit::func::upper(dbkit::func::coalesce(FuncRow::email, FuncRow::backup_email)).eq("ALPHA@EX.COM"))
        .filter(dbkit::func::date_trunc("day", FuncRow::starts_at).eq(day_start))
        .all(&tx)
        .await?;
    assert_eq!(combined_match.len(), 1);
    assert_eq!(combined_match[0].id, row1.id);

    let _ = later_start;

    Ok(())
}

#[tokio::test]
async fn trim_filter_matches_trimmed_text() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let exact = seed_text_sample(&tx, "exact", Some("alpha")).await?;
    let padded = seed_text_sample(&tx, "padded", Some("  alpha  ")).await?;
    let _different = seed_text_sample(&tx, "different", Some("beta")).await?;
    let _blank = seed_text_sample(&tx, "blank", Some("   ")).await?;
    let _missing = seed_text_sample(&tx, "missing", None).await?;

    let matches = TextSample::query()
        .filter(dbkit::func::trim(TextSample::body).eq("alpha"))
        .all(&tx)
        .await?;

    let mut ids: Vec<i64> = matches.into_iter().map(|row| row.id).collect();
    ids.sort();
    assert_eq!(ids, vec![exact.id, padded.id]);

    Ok(())
}

#[tokio::test]
async fn char_length_of_trimmed_nullable_text_filters_rows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let _short = seed_text_sample(&tx, "short", Some("  abc  ")).await?;
    let long = seed_text_sample(&tx, "long", Some("  abcdef  ")).await?;
    let spaced = seed_text_sample(&tx, "spaced", Some("     abcde     ")).await?;
    let _blank = seed_text_sample(&tx, "blank", Some("    ")).await?;
    let _missing = seed_text_sample(&tx, "missing", None).await?;

    let matches = TextSample::query()
        .filter(TextSample::body.is_not_null())
        .filter(dbkit::func::char_length(dbkit::func::trim(TextSample::body)).ge(5_i32))
        .all(&tx)
        .await?;

    let mut ids: Vec<i64> = matches.into_iter().map(|row| row.id).collect();
    ids.sort();
    assert_eq!(ids, vec![long.id, spaced.id]);

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct StringLengthResult {
    label: String,
    characters: Option<i32>,
    bytes: Option<i32>,
    bits: Option<i32>,
}

#[tokio::test]
async fn string_lengths_distinguish_characters_bytes_and_bits() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_text_sample(&tx, "ascii", Some("abc")).await?;
    seed_text_sample(&tx, "unicode", Some("é🙂")).await?;
    seed_text_sample(&tx, "empty", Some("")).await?;
    seed_text_sample(&tx, "whitespace", Some(" \t")).await?;
    seed_text_sample(&tx, "missing", None).await?;

    let rows: Vec<StringLengthResult> = TextSample::query()
        .select_only()
        .column(TextSample::label)
        .column_as(dbkit::func::char_length(TextSample::body), "characters")
        .column_as(dbkit::func::byte_length(TextSample::body), "bytes")
        .column_as(dbkit::func::bit_length(TextSample::body), "bits")
        .order_by(dbkit::Order::asc(TextSample::id))
        .into_model()
        .all(&tx)
        .await?;

    let values: Vec<_> = rows
        .into_iter()
        .map(|row| (row.label, row.characters, row.bytes, row.bits))
        .collect();
    assert_eq!(
        values,
        vec![
            ("ascii".to_string(), Some(3), Some(3), Some(24)),
            ("unicode".to_string(), Some(2), Some(6), Some(48)),
            ("empty".to_string(), Some(0), Some(0), Some(0)),
            ("whitespace".to_string(), Some(2), Some(2), Some(16)),
            ("missing".to_string(), None, None, None),
        ]
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct StringSearchResult {
    needle: String,
    position: Option<i32>,
    starts_with: Option<bool>,
    reverse_position: Option<i32>,
    reverse_starts_with: Option<bool>,
}

#[tokio::test]
async fn string_searches_cover_postgresql_boundaries_and_nulls() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_text_sample(&tx, "", Some("abc")).await?;
    seed_text_sample(&tx, "abc", Some("abc")).await?;
    seed_text_sample(&tx, "abcd", Some("abc")).await?;
    seed_text_sample(&tx, "bc", Some("abcabc")).await?;
    seed_text_sample(&tx, "aba", Some("xababa")).await?;
    seed_text_sample(&tx, "A", Some("abc")).await?;
    seed_text_sample(&tx, "é🙂", Some("xé🙂é")).await?;
    seed_text_sample(&tx, "x", Some("abc")).await?;
    seed_text_sample(&tx, "missing", None).await?;

    let rows: Vec<StringSearchResult> = TextSample::query()
        .select_only()
        .column_as(TextSample::label, "needle")
        .column_as(dbkit::func::position(TextSample::body, TextSample::label), "position")
        .column_as(dbkit::func::starts_with(TextSample::body, TextSample::label), "starts_with")
        .column_as(dbkit::func::position(TextSample::label, TextSample::body), "reverse_position")
        .column_as(dbkit::func::starts_with(TextSample::label, TextSample::body), "reverse_starts_with")
        .order_by(dbkit::Order::asc(TextSample::id))
        .into_model()
        .all(&tx)
        .await?;

    let values: Vec<_> = rows
        .into_iter()
        .map(|row| {
            (
                row.needle,
                row.position,
                row.starts_with,
                row.reverse_position,
                row.reverse_starts_with,
            )
        })
        .collect();
    assert_eq!(
        values,
        vec![
            ("".to_string(), Some(1), Some(true), Some(0), Some(false)),
            ("abc".to_string(), Some(1), Some(true), Some(1), Some(true)),
            ("abcd".to_string(), Some(0), Some(false), Some(1), Some(true)),
            ("bc".to_string(), Some(2), Some(false), Some(0), Some(false)),
            ("aba".to_string(), Some(2), Some(false), Some(0), Some(false)),
            ("A".to_string(), Some(0), Some(false), Some(0), Some(false)),
            ("é🙂".to_string(), Some(2), Some(false), Some(0), Some(false)),
            ("x".to_string(), Some(0), Some(false), Some(0), Some(false)),
            ("missing".to_string(), None, None, None, None),
        ]
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct BoundStringSearchResult {
    position: i32,
    starts_with: bool,
}

#[tokio::test]
async fn string_search_arguments_remain_bound_values() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let search = "'%_\\); DROP TABLE text_samples; --";
    let row = seed_text_sample(&tx, &format!("{search}suffix"), None).await?;

    let result: BoundStringSearchResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::position(TextSample::label, search), "position")
        .column_as(dbkit::func::starts_with(TextSample::label, search), "starts_with")
        .filter(TextSample::id.eq(row.id))
        .into_model()
        .one(&tx)
        .await?
        .expect("bound search result");

    assert_eq!(result.position, 1);
    assert!(result.starts_with);

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct WhitespaceNormalizationResult {
    label: String,
    lowered: Option<String>,
    trimmed: Option<String>,
    start_trimmed: Option<String>,
    end_trimmed: Option<String>,
}

#[tokio::test]
async fn string_normalization_preserves_direction_and_nullability() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_text_sample(&tx, "mixed", Some("  MiXeD  ")).await?;
    seed_text_sample(&tx, "lower", Some("already lower")).await?;
    seed_text_sample(&tx, "empty", Some("")).await?;
    seed_text_sample(&tx, "spaces", Some("   ")).await?;
    seed_text_sample(&tx, "missing", None).await?;

    let rows: Vec<WhitespaceNormalizationResult> = TextSample::query()
        .select_only()
        .column(TextSample::label)
        .column_as(dbkit::func::lower(TextSample::body), "lowered")
        .column_as(dbkit::func::trim(TextSample::body), "trimmed")
        .column_as(dbkit::func::trim_start(TextSample::body), "start_trimmed")
        .column_as(dbkit::func::trim_end(TextSample::body), "end_trimmed")
        .order_by(dbkit::Order::asc(TextSample::label))
        .into_model()
        .all(&tx)
        .await?;

    let values: Vec<_> = rows
        .into_iter()
        .map(|row| (row.label, row.lowered, row.trimmed, row.start_trimmed, row.end_trimmed))
        .collect();
    assert_eq!(
        values,
        vec![
            (
                "empty".to_string(),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
            ),
            (
                "lower".to_string(),
                Some("already lower".to_string()),
                Some("already lower".to_string()),
                Some("already lower".to_string()),
                Some("already lower".to_string()),
            ),
            ("missing".to_string(), None, None, None, None),
            (
                "mixed".to_string(),
                Some("  mixed  ".to_string()),
                Some("MiXeD".to_string()),
                Some("MiXeD  ".to_string()),
                Some("  MiXeD".to_string()),
            ),
            (
                "spaces".to_string(),
                Some("   ".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
            ),
        ]
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct CustomTrimResult {
    label: String,
    original: Option<String>,
    both: Option<String>,
    start: Option<String>,
    end: Option<String>,
    empty_set: Option<String>,
}

#[tokio::test]
async fn custom_trim_uses_a_character_set_and_handles_edge_cases() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_text_sample(&tx, "all", Some("xyyx")).await?;
    seed_text_sample(&tx, "blocked", Some(" xalphay ")).await?;
    seed_text_sample(&tx, "empty", Some("")).await?;
    seed_text_sample(&tx, "internal", Some("xyxyalxphayx")).await?;
    seed_text_sample(&tx, "missing", None).await?;
    seed_text_sample(&tx, "no_match", Some("alpha")).await?;

    let rows: Vec<CustomTrimResult> = TextSample::query()
        .select_only()
        .column(TextSample::label)
        .column_as(TextSample::body, "original")
        .column_as(dbkit::func::trim_chars(TextSample::body, "xy"), "both")
        .column_as(dbkit::func::trim_start_chars(TextSample::body, "xy"), "start")
        .column_as(dbkit::func::trim_end_chars(TextSample::body, "xy"), "end")
        .column_as(dbkit::func::trim_chars(TextSample::body, ""), "empty_set")
        .order_by(dbkit::Order::asc(TextSample::label))
        .into_model()
        .all(&tx)
        .await?;

    let values: Vec<_> = rows
        .into_iter()
        .map(|row| (row.label, row.original, row.both, row.start, row.end, row.empty_set))
        .collect();
    assert_eq!(
        values,
        vec![
            (
                "all".to_string(),
                Some("xyyx".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("xyyx".to_string()),
            ),
            (
                "blocked".to_string(),
                Some(" xalphay ".to_string()),
                Some(" xalphay ".to_string()),
                Some(" xalphay ".to_string()),
                Some(" xalphay ".to_string()),
                Some(" xalphay ".to_string()),
            ),
            (
                "empty".to_string(),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
            ),
            (
                "internal".to_string(),
                Some("xyxyalxphayx".to_string()),
                Some("alxpha".to_string()),
                Some("alxphayx".to_string()),
                Some("xyxyalxpha".to_string()),
                Some("xyxyalxphayx".to_string()),
            ),
            ("missing".to_string(), None, None, None, None, None),
            (
                "no_match".to_string(),
                Some("alpha".to_string()),
                Some("alpha".to_string()),
                Some("alpha".to_string()),
                Some("alpha".to_string()),
                Some("alpha".to_string()),
            ),
        ]
    );

    let unicode = seed_text_sample(&tx, "unicode", Some("ééalphéaé")).await?;
    let escaped = seed_text_sample(&tx, "escaped", Some("'\\alpha\\'")).await?;

    let unicode_match = TextSample::query()
        .filter(dbkit::func::trim_chars(TextSample::body, "é").eq("alphéa"))
        .one(&tx)
        .await?
        .expect("Unicode trim match");
    assert_eq!(unicode_match.id, unicode.id);

    let escaped_match = TextSample::query()
        .filter(TextSample::label.eq("escaped"))
        .filter(dbkit::func::trim_chars(TextSample::body, "'\\").eq("alpha"))
        .one(&tx)
        .await?
        .expect("bound quote/backslash trim match");
    assert_eq!(escaped_match.id, escaped.id);

    Ok(())
}

#[tokio::test]
async fn nested_normalized_handle_lookup_matches_equivalent_inputs() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let canonical = seed_text_sample(&tx, "alice", None).await?;
    let spaced = seed_text_sample(&tx, " Alice ", None).await?;
    let prefixed = seed_text_sample(&tx, "@Alice", None).await?;
    let revealed_space = seed_text_sample(&tx, " @ Alice ", None).await?;
    let repeated_prefix = seed_text_sample(&tx, "@@@ALICE", None).await?;
    seed_text_sample(&tx, "bob", None).await?;

    let handle = "alice";
    let normalized = dbkit::func::lower(dbkit::func::trim(dbkit::func::trim_start_chars(
        dbkit::func::trim(TextSample::label),
        "@",
    )));
    let matches = TextSample::query().filter(normalized.eq(handle)).all(&tx).await?;

    let mut ids: Vec<_> = matches.into_iter().map(|row| row.id).collect();
    ids.sort();
    let mut expected = vec![canonical.id, spaced.id, prefixed.id, revealed_space.id, repeated_prefix.id];
    expected.sort();
    assert_eq!(ids, expected);

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct ExtractionSizingResult {
    label: String,
    left_value: Option<String>,
    right_value: Option<String>,
    substring_value: Option<String>,
    repeated_value: Option<String>,
    start_padded: Option<String>,
    end_padded: Option<String>,
}

#[tokio::test]
async fn string_extraction_and_sizing_preserve_characters_and_nulls() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_text_sample(&tx, "ascii", Some("abcdef")).await?;
    seed_text_sample(&tx, "empty", Some("")).await?;
    seed_text_sample(&tx, "missing", None).await?;
    seed_text_sample(&tx, "unicode", Some("é🦀界")).await?;

    let rows: Vec<ExtractionSizingResult> = TextSample::query()
        .select_only()
        .column(TextSample::label)
        .column_as(dbkit::func::left(TextSample::body, 2_i32), "left_value")
        .column_as(dbkit::func::right(TextSample::body, 2_i32), "right_value")
        .column_as(dbkit::func::substring(TextSample::body, 2_i32, 3_i32), "substring_value")
        .column_as(dbkit::func::repeat(TextSample::body, 2_i32), "repeated_value")
        .column_as(dbkit::func::pad_start(TextSample::body, 8_i32, "xy"), "start_padded")
        .column_as(dbkit::func::pad_end(TextSample::body, 8_i32, "xy"), "end_padded")
        .order_by(dbkit::Order::asc(TextSample::label))
        .into_model()
        .all(&tx)
        .await?;

    let values: Vec<_> = rows
        .into_iter()
        .map(|row| {
            (
                row.label,
                row.left_value,
                row.right_value,
                row.substring_value,
                row.repeated_value,
                row.start_padded,
                row.end_padded,
            )
        })
        .collect();
    assert_eq!(
        values,
        vec![
            (
                "ascii".to_string(),
                Some("ab".to_string()),
                Some("ef".to_string()),
                Some("bcd".to_string()),
                Some("abcdefabcdef".to_string()),
                Some("xyabcdef".to_string()),
                Some("abcdefxy".to_string()),
            ),
            (
                "empty".to_string(),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("".to_string()),
                Some("xyxyxyxy".to_string()),
                Some("xyxyxyxy".to_string()),
            ),
            ("missing".to_string(), None, None, None, None, None, None),
            (
                "unicode".to_string(),
                Some("é🦀".to_string()),
                Some("🦀界".to_string()),
                Some("🦀界".to_string()),
                Some("é🦀界é🦀界".to_string()),
                Some("xyxyxé🦀界".to_string()),
                Some("é🦀界xyxyx".to_string()),
            ),
        ]
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct StringFunctionBoundaryResult {
    left_zero: String,
    left_long: String,
    left_negative: String,
    right_zero: String,
    right_long: String,
    right_negative: String,
    substring_one: String,
    substring_zero_start: String,
    substring_negative_start: String,
    substring_zero_count: String,
    substring_long_count: String,
    repeat_zero: String,
    repeat_one: String,
    repeat_negative: String,
    pad_start_shorter: String,
    pad_start_equal: String,
    pad_start_longer: String,
    pad_start_single_fill: String,
    pad_start_empty_fill: String,
    pad_start_negative: String,
    pad_end_shorter: String,
    pad_end_equal: String,
    pad_end_longer: String,
    pad_end_single_fill: String,
    pad_end_empty_fill: String,
    pad_end_negative: String,
}

#[tokio::test]
async fn string_extraction_and_sizing_follow_postgres_boundary_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "boundary", None).await?;

    let result: StringFunctionBoundaryResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::left("abcdef", 0_i32), "left_zero")
        .column_as(dbkit::func::left("abcdef", 99_i32), "left_long")
        .column_as(dbkit::func::left("abcdef", -2_i32), "left_negative")
        .column_as(dbkit::func::right("abcdef", 0_i32), "right_zero")
        .column_as(dbkit::func::right("abcdef", 99_i32), "right_long")
        .column_as(dbkit::func::right("abcdef", -2_i32), "right_negative")
        .column_as(dbkit::func::substring("abcdef", 1_i32, 3_i32), "substring_one")
        .column_as(dbkit::func::substring("abcdef", 0_i32, 3_i32), "substring_zero_start")
        .column_as(dbkit::func::substring("abcdef", -2_i32, 5_i32), "substring_negative_start")
        .column_as(dbkit::func::substring("abcdef", 2_i32, 0_i32), "substring_zero_count")
        .column_as(dbkit::func::substring("abcdef", 2_i32, 99_i32), "substring_long_count")
        .column_as(dbkit::func::repeat("ab", 0_i32), "repeat_zero")
        .column_as(dbkit::func::repeat("ab", 1_i32), "repeat_one")
        .column_as(dbkit::func::repeat("ab", -2_i32), "repeat_negative")
        .column_as(dbkit::func::pad_start("abcdef", 4_i32, "xy"), "pad_start_shorter")
        .column_as(dbkit::func::pad_start("abcdef", 6_i32, "xy"), "pad_start_equal")
        .column_as(dbkit::func::pad_start("abcdef", 9_i32, "xy"), "pad_start_longer")
        .column_as(dbkit::func::pad_start("ab", 5_i32, "."), "pad_start_single_fill")
        .column_as(dbkit::func::pad_start("abcdef", 9_i32, ""), "pad_start_empty_fill")
        .column_as(dbkit::func::pad_start("abcdef", -1_i32, "xy"), "pad_start_negative")
        .column_as(dbkit::func::pad_end("abcdef", 4_i32, "xy"), "pad_end_shorter")
        .column_as(dbkit::func::pad_end("abcdef", 6_i32, "xy"), "pad_end_equal")
        .column_as(dbkit::func::pad_end("abcdef", 9_i32, "xy"), "pad_end_longer")
        .column_as(dbkit::func::pad_end("ab", 5_i32, "."), "pad_end_single_fill")
        .column_as(dbkit::func::pad_end("abcdef", 9_i32, ""), "pad_end_empty_fill")
        .column_as(dbkit::func::pad_end("abcdef", -1_i32, "xy"), "pad_end_negative")
        .into_model()
        .one(&tx)
        .await?
        .expect("boundary result");

    assert_eq!(result.left_zero, "");
    assert_eq!(result.left_long, "abcdef");
    assert_eq!(result.left_negative, "abcd");
    assert_eq!(result.right_zero, "");
    assert_eq!(result.right_long, "abcdef");
    assert_eq!(result.right_negative, "cdef");
    assert_eq!(result.substring_one, "abc");
    assert_eq!(result.substring_zero_start, "ab");
    assert_eq!(result.substring_negative_start, "ab");
    assert_eq!(result.substring_zero_count, "");
    assert_eq!(result.substring_long_count, "bcdef");
    assert_eq!(result.repeat_zero, "");
    assert_eq!(result.repeat_one, "ab");
    assert_eq!(result.repeat_negative, "");
    assert_eq!(result.pad_start_shorter, "abcd");
    assert_eq!(result.pad_start_equal, "abcdef");
    assert_eq!(result.pad_start_longer, "xyxabcdef");
    assert_eq!(result.pad_start_single_fill, "...ab");
    assert_eq!(result.pad_start_empty_fill, "abcdef");
    assert_eq!(result.pad_start_negative, "");
    assert_eq!(result.pad_end_shorter, "abcd");
    assert_eq!(result.pad_end_equal, "abcdef");
    assert_eq!(result.pad_end_longer, "abcdefxyx");
    assert_eq!(result.pad_end_single_fill, "ab...");
    assert_eq!(result.pad_end_empty_fill, "abcdef");
    assert_eq!(result.pad_end_negative, "");

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct ComposedStringFunctionResult {
    extracted: Option<String>,
    padded: Option<String>,
    repeated: Option<String>,
}

#[tokio::test]
async fn string_extraction_and_sizing_compose_with_normalization_and_expression_counts() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "xxx", Some("  AbCd  ")).await?;

    let normalized = dbkit::func::lower(dbkit::func::trim(TextSample::body));
    let count = dbkit::func::char_length(TextSample::label);
    let result: ComposedStringFunctionResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::left(dbkit::func::trim(TextSample::body), count.clone()), "extracted")
        .column_as(
            dbkit::func::pad_end(
                dbkit::func::substring(normalized.clone(), 2_i32, count.clone()),
                8_i32,
                dbkit::func::lower("Q%_\\'"),
            ),
            "padded",
        )
        .column_as(
            dbkit::func::repeat(dbkit::func::right(normalized.clone(), count), 2_i32),
            "repeated",
        )
        .filter(dbkit::func::left(normalized, 2_i32).eq("ab"))
        .order_by(dbkit::Order::asc(dbkit::func::pad_start(TextSample::label, 5_i32, "0")))
        .into_model()
        .one(&tx)
        .await?
        .expect("composed string result");

    assert_eq!(result.extracted.as_deref(), Some("AbC"));
    assert_eq!(result.padded.as_deref(), Some("bcdq%_\\'"));
    assert_eq!(result.repeated.as_deref(), Some("bcdbcd"));

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct NullableStringResult {
    value: Option<String>,
}

#[tokio::test]
async fn substring_rejects_negative_count() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "negative", Some("abcdef")).await?;

    let result: Result<Vec<NullableStringResult>, dbkit::Error> = TextSample::query()
        .select_only()
        .column_as(dbkit::func::substring(TextSample::body, 1_i32, -1_i32), "value")
        .into_model()
        .all(&tx)
        .await;

    let error = result.expect_err("PostgreSQL must reject a negative substring count");
    assert!(
        error.to_string().contains("negative substring length not allowed"),
        "unexpected error: {error}"
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct TextCaseAndReverseResult {
    title_words: String,
    title_already_cased: String,
    title_punctuation: String,
    title_digits: String,
    title_whitespace: String,
    title_unicode_separator: String,
    title_empty: String,
    reverse_ascii: String,
    reverse_unicode: String,
    reverse_combining_sequence: String,
    reverse_empty: String,
}

#[tokio::test]
async fn title_case_and_reverse_follow_postgresql_character_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "character-semantics", None).await?;

    let result: TextCaseAndReverseResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::title_case("hELLO wORLD"), "title_words")
        .column_as(dbkit::func::title_case("Already Cased"), "title_already_cased")
        .column_as(dbkit::func::title_case("one-two_three.four"), "title_punctuation")
        .column_as(dbkit::func::title_case("abc123 42FOO"), "title_digits")
        .column_as(dbkit::func::title_case("\theLLo  wORLD\n"), "title_whitespace")
        .column_as(dbkit::func::title_case("hello—WORLD"), "title_unicode_separator")
        .column_as(dbkit::func::title_case(""), "title_empty")
        .column_as(dbkit::func::reverse("abcde"), "reverse_ascii")
        .column_as(dbkit::func::reverse("é🦀界"), "reverse_unicode")
        .column_as(dbkit::func::reverse("a\u{301}b"), "reverse_combining_sequence")
        .column_as(dbkit::func::reverse(""), "reverse_empty")
        .into_model()
        .one(&tx)
        .await?
        .expect("character transformation result");

    assert_eq!(result.title_words, "Hello World");
    assert_eq!(result.title_already_cased, "Already Cased");
    assert_eq!(result.title_punctuation, "One-Two_Three.Four");
    assert_eq!(result.title_digits, "Abc123 42foo");
    assert_eq!(result.title_whitespace, "\tHello  World\n");
    assert_eq!(result.title_unicode_separator, "Hello—World");
    assert_eq!(result.title_empty, "");
    assert_eq!(result.reverse_ascii, "edcba");
    assert_eq!(result.reverse_unicode, "界🦀é");
    assert_eq!(result.reverse_combining_sequence, "b\u{301}a");
    assert_eq!(result.reverse_empty, "");

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct ReplaceAndTranslateResult {
    replace_all: String,
    replace_non_overlapping: String,
    replace_absent: String,
    replace_empty_source: String,
    replace_empty_from: String,
    replace_empty_to: String,
    replace_case_sensitive: String,
    replace_unicode: String,
    translate_positional: String,
    translate_deletion: String,
    translate_extra_to: String,
    translate_repeated_source: String,
    translate_unicode: String,
    translate_empty_from: String,
    translate_empty_to: String,
    translate_case_sensitive: String,
}

#[tokio::test]
async fn replace_and_translate_cover_exact_mapping_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "mapping-semantics", None).await?;

    let result: ReplaceAndTranslateResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::replace("abcabc", "bc", "X"), "replace_all")
        .column_as(dbkit::func::replace("aaaaa", "aa", "b"), "replace_non_overlapping")
        .column_as(dbkit::func::replace("abc", "z", "X"), "replace_absent")
        .column_as(dbkit::func::replace("", "a", "X"), "replace_empty_source")
        .column_as(dbkit::func::replace("abc", "", "X"), "replace_empty_from")
        .column_as(dbkit::func::replace("banana", "an", ""), "replace_empty_to")
        .column_as(dbkit::func::replace("AaA", "a", "x"), "replace_case_sensitive")
        .column_as(dbkit::func::replace("é🦀é", "é", "界"), "replace_unicode")
        .column_as(dbkit::func::translate_chars("12345", "143", "ax"), "translate_positional")
        .column_as(dbkit::func::translate_chars("12345", "143", "a"), "translate_deletion")
        .column_as(dbkit::func::translate_chars("abc", "ab", "XYZ"), "translate_extra_to")
        .column_as(dbkit::func::translate_chars("banana", "an", "12"), "translate_repeated_source")
        .column_as(dbkit::func::translate_chars("é🦀界é", "é界", "ab"), "translate_unicode")
        .column_as(dbkit::func::translate_chars("abc", "", "xyz"), "translate_empty_from")
        .column_as(dbkit::func::translate_chars("banana", "an", ""), "translate_empty_to")
        .column_as(dbkit::func::translate_chars("Aa", "a", "x"), "translate_case_sensitive")
        .into_model()
        .one(&tx)
        .await?
        .expect("replace and translate result");

    assert_eq!(result.replace_all, "aXaX");
    assert_eq!(result.replace_non_overlapping, "bba");
    assert_eq!(result.replace_absent, "abc");
    assert_eq!(result.replace_empty_source, "");
    assert_eq!(result.replace_empty_from, "abc");
    assert_eq!(result.replace_empty_to, "ba");
    assert_eq!(result.replace_case_sensitive, "AxA");
    assert_eq!(result.replace_unicode, "界🦀界");
    assert_eq!(result.translate_positional, "a2x5");
    assert_eq!(result.translate_deletion, "a25");
    assert_eq!(result.translate_extra_to, "XYc");
    assert_eq!(result.translate_repeated_source, "b12121");
    assert_eq!(result.translate_unicode, "a🦀ba");
    assert_eq!(result.translate_empty_from, "abc");
    assert_eq!(result.translate_empty_to, "b");
    assert_eq!(result.translate_case_sensitive, "Ax");

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct NullableTextTransformationResult {
    title_case: Option<String>,
    reverse: Option<String>,
    replace_input: Option<String>,
    replace_from: Option<String>,
    replace_to: Option<String>,
    range_input: Option<String>,
    range_replacement: Option<String>,
    translate_input: Option<String>,
    translate_from: Option<String>,
    translate_to: Option<String>,
}

#[tokio::test]
async fn text_transformations_propagate_null_from_every_string_argument() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    let row = seed_text_sample(&tx, "nullable", None).await?;

    let result: NullableTextTransformationResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::title_case(TextSample::body), "title_case")
        .column_as(dbkit::func::reverse(TextSample::body), "reverse")
        .column_as(dbkit::func::replace(TextSample::body, "a", "b"), "replace_input")
        .column_as(dbkit::func::replace(TextSample::label, TextSample::body, "b"), "replace_from")
        .column_as(dbkit::func::replace(TextSample::label, "a", TextSample::body), "replace_to")
        .column_as(dbkit::func::replace_range(TextSample::body, "x", 1_i32, 1_i32), "range_input")
        .column_as(
            dbkit::func::replace_range(TextSample::label, TextSample::body, 1_i32, 1_i32),
            "range_replacement",
        )
        .column_as(dbkit::func::translate_chars(TextSample::body, "a", "b"), "translate_input")
        .column_as(
            dbkit::func::translate_chars(TextSample::label, TextSample::body, "b"),
            "translate_from",
        )
        .column_as(
            dbkit::func::translate_chars(TextSample::label, "a", TextSample::body),
            "translate_to",
        )
        .filter(TextSample::id.eq(row.id))
        .into_model()
        .one(&tx)
        .await?
        .expect("nullable transformation result");

    assert_eq!(
        (
            result.title_case,
            result.reverse,
            result.replace_input,
            result.replace_from,
            result.replace_to,
            result.range_input,
            result.range_replacement,
            result.translate_input,
            result.translate_from,
            result.translate_to,
        ),
        (None, None, None, None, None, None, None, None, None, None)
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct ComposedTextTransformationResult {
    label: String,
    transformed: Option<String>,
}

#[tokio::test]
async fn nested_text_transformations_filter_real_postgresql_rows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_text_sample(&tx, "matched", Some("  hELLO-old  ")).await?;
    seed_text_sample(&tx, "other", Some("  HELLO-current  ")).await?;
    seed_text_sample(&tx, "missing", None).await?;

    let transformed = dbkit::func::reverse(dbkit::func::translate_chars(
        dbkit::func::replace(
            dbkit::func::title_case(dbkit::func::lower(dbkit::func::trim(TextSample::body))),
            dbkit::func::title_case("OLD"),
            dbkit::func::reverse("wen"),
        ),
        dbkit::func::lower("E"),
        dbkit::func::upper("x"),
    ));
    let result: ComposedTextTransformationResult = TextSample::query()
        .select_only()
        .column(TextSample::label)
        .column_as(transformed.clone(), "transformed")
        .filter(transformed.eq("wXn-ollXH"))
        .into_model()
        .one(&tx)
        .await?
        .expect("nested transformation match");

    assert_eq!(result.label, "matched");
    assert_eq!(result.transformed.as_deref(), Some("wXn-ollXH"));

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct ReplaceRangeBoundaryResult {
    shorter: String,
    equal: String,
    longer: String,
    start_one: String,
    start_beyond_end: String,
    zero_count: String,
    negative_count: String,
    oversized_count: String,
    empty_source: String,
    empty_replacement: String,
    unicode: String,
}

#[tokio::test]
async fn replace_range_follows_postgresql_overlay_boundaries() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "overlay-boundaries", None).await?;

    let result: ReplaceRangeBoundaryResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::replace_range("abcdef", "X", 3_i32, 2_i32), "shorter")
        .column_as(dbkit::func::replace_range("abcdef", "XY", 3_i32, 2_i32), "equal")
        .column_as(dbkit::func::replace_range("abcdef", "WXYZ", 3_i32, 2_i32), "longer")
        .column_as(dbkit::func::replace_range("abcdef", "X", 1_i32, 2_i32), "start_one")
        .column_as(dbkit::func::replace_range("abcdef", "X", 8_i32, 2_i32), "start_beyond_end")
        .column_as(dbkit::func::replace_range("abcdef", "X", 3_i32, 0_i32), "zero_count")
        .column_as(dbkit::func::replace_range("abcdef", "X", 3_i32, -1_i32), "negative_count")
        .column_as(dbkit::func::replace_range("abcdef", "X", 3_i32, 99_i32), "oversized_count")
        .column_as(dbkit::func::replace_range("", "X", 1_i32, 0_i32), "empty_source")
        .column_as(dbkit::func::replace_range("abcdef", "", 3_i32, 2_i32), "empty_replacement")
        .column_as(dbkit::func::replace_range("a🦀界b", "é", 2_i32, 2_i32), "unicode")
        .into_model()
        .one(&tx)
        .await?
        .expect("replace range boundary result");

    assert_eq!(result.shorter, "abXef");
    assert_eq!(result.equal, "abXYef");
    assert_eq!(result.longer, "abWXYZef");
    assert_eq!(result.start_one, "Xcdef");
    assert_eq!(result.start_beyond_end, "abcdefX");
    assert_eq!(result.zero_count, "abXcdef");
    assert_eq!(result.negative_count, "abXbcdef");
    assert_eq!(result.oversized_count, "abX");
    assert_eq!(result.empty_source, "X");
    assert_eq!(result.empty_replacement, "abef");
    assert_eq!(result.unicode, "aéb");

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct BoundTextTransformationResult {
    replaced: String,
    ranged: String,
    translated: String,
}

#[tokio::test]
async fn text_transformation_metacharacters_remain_bound_values() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "bind-safety", None).await?;

    let unsafe_text = "'%_\\[]().*+";
    let result: BoundTextTransformationResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::replace(unsafe_text, "%_\\", "safe"), "replaced")
        .column_as(dbkit::func::replace_range(unsafe_text, "safe", 2_i32, 3_i32), "ranged")
        .column_as(dbkit::func::translate_chars(unsafe_text, "'\\", "QB"), "translated")
        .into_model()
        .one(&tx)
        .await?
        .expect("bound transformation result");

    assert_eq!(result.replaced, "'safe[]().*+");
    assert_eq!(result.ranged, "'safe[]().*+");
    assert_eq!(result.translated, "Q%_B[]().*+");

    Ok(())
}

#[tokio::test]
async fn replace_range_rejects_zero_start() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "invalid-overlay-start", None).await?;

    let result: Result<Vec<NullableStringResult>, dbkit::Error> = TextSample::query()
        .select_only()
        .column_as(dbkit::func::replace_range("abcdef", "X", 0_i32, 2_i32), "value")
        .into_model()
        .all(&tx)
        .await;

    let error = result.expect_err("PostgreSQL must reject a non-positive overlay start");
    assert!(
        error.to_string().contains("negative substring length not allowed"),
        "unexpected error: {error}"
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct StringCompositionResult {
    joined: String,
    separated: String,
    empty_separator: String,
    nullable_separator: Option<String>,
    single: String,
    single_separated: String,
    no_values: String,
    no_separated_values: String,
    no_values_nullable_separator: Option<String>,
}

#[tokio::test]
async fn concat_functions_follow_postgres_null_empty_and_unicode_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    for (email, backup_email, region) in [
        (Some("é"), None, Some("界")),
        (None, None, None),
        (Some(""), Some(""), Some("")),
        (Some("A"), Some("B"), None),
    ] {
        FuncRow::insert(FuncRowInsert {
            email: email.map(str::to_string),
            backup_email: backup_email.map(str::to_string),
            region: region.map(str::to_string),
            starts_at: NaiveDateTime::default(),
        })
        .execute(&tx)
        .await?;
    }

    let no_values: [dbkit::Expr<String>; 0] = [];
    let no_separated_values: [dbkit::Expr<String>; 0] = [];
    let no_nullable_separator_values: [dbkit::Expr<String>; 0] = [];
    let rows: Vec<StringCompositionResult> = FuncRow::query()
        .select_only()
        .column_as(
            dbkit::func::concat([FuncRow::email, FuncRow::backup_email, FuncRow::region]),
            "joined",
        )
        .column_as(
            dbkit::func::concat_with_separator("|", [FuncRow::email, FuncRow::backup_email, FuncRow::region]),
            "separated",
        )
        .column_as(
            dbkit::func::concat_with_separator("", [FuncRow::email, FuncRow::backup_email, FuncRow::region]),
            "empty_separator",
        )
        .column_as(
            dbkit::func::concat_with_separator(FuncRow::region, [FuncRow::email, FuncRow::backup_email]),
            "nullable_separator",
        )
        .column_as(dbkit::func::concat([FuncRow::email]), "single")
        .column_as(dbkit::func::concat_with_separator("|", [FuncRow::email]), "single_separated")
        .column_as(dbkit::func::concat(no_values), "no_values")
        .column_as(dbkit::func::concat_with_separator("|", no_separated_values), "no_separated_values")
        .column_as(
            dbkit::func::concat_with_separator(FuncRow::region, no_nullable_separator_values),
            "no_values_nullable_separator",
        )
        .order_by(dbkit::Order::asc(FuncRow::id))
        .into_model()
        .all(&tx)
        .await?;

    let values: Vec<_> = rows
        .into_iter()
        .map(|row| {
            (
                row.joined,
                row.separated,
                row.empty_separator,
                row.nullable_separator,
                row.single,
                row.single_separated,
                row.no_values,
                row.no_separated_values,
                row.no_values_nullable_separator,
            )
        })
        .collect();
    assert_eq!(
        values,
        vec![
            (
                "é界".to_string(),
                "é|界".to_string(),
                "é界".to_string(),
                Some("é".to_string()),
                "é".to_string(),
                "é".to_string(),
                String::new(),
                String::new(),
                Some(String::new()),
            ),
            (
                String::new(),
                String::new(),
                String::new(),
                None,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                None,
            ),
            (
                String::new(),
                "||".to_string(),
                String::new(),
                Some(String::new()),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                Some(String::new()),
            ),
            (
                "AB".to_string(),
                "A|B".to_string(),
                "AB".to_string(),
                None,
                "A".to_string(),
                "A".to_string(),
                String::new(),
                String::new(),
                None,
            ),
        ]
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug, PartialEq, Eq)]
struct MixedModelColumnConcatResult {
    joined: String,
    separated: String,
    nullable_separator: Option<String>,
    dynamic_joined: String,
    dynamic_separated: String,
    empty_joined: String,
    empty_separated: String,
}

#[tokio::test]
async fn concat_macros_support_required_nullable_dynamic_and_empty_values() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_text_sample(&tx, "required", Some("optional")).await?;
    seed_text_sample(&tx, "required", None).await?;

    let dynamic_values = vec![TextSample::label.into_expr(), dbkit::func::lower("TAIL")];
    let dynamic_separated_values = vec![TextSample::label.into_expr(), dbkit::func::lower("TAIL")];
    let rows: Vec<MixedModelColumnConcatResult> = TextSample::query()
        .select_only()
        .column_as(dbkit::func::concat!([TextSample::label, TextSample::body]), "joined")
        .column_as(
            dbkit::func::concat_with_separator!("|", [TextSample::label, TextSample::body]),
            "separated",
        )
        .column_as(
            dbkit::func::concat_with_separator!(TextSample::body, [TextSample::label, "suffix"]),
            "nullable_separator",
        )
        .column_as(dbkit::func::concat!(dynamic_values), "dynamic_joined")
        .column_as(
            dbkit::func::concat_with_separator!("|", dynamic_separated_values),
            "dynamic_separated",
        )
        .column_as(dbkit::func::concat!([]), "empty_joined")
        .column_as(dbkit::func::concat_with_separator!("|", []), "empty_separated")
        .order_by(dbkit::Order::asc(TextSample::id))
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(
        rows,
        vec![
            MixedModelColumnConcatResult {
                joined: "requiredoptional".to_string(),
                separated: "required|optional".to_string(),
                nullable_separator: Some("requiredoptionalsuffix".to_string()),
                dynamic_joined: "requiredtail".to_string(),
                dynamic_separated: "required|tail".to_string(),
                empty_joined: String::new(),
                empty_separated: String::new(),
            },
            MixedModelColumnConcatResult {
                joined: "required".to_string(),
                separated: "required".to_string(),
                nullable_separator: None,
                dynamic_joined: "requiredtail".to_string(),
                dynamic_separated: "required|tail".to_string(),
                empty_joined: String::new(),
                empty_separated: String::new(),
            },
        ]
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct SplitResult {
    normal: Vec<String>,
    repeated: Vec<String>,
    absent: Vec<String>,
    empty_source: Vec<String>,
    empty_delimiter: Vec<String>,
    unicode: Vec<String>,
    null_delimiter: Vec<String>,
    null_source: Option<Vec<String>>,
}

#[tokio::test]
async fn split_follows_postgres_array_and_null_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "boundary", None).await?;

    let result: SplitResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::split("alpha::beta::gamma", "::"), "normal")
        .column_as(dbkit::func::split("a::::c::", "::"), "repeated")
        .column_as(dbkit::func::split("abc", ","), "absent")
        .column_as(dbkit::func::split("", ","), "empty_source")
        .column_as(dbkit::func::split("abc", ""), "empty_delimiter")
        .column_as(dbkit::func::split("é🙂界🙂ß", "🙂"), "unicode")
        .column_as(dbkit::func::split("é🙂", TextSample::body), "null_delimiter")
        .column_as(dbkit::func::split(TextSample::body, ","), "null_source")
        .into_model()
        .one(&tx)
        .await?
        .expect("split result");

    assert_eq!(result.normal, ["alpha", "beta", "gamma"]);
    assert_eq!(result.repeated, ["a", "", "c", ""]);
    assert_eq!(result.absent, ["abc"]);
    assert!(result.empty_source.is_empty());
    assert_eq!(result.empty_delimiter, ["abc"]);
    assert_eq!(result.unicode, ["é", "界", "ß"]);
    assert_eq!(result.null_delimiter, ["é", "🙂"]);
    assert_eq!(result.null_source, None);

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct SplitPartResult {
    normal: String,
    negative: String,
    out_of_range: String,
    empty_delimiter_first: String,
    empty_delimiter_second: String,
    repeated_delimiter: String,
    leading_empty: String,
    trailing_empty: String,
    unicode: String,
    expression_index: String,
    null_source: Option<String>,
    null_delimiter: Option<String>,
}

#[tokio::test]
async fn split_part_follows_postgres_index_field_and_null_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "boundary", None).await?;

    let result: SplitPartResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::split_part("alpha::beta::gamma", "::", 2_i32), "normal")
        .column_as(dbkit::func::split_part("a,b,c,d", ",", -2_i32), "negative")
        .column_as(dbkit::func::split_part("a,b", ",", 3_i32), "out_of_range")
        .column_as(dbkit::func::split_part("abc", "", 1_i32), "empty_delimiter_first")
        .column_as(dbkit::func::split_part("abc", "", 2_i32), "empty_delimiter_second")
        .column_as(dbkit::func::split_part("a,,c", ",", 2_i32), "repeated_delimiter")
        .column_as(dbkit::func::split_part(",a", ",", 1_i32), "leading_empty")
        .column_as(dbkit::func::split_part("a,", ",", -1_i32), "trailing_empty")
        .column_as(dbkit::func::split_part("é🙂界🙂ß", "🙂", -2_i32), "unicode")
        .column_as(
            dbkit::func::split_part("é🙂界", "🙂", dbkit::func::char_length("x")),
            "expression_index",
        )
        .column_as(dbkit::func::split_part(TextSample::body, ",", 1_i32), "null_source")
        .column_as(dbkit::func::split_part("abc", TextSample::body, 1_i32), "null_delimiter")
        .into_model()
        .one(&tx)
        .await?
        .expect("split part result");

    assert_eq!(result.normal, "beta");
    assert_eq!(result.negative, "c");
    assert_eq!(result.out_of_range, "");
    assert_eq!(result.empty_delimiter_first, "abc");
    assert_eq!(result.empty_delimiter_second, "");
    assert_eq!(result.repeated_delimiter, "");
    assert_eq!(result.leading_empty, "");
    assert_eq!(result.trailing_empty, "");
    assert_eq!(result.unicode, "界");
    assert_eq!(result.expression_index, "é");
    assert_eq!(result.null_source, None);
    assert_eq!(result.null_delimiter, None);

    Ok(())
}

#[tokio::test]
async fn split_part_rejects_zero_index() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "zero", None).await?;

    let result: Result<Vec<NullableStringResult>, dbkit::Error> = TextSample::query()
        .select_only()
        .column_as(dbkit::func::split_part(TextSample::label, ",", 0_i32), "value")
        .into_model()
        .all(&tx)
        .await;

    let error = result.expect_err("PostgreSQL must reject split_part index zero");
    assert!(
        error.to_string().contains("field position must not be zero"),
        "unexpected error: {error}"
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct BoundCompositionAndSplitResult {
    joined: String,
    separated: String,
    parts: Vec<String>,
    second_part: String,
}

#[tokio::test]
async fn composition_and_split_arguments_remain_bound_values() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "bound", None).await?;

    let unsafe_text = "'%_\\.*+?[](){}^$|";
    let source = format!("left{unsafe_text}right");
    let result: BoundCompositionAndSplitResult = TextSample::query()
        .select_only()
        .column_as(dbkit::func::concat([unsafe_text.into_expr(), dbkit::func::lower("TAIL")]), "joined")
        .column_as(
            dbkit::func::concat_with_separator(unsafe_text, ["left".into_expr(), "right".into_expr()]),
            "separated",
        )
        .column_as(dbkit::func::split(source.clone(), unsafe_text), "parts")
        .column_as(dbkit::func::split_part(source, unsafe_text, 2_i32), "second_part")
        .into_model()
        .one(&tx)
        .await?
        .expect("bound composition and split result");

    assert_eq!(result.joined, format!("{unsafe_text}tail"));
    assert_eq!(result.separated, format!("left{unsafe_text}right"));
    assert_eq!(result.parts, ["left", "right"]);
    assert_eq!(result.second_part, "right");

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct RegexReplaceBoundaryResult {
    first_match: String,
    global: String,
    no_match: String,
    empty_source: String,
    empty_pattern_first: String,
    empty_pattern_global: String,
    empty_replacement: String,
    capture_groups: String,
    whole_match: String,
    literal_backslash: String,
    literal_dollar_quote: String,
    case_sensitive: String,
    case_insensitive: String,
    zero_width: String,
    unicode: String,
}

#[tokio::test]
async fn regex_replace_follows_postgresql_match_replacement_and_flag_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "regex-replace", None).await?;

    let result: RegexReplaceBoundaryResult = TextSample::query()
        .select_only()
        .column_as(
            dbkit::func::regex_replace("abcabc", "a.", "X", RegexReplaceFlags::empty()),
            "first_match",
        )
        .column_as(dbkit::func::regex_replace("abcabc", "a.", "X", RegexReplaceFlags::GLOBAL), "global")
        .column_as(dbkit::func::regex_replace("abc", "z+", "X", RegexReplaceFlags::empty()), "no_match")
        .column_as(dbkit::func::regex_replace("", "a", "X", RegexReplaceFlags::empty()), "empty_source")
        .column_as(
            dbkit::func::regex_replace("abc", "", "X", RegexReplaceFlags::empty()),
            "empty_pattern_first",
        )
        .column_as(
            dbkit::func::regex_replace("abc", "", "X", RegexReplaceFlags::GLOBAL),
            "empty_pattern_global",
        )
        .column_as(
            dbkit::func::regex_replace("a1b22c", r"\d+", "", RegexReplaceFlags::GLOBAL),
            "empty_replacement",
        )
        .column_as(
            dbkit::func::regex_replace("Ada Lovelace", "([A-Za-z]+) ([A-Za-z]+)", r"\2, \1", RegexReplaceFlags::empty()),
            "capture_groups",
        )
        .column_as(
            dbkit::func::regex_replace("abc", "b", r"[\&]", RegexReplaceFlags::empty()),
            "whole_match",
        )
        .column_as(
            dbkit::func::regex_replace("abc", "b", r"\\", RegexReplaceFlags::empty()),
            "literal_backslash",
        )
        .column_as(
            dbkit::func::regex_replace("abc", "b", "$1'&", RegexReplaceFlags::empty()),
            "literal_dollar_quote",
        )
        .column_as(
            dbkit::func::regex_replace("AaA", "a", "x", RegexReplaceFlags::GLOBAL),
            "case_sensitive",
        )
        .column_as(
            dbkit::func::regex_replace("AaA", "a", "x", RegexReplaceFlags::GLOBAL | RegexReplaceFlags::CASE_INSENSITIVE),
            "case_insensitive",
        )
        .column_as(
            dbkit::func::regex_replace("ab", "(^|$)", "_", RegexReplaceFlags::GLOBAL),
            "zero_width",
        )
        .column_as(
            dbkit::func::regex_replace("é🙂é", "[é🙂]", "X", RegexReplaceFlags::GLOBAL),
            "unicode",
        )
        .into_model()
        .one(&tx)
        .await?
        .expect("regex replacement result");

    assert_eq!(result.first_match, "Xcabc");
    assert_eq!(result.global, "XcXc");
    assert_eq!(result.no_match, "abc");
    assert_eq!(result.empty_source, "");
    assert_eq!(result.empty_pattern_first, "Xabc");
    assert_eq!(result.empty_pattern_global, "XaXbXcX");
    assert_eq!(result.empty_replacement, "abc");
    assert_eq!(result.capture_groups, "Lovelace, Ada");
    assert_eq!(result.whole_match, "a[b]c");
    assert_eq!(result.literal_backslash, "a\\c");
    assert_eq!(result.literal_dollar_quote, "a$1'&c");
    assert_eq!(result.case_sensitive, "AxA");
    assert_eq!(result.case_insensitive, "xxx");
    assert_eq!(result.zero_width, "_ab_");
    assert_eq!(result.unicode, "XXX");

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct RegexSplitBoundaryResult {
    normal: Vec<String>,
    repeated: Vec<String>,
    adjacent: Vec<String>,
    edges: Vec<String>,
    no_match: Vec<String>,
    empty_source: Vec<String>,
    empty_pattern: Vec<String>,
    zero_width_star: Vec<String>,
    zero_width_lookahead: Vec<String>,
    zero_width_boundaries: Vec<String>,
    unicode: Vec<String>,
    case_sensitive: Vec<String>,
    case_insensitive: Vec<String>,
}

#[tokio::test]
async fn regex_split_returns_exact_text_arrays_and_suppresses_special_zero_width_matches() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "regex-split", None).await?;

    let result: RegexSplitBoundaryResult = TextSample::query()
        .select_only()
        .column_as(
            dbkit::func::regex_split("one, two;three", "[,;][ ]*", RegexSplitFlags::empty()),
            "normal",
        )
        .column_as(dbkit::func::regex_split("a,,b;;;c", "[,;]+", RegexSplitFlags::empty()), "repeated")
        .column_as(dbkit::func::regex_split("a,,b", ",", RegexSplitFlags::empty()), "adjacent")
        .column_as(dbkit::func::regex_split(",a,", ",", RegexSplitFlags::empty()), "edges")
        .column_as(dbkit::func::regex_split("abc", ",", RegexSplitFlags::empty()), "no_match")
        .column_as(dbkit::func::regex_split("", ",", RegexSplitFlags::empty()), "empty_source")
        .column_as(dbkit::func::regex_split("abc", "", RegexSplitFlags::empty()), "empty_pattern")
        .column_as(
            dbkit::func::regex_split("the quick", "[ ]*", RegexSplitFlags::empty()),
            "zero_width_star",
        )
        .column_as(
            dbkit::func::regex_split("abc", "(?=b)", RegexSplitFlags::empty()),
            "zero_width_lookahead",
        )
        .column_as(
            dbkit::func::regex_split("abc", "(^|$)", RegexSplitFlags::empty()),
            "zero_width_boundaries",
        )
        .column_as(
            dbkit::func::regex_split("été🙂hiver", "[é🙂]+", RegexSplitFlags::empty()),
            "unicode",
        )
        .column_as(dbkit::func::regex_split("aXbxc", "x", RegexSplitFlags::empty()), "case_sensitive")
        .column_as(
            dbkit::func::regex_split("aXbxc", "x", RegexSplitFlags::CASE_INSENSITIVE),
            "case_insensitive",
        )
        .into_model()
        .one(&tx)
        .await?
        .expect("regex split result");

    assert_eq!(result.normal, ["one", "two", "three"]);
    assert_eq!(result.repeated, ["a", "b", "c"]);
    assert_eq!(result.adjacent, ["a", "", "b"]);
    assert_eq!(result.edges, ["", "a", ""]);
    assert_eq!(result.no_match, ["abc"]);
    assert_eq!(result.empty_source, [""]);
    assert_eq!(result.empty_pattern, ["a", "b", "c"]);
    assert_eq!(result.zero_width_star, ["t", "h", "e", "q", "u", "i", "c", "k"]);
    assert_eq!(result.zero_width_lookahead, ["a", "bc"]);
    assert_eq!(result.zero_width_boundaries, ["abc"]);
    assert_eq!(result.unicode, ["", "t", "hiver"]);
    assert_eq!(result.case_sensitive, ["aXb", "c"]);
    assert_eq!(result.case_insensitive, ["a", "b", "c"]);

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct NullableRegexTransformResult {
    replace_source: Option<String>,
    replace_pattern: Option<String>,
    replace_replacement: Option<String>,
    split_source: Option<Vec<String>>,
    split_pattern: Option<Vec<String>>,
}

#[tokio::test]
async fn regex_transforms_propagate_null_from_every_nullable_text_argument() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    let row = seed_text_sample(&tx, "abc", None).await?;

    let result: NullableRegexTransformResult = TextSample::query()
        .select_only()
        .column_as(
            dbkit::func::regex_replace(TextSample::body, "a", "x", RegexReplaceFlags::empty()),
            "replace_source",
        )
        .column_as(
            dbkit::func::regex_replace(TextSample::label, TextSample::body, "x", RegexReplaceFlags::empty()),
            "replace_pattern",
        )
        .column_as(
            dbkit::func::regex_replace(TextSample::label, "a", TextSample::body, RegexReplaceFlags::empty()),
            "replace_replacement",
        )
        .column_as(
            dbkit::func::regex_split(TextSample::body, ",", RegexSplitFlags::empty()),
            "split_source",
        )
        .column_as(
            dbkit::func::regex_split(TextSample::label, TextSample::body, RegexSplitFlags::empty()),
            "split_pattern",
        )
        .filter(TextSample::id.eq(row.id))
        .into_model()
        .one(&tx)
        .await?
        .expect("nullable regex result");

    assert_eq!(result.replace_source, None);
    assert_eq!(result.replace_pattern, None);
    assert_eq!(result.replace_replacement, None);
    assert_eq!(result.split_source, None);
    assert_eq!(result.split_pattern, None);

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct BoundRegexTransformResult {
    replaced: String,
    normalized: Option<String>,
    parts: Option<Vec<String>>,
}

#[tokio::test]
async fn regex_transform_arguments_remain_bound_and_compose_with_existing_string_functions() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let source = "a'%_\\b";
    let pattern = r"(['%_\\])";
    let replacement = r"<\1>$1'\\";
    let row = seed_text_sample(&tx, source, Some("  AA  BB  ")).await?;
    let normalized = dbkit::func::regex_replace(
        dbkit::func::lower(dbkit::func::trim(dbkit::func::substring(TextSample::body, 1_i32, 99_i32))),
        r"\s+",
        "-",
        RegexReplaceFlags::GLOBAL,
    );

    let result: BoundRegexTransformResult = TextSample::query()
        .select_only()
        .column_as(
            dbkit::func::regex_replace(TextSample::label, pattern, replacement, RegexReplaceFlags::GLOBAL),
            "replaced",
        )
        .column_as(normalized.clone(), "normalized")
        .column_as(dbkit::func::regex_split(normalized.clone(), "-", RegexSplitFlags::empty()), "parts")
        .filter(TextSample::id.eq(row.id))
        .filter(normalized.eq("aa-bb"))
        .order_by(dbkit::Order::asc(dbkit::func::regex_split(
            TextSample::label,
            "[%_]+",
            RegexSplitFlags::empty(),
        )))
        .into_model()
        .one(&tx)
        .await?
        .expect("bound regex result");

    assert_eq!(result.replaced, "a<'>$1'\\<%>$1'\\<_>$1'\\<\\>$1'\\b");
    assert_eq!(result.normalized.as_deref(), Some("aa-bb"));
    assert_eq!(result.parts, Some(vec!["aa".to_string(), "bb".to_string()]));

    Ok(())
}

#[tokio::test]
async fn regex_replace_reports_invalid_patterns() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "invalid-replace", None).await?;

    let result: Result<Vec<NullableStringResult>, dbkit::Error> = TextSample::query()
        .select_only()
        .column_as(
            dbkit::func::regex_replace(TextSample::label, "[", "x", RegexReplaceFlags::empty()),
            "value",
        )
        .into_model()
        .all(&tx)
        .await;

    let error = result.expect_err("PostgreSQL must reject an invalid replacement pattern");
    assert!(
        error.to_string().contains("invalid regular expression"),
        "unexpected error: {error}"
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct RegexSplitResult {
    value: Vec<String>,
}

#[tokio::test]
async fn regex_split_reports_invalid_patterns() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_text_sample(&tx, "invalid-split", None).await?;

    let result: Result<Vec<RegexSplitResult>, dbkit::Error> = TextSample::query()
        .select_only()
        .column_as(dbkit::func::regex_split(TextSample::label, "(", RegexSplitFlags::empty()), "value")
        .into_model()
        .all(&tx)
        .await;

    let error = result.expect_err("PostgreSQL must reject an invalid split pattern");
    assert!(
        error.to_string().contains("invalid regular expression"),
        "unexpected error: {error}"
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct RegionAgg {
    region: String,
    total: dbkit::sqlx::types::BigDecimal,
    count: i64,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct BucketAgg {
    bucket: NaiveDateTime,
    total: dbkit::sqlx::types::BigDecimal,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct SaleExtremaAgg {
    region: String,
    first_sale_at: NaiveDateTime,
    last_sale_at: NaiveDateTime,
    min_amount: i64,
    max_amount: i64,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct EmptySaleExtremaAgg {
    first_sale_at: Option<NaiveDateTime>,
    last_sale_at: Option<NaiveDateTime>,
    min_amount: Option<i64>,
    max_amount: Option<i64>,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct NullableNoteExtremaAgg {
    min_note: Option<String>,
    max_note: Option<String>,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct UserTodoAgg {
    name: String,
    todo_count: i64,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct FilteredSaleAgg {
    active_sales: i64,
    us_sales: i64,
    large_us_sales: i64,
    missing_sales: i64,
    oldest_large_us_sale_at: Option<NaiveDateTime>,
    oldest_missing_sale_at: Option<NaiveDateTime>,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct EmptyFilteredSaleAgg {
    matching_sales: i64,
    oldest_matching_sale_at: Option<NaiveDateTime>,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct FilteredNullableAgg {
    all_rows: i64,
    null_rows: i64,
    non_null_notes: i64,
    first_null_note: Option<String>,
}

#[tokio::test]
async fn filtered_aggregates_roundtrip_without_group_by() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day1 = NaiveDate::from_ymd_opt(2024, 2, 1).expect("day");
    let day2 = NaiveDate::from_ymd_opt(2024, 2, 2).expect("day");
    let oldest_large_us_sale_at = NaiveDateTime::new(day1, NaiveTime::from_hms_opt(12, 0, 0).expect("time"));
    Sale::insert_many(vec![
        SaleInsert {
            region: "us".to_string(),
            amount: 40,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(10, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "us".to_string(),
            amount: 70,
            created_at: oldest_large_us_sale_at,
        },
        SaleInsert {
            region: "eu".to_string(),
            amount: 30,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(14, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "apac".to_string(),
            amount: 200,
            created_at: NaiveDateTime::new(day2, NaiveTime::from_hms_opt(9, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "us".to_string(),
            amount: 0,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(8, 0, 0).expect("time")),
        },
    ])
    .execute(&tx)
    .await?;

    let large_us_sale = Sale::region.eq("us").and(Sale::amount.ge(50_i64));
    let aggregate: FilteredSaleAgg = Sale::query()
        .select_only()
        .column_as(dbkit::func::count(Sale::id), "active_sales")
        .column_as(dbkit::func::count(Sale::id).filter(Sale::region.eq("us")), "us_sales")
        .column_as(dbkit::func::count(Sale::id).filter(large_us_sale.clone()), "large_us_sales")
        .column_as(dbkit::func::count(Sale::id).filter(Sale::region.eq("missing")), "missing_sales")
        .column_as(dbkit::func::min(Sale::created_at).filter(large_us_sale), "oldest_large_us_sale_at")
        .column_as(
            dbkit::func::min(Sale::created_at).filter(Sale::region.eq("missing")),
            "oldest_missing_sale_at",
        )
        .filter(Sale::amount.gt(0_i64))
        .into_model()
        .one(&tx)
        .await?
        .expect("aggregate without GROUP BY returns one row");

    assert_eq!(aggregate.active_sales, 4);
    assert_eq!(aggregate.us_sales, 2);
    assert_eq!(aggregate.large_us_sales, 1);
    assert_eq!(aggregate.missing_sales, 0);
    assert_eq!(aggregate.oldest_large_us_sale_at, Some(oldest_large_us_sale_at));
    assert_eq!(aggregate.oldest_missing_sale_at, None);

    Ok(())
}

#[tokio::test]
async fn filtered_aggregates_handle_empty_and_nullable_inputs() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let empty: EmptyFilteredSaleAgg = Sale::query()
        .select_only()
        .column_as(dbkit::func::count(Sale::id).filter(Sale::region.eq("us")), "matching_sales")
        .column_as(
            dbkit::func::min(Sale::created_at).filter(Sale::region.eq("us")),
            "oldest_matching_sale_at",
        )
        .into_model()
        .one(&tx)
        .await?
        .expect("aggregate without GROUP BY returns one row");
    assert_eq!(empty.matching_sales, 0);
    assert_eq!(empty.oldest_matching_sale_at, None);

    seed_nullable_row(&tx, None).await?;
    seed_nullable_row(&tx, Some("gamma".to_string())).await?;
    seed_nullable_row(&tx, Some("alpha".to_string())).await?;

    let nullable: FilteredNullableAgg = NullableRow::query()
        .select_only()
        .column_as(dbkit::func::count(NullableRow::id), "all_rows")
        .column_as(dbkit::func::count(NullableRow::id).filter(NullableRow::note.is_null()), "null_rows")
        .column_as(
            dbkit::func::count(NullableRow::note).filter(NullableRow::id.gt(0_i64)),
            "non_null_notes",
        )
        .column_as(
            dbkit::func::min(NullableRow::note).filter(NullableRow::note.is_null()),
            "first_null_note",
        )
        .into_model()
        .one(&tx)
        .await?
        .expect("aggregate without GROUP BY returns one row");
    assert_eq!(nullable.all_rows, 3);
    assert_eq!(nullable.null_rows, 1);
    assert_eq!(nullable.non_null_notes, 2);
    assert_eq!(nullable.first_null_note, None);

    Ok(())
}

#[tokio::test]
async fn aggregation_and_group_by_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day1 = NaiveDate::from_ymd_opt(2024, 2, 1).expect("day");
    let day2 = NaiveDate::from_ymd_opt(2024, 2, 2).expect("day");
    let day1_start = NaiveDateTime::new(day1, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));
    let day2_start = NaiveDateTime::new(day2, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));

    let inserted = Sale::insert_many(vec![
        SaleInsert {
            region: "us".to_string(),
            amount: 40,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(10, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "us".to_string(),
            amount: 70,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(12, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "eu".to_string(),
            amount: 30,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(14, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "apac".to_string(),
            amount: 200,
            created_at: NaiveDateTime::new(day2, NaiveTime::from_hms_opt(9, 0, 0).expect("time")),
        },
    ])
    .execute(&tx)
    .await?;
    assert_eq!(inserted, 4);

    let day1_end = NaiveDateTime::new(day1, NaiveTime::from_hms_opt(23, 59, 59).expect("time"));

    let mut amount_between = Sale::query().filter(Sale::amount.between(40_i64, 70_i64)).all(&tx).await?;
    amount_between.sort_by(|a, b| a.amount.cmp(&b.amount));
    assert_eq!(amount_between.len(), 2);
    assert_eq!(amount_between[0].amount, 40);
    assert_eq!(amount_between[1].amount, 70);

    let day1_sales = Sale::query()
        .filter(Sale::created_at.between(day1_start, day1_end))
        .all(&tx)
        .await?;
    assert_eq!(day1_sales.len(), 3);

    let mut region_rows: Vec<RegionAgg> = Sale::query()
        .select_only()
        .column(Sale::region)
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .column_as(dbkit::func::count(Sale::id), "count")
        .group_by(Sale::region)
        .order_by(dbkit::Order::asc(Sale::region.as_ref()))
        .having(dbkit::func::sum(Sale::amount).gt(100_i64))
        .into_model()
        .all(&tx)
        .await?;
    region_rows.sort_by(|a, b| a.region.cmp(&b.region));
    assert_eq!(region_rows.len(), 2);
    assert_eq!(region_rows[0].region, "apac");
    assert_eq!(region_rows[0].total.to_string(), "200");
    assert_eq!(region_rows[0].count, 1);
    assert_eq!(region_rows[1].region, "us");
    assert_eq!(region_rows[1].total.to_string(), "110");
    assert_eq!(region_rows[1].count, 2);

    let mut bucket_rows: Vec<BucketAgg> = Sale::query()
        .select_only()
        .column_as(dbkit::func::date_trunc("day", Sale::created_at), "bucket")
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .group_by(dbkit::func::date_trunc("day", Sale::created_at))
        .into_model()
        .all(&tx)
        .await?;
    bucket_rows.sort_by(|a, b| a.bucket.cmp(&b.bucket));
    assert_eq!(bucket_rows.len(), 2);
    assert_eq!(bucket_rows[0].bucket, day1_start);
    assert_eq!(bucket_rows[0].total.to_string(), "140");
    assert_eq!(bucket_rows[1].bucket, day2_start);
    assert_eq!(bucket_rows[1].total.to_string(), "200");

    let ordered_buckets: Vec<BucketAgg> = Sale::query()
        .select_only()
        .column_as(dbkit::func::date_trunc("day", Sale::created_at), "bucket")
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .group_by(dbkit::func::date_trunc("day", Sale::created_at))
        .order_by(dbkit::Order::desc(dbkit::func::date_trunc("day", Sale::created_at)))
        .into_model()
        .all(&tx)
        .await?;
    assert_eq!(ordered_buckets.len(), 2);
    assert_eq!(ordered_buckets[0].bucket, day2_start);
    assert_eq!(ordered_buckets[0].total.to_string(), "200");
    assert_eq!(ordered_buckets[1].bucket, day1_start);
    assert_eq!(ordered_buckets[1].total.to_string(), "140");

    let ordered_regions: Vec<RegionAgg> = Sale::query()
        .select_only()
        .column(Sale::region)
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .column_as(dbkit::func::count(Sale::id), "count")
        .group_by(Sale::region)
        .order_by(dbkit::Order::desc_alias("total"))
        .into_model()
        .all(&tx)
        .await?;
    assert_eq!(ordered_regions.len(), 3);
    assert_eq!(ordered_regions[0].region, "apac");
    assert_eq!(ordered_regions[0].total.to_string(), "200");
    assert_eq!(ordered_regions[1].region, "us");
    assert_eq!(ordered_regions[1].total.to_string(), "110");
    assert_eq!(ordered_regions[2].region, "eu");
    assert_eq!(ordered_regions[2].total.to_string(), "30");

    let mut extrema_rows: Vec<SaleExtremaAgg> = Sale::query()
        .select_only()
        .column(Sale::region)
        .column_as(dbkit::func::min(Sale::created_at), "first_sale_at")
        .column_as(dbkit::func::max(Sale::created_at), "last_sale_at")
        .column_as(dbkit::func::min(Sale::amount), "min_amount")
        .column_as(dbkit::func::max(Sale::amount), "max_amount")
        .group_by(Sale::region)
        .order_by(dbkit::Order::asc(Sale::region.as_ref()))
        .into_model()
        .all(&tx)
        .await?;
    extrema_rows.sort_by(|a, b| a.region.cmp(&b.region));
    assert_eq!(extrema_rows.len(), 3);
    assert_eq!(extrema_rows[0].region, "apac");
    assert_eq!(
        extrema_rows[0].first_sale_at,
        NaiveDateTime::new(day2, NaiveTime::from_hms_opt(9, 0, 0).expect("time"))
    );
    assert_eq!(
        extrema_rows[0].last_sale_at,
        NaiveDateTime::new(day2, NaiveTime::from_hms_opt(9, 0, 0).expect("time"))
    );
    assert_eq!(extrema_rows[0].min_amount, 200);
    assert_eq!(extrema_rows[0].max_amount, 200);
    assert_eq!(extrema_rows[2].region, "us");
    assert_eq!(
        extrema_rows[2].first_sale_at,
        NaiveDateTime::new(day1, NaiveTime::from_hms_opt(10, 0, 0).expect("time"))
    );
    assert_eq!(
        extrema_rows[2].last_sale_at,
        NaiveDateTime::new(day1, NaiveTime::from_hms_opt(12, 0, 0).expect("time"))
    );
    assert_eq!(extrema_rows[2].min_amount, 40);
    assert_eq!(extrema_rows[2].max_amount, 70);

    let empty_extrema: EmptySaleExtremaAgg = Sale::query()
        .select_only()
        .column_as(dbkit::func::min(Sale::created_at), "first_sale_at")
        .column_as(dbkit::func::max(Sale::created_at), "last_sale_at")
        .column_as(dbkit::func::min(Sale::amount), "min_amount")
        .column_as(dbkit::func::max(Sale::amount), "max_amount")
        .filter(Sale::region.eq("missing"))
        .into_model()
        .one(&tx)
        .await?
        .expect("aggregate without GROUP BY returns one row");
    assert_eq!(empty_extrema.first_sale_at, None);
    assert_eq!(empty_extrema.last_sale_at, None);
    assert_eq!(empty_extrema.min_amount, None);
    assert_eq!(empty_extrema.max_amount, None);

    let user = seed_user(&tx, "AggUser", "agg@db.com").await?;
    let _todo1 = seed_todo(&tx, user.id, "Alpha").await?;
    let _todo2 = seed_todo(&tx, user.id, "Beta").await?;

    let joined_rows: Vec<UserTodoAgg> = User::query()
        .select_only()
        .column_as(User::name, "name")
        .column_as(dbkit::func::count(Todo::id), "todo_count")
        .join(User::todos)
        .group_by(User::name)
        .order_by(dbkit::Order::desc(User::name.as_ref()))
        .into_model()
        .all(&tx)
        .await?;
    assert_eq!(joined_rows.len(), 1);
    assert_eq!(joined_rows[0].name, "AggUser");
    assert_eq!(joined_rows[0].todo_count, 2);

    Ok(())
}

#[tokio::test]
async fn min_max_nullable_text_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_nullable_row(&tx, None).await?;
    seed_nullable_row(&tx, Some("gamma".to_string())).await?;
    seed_nullable_row(&tx, Some("alpha".to_string())).await?;

    let extrema: NullableNoteExtremaAgg = NullableRow::query()
        .select_only()
        .column_as(dbkit::func::min(NullableRow::note), "min_note")
        .column_as(dbkit::func::max(NullableRow::note), "max_note")
        .into_model()
        .one(&tx)
        .await?
        .expect("aggregate without GROUP BY returns one row");
    assert_eq!(extrema.min_note.as_deref(), Some("alpha"));
    assert_eq!(extrema.max_note.as_deref(), Some("gamma"));

    let all_null_extrema: NullableNoteExtremaAgg = NullableRow::query()
        .select_only()
        .column_as(dbkit::func::min(NullableRow::note), "min_note")
        .column_as(dbkit::func::max(NullableRow::note), "max_note")
        .filter(NullableRow::note.is_null())
        .into_model()
        .one(&tx)
        .await?
        .expect("aggregate without GROUP BY returns one row");
    assert_eq!(all_null_extrema.min_note, None);
    assert_eq!(all_null_extrema.max_note, None);

    Ok(())
}

#[tokio::test]
async fn query_helpers_count_exists_first_paginate() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user1 = seed_user(&tx, "PageOne", "page1@db.com").await?;
    let user2 = seed_user(&tx, "PageTwo", "page2@db.com").await?;
    let user3 = seed_user(&tx, "PageThree", "page3@db.com").await?;

    let total = User::query().count(&tx).await?;
    assert_eq!(total, 3);

    let filtered_total = User::query().filter(User::email.eq("page2@db.com")).count(&tx).await?;
    assert_eq!(filtered_total, 1);

    let exists = User::query().filter(User::email.eq("page2@db.com")).exists(&tx).await?;
    assert!(exists);

    let missing = User::query().filter(User::email.eq("missing@db.com")).exists(&tx).await?;
    assert!(!missing);

    let first = User::query().order_by(dbkit::Order::asc(User::id.as_ref())).one(&tx).await?;
    assert_eq!(first.expect("first").id, user1.id);

    let page1 = User::query()
        .order_by(dbkit::Order::asc(User::id.as_ref()))
        .paginate(1, 2, &tx)
        .await?;
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.items[0].id, user1.id);
    assert_eq!(page1.items[1].id, user2.id);
    assert_eq!(page1.page, 1);
    assert_eq!(page1.per_page, 2);
    assert_eq!(page1.total, 3);
    assert_eq!(page1.total_pages(), 2);

    let page2 = User::query()
        .order_by(dbkit::Order::asc(User::id.as_ref()))
        .paginate(2, 2, &tx)
        .await?;
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.items[0].id, user3.id);
    assert_eq!(page2.page, 2);
    assert_eq!(page2.per_page, 2);
    assert_eq!(page2.total, 3);
    assert_eq!(page2.total_pages(), 2);

    Ok(())
}

#[tokio::test]
async fn many_to_many_selectin_loads_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Tagger", "tagger@db.com").await?;
    let todo1 = seed_todo(&tx, user.id, "First").await?;
    let todo2 = seed_todo(&tx, user.id, "Second").await?;

    let tag_a = seed_tag(&tx, "A").await?;
    let tag_b = seed_tag(&tx, "B").await?;

    let _t1a = seed_todo_tag(&tx, todo1.id, tag_a.id).await?;
    let _t1b = seed_todo_tag(&tx, todo1.id, tag_b.id).await?;
    let _t2b = seed_todo_tag(&tx, todo2.id, tag_b.id).await?;

    let todos: Vec<Todo<dbkit::NotLoaded, Vec<Tag>>> = Todo::query()
        .filter(Todo::user_id.eq(user.id))
        .with(Todo::tags.selectin())
        .all(&tx)
        .await?;

    assert_eq!(todos.len(), 2);
    let mut tags_t1: Vec<String> = todos
        .iter()
        .find(|todo| todo.id == todo1.id)
        .expect("todo1")
        .tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect();
    tags_t1.sort();
    assert_eq!(tags_t1, vec!["A", "B"]);

    let mut tags_t2: Vec<String> = todos
        .iter()
        .find(|todo| todo.id == todo2.id)
        .expect("todo2")
        .tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect();
    tags_t2.sort();
    assert_eq!(tags_t2, vec!["B"]);

    Ok(())
}

#[tokio::test]
async fn many_to_many_selectin_reverse_loads_parents() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Tagger", "tagger2@db.com").await?;
    let todo1 = seed_todo(&tx, user.id, "First").await?;
    let todo2 = seed_todo(&tx, user.id, "Second").await?;

    let tag_a = seed_tag(&tx, "A").await?;
    let tag_b = seed_tag(&tx, "B").await?;

    let _t1a = seed_todo_tag(&tx, todo1.id, tag_a.id).await?;
    let _t1b = seed_todo_tag(&tx, todo1.id, tag_b.id).await?;
    let _t2b = seed_todo_tag(&tx, todo2.id, tag_b.id).await?;

    let tags: Vec<Tag<Vec<Todo>>> = Tag::query().with(Tag::todos.selectin()).all(&tx).await?;

    let tag_a_loaded = tags.iter().find(|tag| tag.id == tag_a.id).expect("tag a");
    let mut todos_a: Vec<String> = tag_a_loaded.todos.iter().map(|todo| todo.title.clone()).collect();
    todos_a.sort();
    assert_eq!(todos_a, vec!["First"]);

    let tag_b_loaded = tags.iter().find(|tag| tag.id == tag_b.id).expect("tag b");
    let mut todos_b: Vec<String> = tag_b_loaded.todos.iter().map(|todo| todo.title.clone()).collect();
    todos_b.sort();
    assert_eq!(todos_b, vec!["First", "Second"]);

    Ok(())
}

#[tokio::test]
async fn many_to_many_join_filter() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Joiner", "joiner@db.com").await?;
    let todo1 = seed_todo(&tx, user.id, "First").await?;
    let todo2 = seed_todo(&tx, user.id, "Second").await?;

    let tag_a = seed_tag(&tx, "A").await?;
    let tag_b = seed_tag(&tx, "B").await?;

    let _t1a = seed_todo_tag(&tx, todo1.id, tag_a.id).await?;
    let _t2b = seed_todo_tag(&tx, todo2.id, tag_b.id).await?;

    let todos = Todo::query().join(Todo::tags).filter(Tag::name.eq("B")).distinct().all(&tx).await?;

    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, todo2.id);

    Ok(())
}

#[tokio::test]
async fn many_to_many_lazy_load() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Lazy", "lazy@db.com").await?;
    let todo = seed_todo(&tx, user.id, "First").await?;
    let tag = seed_tag(&tx, "A").await?;
    let _link = seed_todo_tag(&tx, todo.id, tag.id).await?;

    let loaded = todo.load(Todo::tags, &tx).await?;
    assert_eq!(loaded.tags.len(), 1);
    assert_eq!(loaded.tags[0].name, "A");

    Ok(())
}
#[tokio::test]
async fn active_insert_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut active = User::new_active();
    active.name = "Active".into();
    active.email = "active@db.com".into();

    let inserted = active.insert(&tx).await?;
    let _: User = inserted.clone();
    assert!(inserted.id > 0);
    assert_eq!(inserted.name, "Active");
    assert_eq!(inserted.email, "active@db.com");

    Ok(())
}

#[tokio::test]
async fn active_insert_missing_required_field_errors() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut active = UserActive::new();
    active.name = "Missing email".into();

    let result = active.insert(&tx).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn active_save_inserts_new_active_even_with_primary_key_set() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut active = OrderLine::new_active();
    active.order_id = 7.into();
    active.line_id = 8.into();
    active.note = "Saved".into();

    let inserted = active.save(&tx).await?;
    assert_eq!(inserted.order_id, 7);
    assert_eq!(inserted.line_id, 8);
    assert_eq!(inserted.note, "Saved");

    let fetched = OrderLine::query()
        .filter(OrderLine::order_id.eq(7))
        .filter(OrderLine::line_id.eq(8))
        .one(&tx)
        .await?
        .expect("order line");
    assert_eq!(fetched.note, "Saved");

    Ok(())
}

#[tokio::test]
async fn active_save_updates_loaded_models() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Before Save", "before-save@db.com").await?;
    let user_id = user.id;
    let user_email = user.email.clone();
    let mut active = user.into_active();
    active.name = "After Save".into();

    let saved = active.save(&tx).await?;
    assert_eq!(saved.id, user_id);
    assert_eq!(saved.name, "After Save");
    assert_eq!(saved.email, user_email);

    let fetched = User::by_id(user_id).one(&tx).await?.expect("user");
    assert_eq!(fetched.name, "After Save");
    assert_eq!(fetched.email, user_email);

    Ok(())
}

#[tokio::test]
async fn active_save_loaded_without_changes_is_noop() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "No Change", "no-change@db.com").await?;
    let user_id = user.id;
    let user_email = user.email.clone();
    let active = user.into_active();

    let saved = active.save(&tx).await?;
    assert_eq!(saved.id, user_id);
    assert_eq!(saved.name, "No Change");
    assert_eq!(saved.email, user_email);

    let fetched = User::by_id(user_id).one(&tx).await?.expect("user");
    assert_eq!(fetched.name, "No Change");
    assert_eq!(fetched.email, user_email);

    Ok(())
}

#[tokio::test]
async fn active_save_missing_required_fields_errors_on_insert() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut active = User::new_active();
    active.name = "Missing email".into();

    let result = active.save(&tx).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn active_save_composite_changing_key_returns_not_found() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let row = seed_order_line(&tx, 5, 5, "Orig").await?;
    let mut active = row.into_active();
    active.line_id = 9.into();
    active.note = "Changed".into();

    let result = active.save(&tx).await;
    assert!(result.is_err());

    let fetched = OrderLine::query()
        .filter(OrderLine::order_id.eq(5))
        .filter(OrderLine::line_id.eq(5))
        .one(&tx)
        .await?
        .expect("row");
    assert_eq!(fetched.note, "Orig");

    Ok(())
}

#[tokio::test]
async fn active_update_from_loaded() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Before", "before@db.com").await?;
    let user_id = user.id;
    let user_email = user.email.clone();
    let mut active = user.into_active();
    active.name = "After".into();

    let updated = active.update(&tx).await?;
    let _: User = updated.clone();
    assert_eq!(updated.id, user_id);
    assert_eq!(updated.name, "After");
    assert_eq!(updated.email, user_email);

    Ok(())
}

#[tokio::test]
async fn active_update_does_not_overwrite_other_fields() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    // This test simulates a concurrent update to a different column and ensures
    // ActiveModel updates only changed fields (no stale-field overwrite).
    let user = seed_user(&tx, "Before", "before@db.com").await?;
    let mut active = user.clone().into_active();

    User::update()
        .set(User::email, "updated@db.com")
        .filter(User::id.eq(user.id))
        .execute(&tx)
        .await?;

    active.name = "After".into();
    let _ = active.update(&tx).await?;

    let fetched = User::by_id(user.id).one(&tx).await?.expect("user");
    assert_eq!(fetched.name, "After");
    assert_eq!(fetched.email, "updated@db.com");

    Ok(())
}

#[tokio::test]
async fn active_update_set_null() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let row = seed_nullable_row(&tx, Some("note".to_string())).await?;
    let row_id = row.id;
    let mut active = row.into_active();
    active.note = None.into();

    let updated = active.update(&tx).await?;
    let _: NullableRow = updated.clone();
    assert_eq!(updated.id, row_id);
    assert!(updated.note.is_none());

    Ok(())
}

#[tokio::test]
async fn active_update_requires_primary_key() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut active = UserActive::new();
    active.name = "No PK".into();
    active.email = "no-pk@db.com".into();

    let result = active.update(&tx).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn active_update_uses_only_primary_key_filter() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Before", "before@db.com").await?;
    let untouched = seed_user(&tx, "Other", "other@db.com").await?;
    let user_id = user.id;
    let mut active = user.into_active();
    active.name = "After".into();
    active.email = "after@db.com".into();

    let updated = active.update(&tx).await?;
    assert_eq!(updated.id, user_id);
    assert_eq!(updated.name, "After");
    assert_eq!(updated.email, "after@db.com");

    let fetched = User::by_id(user_id).one(&tx).await?.expect("user");
    assert_eq!(fetched.name, "After");
    assert_eq!(fetched.email, "after@db.com");

    let other = User::by_id(untouched.id).one(&tx).await?.expect("other");
    assert_eq!(other.name, "Other");
    assert_eq!(other.email, "other@db.com");

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_update_uses_both_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let first = seed_order_line(&tx, 1, 1, "A").await?;
    let _same_order = seed_order_line(&tx, 1, 2, "B").await?;
    let _same_line = seed_order_line(&tx, 2, 1, "C").await?;

    let mut active = first.into_active();
    active.note = "A1".into();
    let updated = active.update(&tx).await?;
    assert_eq!(updated.order_id, 1);
    assert_eq!(updated.line_id, 1);
    assert_eq!(updated.note, "A1");

    let fetched = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(1))
        .one(&tx)
        .await?
        .expect("updated");
    assert_eq!(fetched.note, "A1");

    let same_order = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(2))
        .one(&tx)
        .await?
        .expect("same order");
    assert_eq!(same_order.note, "B");

    let same_line = OrderLine::query()
        .filter(OrderLine::order_id.eq(2))
        .filter(OrderLine::line_id.eq(1))
        .one(&tx)
        .await?
        .expect("same line");
    assert_eq!(same_line.note, "C");

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_update_requires_all_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut missing_line = OrderLine::new_active();
    missing_line.order_id = 1.into();
    missing_line.note = "Missing line".into();
    assert!(missing_line.update(&tx).await.is_err());

    let mut missing_order = OrderLine::new_active();
    missing_order.line_id = 1.into();
    missing_order.note = "Missing order".into();
    assert!(missing_order.update(&tx).await.is_err());

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_insert_requires_all_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut missing_line = OrderLine::new_active();
    missing_line.order_id = 1.into();
    missing_line.note = "Missing line".into();
    assert!(missing_line.insert(&tx).await.is_err());

    let mut missing_order = OrderLine::new_active();
    missing_order.line_id = 1.into();
    missing_order.note = "Missing order".into();
    assert!(missing_order.insert(&tx).await.is_err());

    Ok(())
}

#[tokio::test]
async fn active_delete_removes_only_target() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Delete", "delete@db.com").await?;
    let user_id = user.id;
    let other = seed_user(&tx, "Keep", "keep@db.com").await?;

    let active = user.into_active();
    let deleted = active.delete(&tx).await?;
    assert_eq!(deleted, 1);

    let removed = User::by_id(user_id).one(&tx).await?;
    assert!(removed.is_none());

    let remaining = User::by_id(other.id).one(&tx).await?.expect("other");
    assert_eq!(remaining.email, "keep@db.com");

    Ok(())
}

#[tokio::test]
async fn active_delete_requires_primary_key() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut active = User::new_active();
    active.name = "No PK".into();
    active.email = "no-pk@db.com".into();

    let result = active.delete(&tx).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn model_delete_removes_row() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let user = seed_user(&tx, "Delete", "delete2@db.com").await?;
    let user_id = user.id;
    let deleted = user.delete(&tx).await?;
    assert_eq!(deleted, 1);

    let removed = User::by_id(user_id).one(&tx).await?;
    assert!(removed.is_none());

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_delete_uses_both_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let target = seed_order_line(&tx, 1, 1, "A").await?;
    let _same_order = seed_order_line(&tx, 1, 2, "B").await?;
    let _same_line = seed_order_line(&tx, 2, 1, "C").await?;

    let deleted = target.into_active().delete(&tx).await?;
    assert_eq!(deleted, 1);

    let removed = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(1))
        .one(&tx)
        .await?;
    assert!(removed.is_none());

    let same_order = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(2))
        .one(&tx)
        .await?
        .expect("same order");
    assert_eq!(same_order.note, "B");

    let same_line = OrderLine::query()
        .filter(OrderLine::order_id.eq(2))
        .filter(OrderLine::line_id.eq(1))
        .one(&tx)
        .await?
        .expect("same line");
    assert_eq!(same_line.note, "C");

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_delete_requires_all_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let mut missing_line = OrderLine::new_active();
    missing_line.order_id = 1.into();
    missing_line.note = "Missing line".into();
    assert!(missing_line.delete(&tx).await.is_err());

    let mut missing_order = OrderLine::new_active();
    missing_order.line_id = 1.into();
    missing_order.note = "Missing order".into();
    assert!(missing_order.delete(&tx).await.is_err());

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_model_delete_removes_row() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let target = seed_order_line(&tx, 9, 9, "Z").await?;
    let order_id = target.order_id;
    let line_id = target.line_id;
    let deleted = target.delete(&tx).await?;
    assert_eq!(deleted, 1);

    let removed = OrderLine::query()
        .filter(OrderLine::order_id.eq(order_id))
        .filter(OrderLine::line_id.eq(line_id))
        .one(&tx)
        .await?;
    assert!(removed.is_none());

    Ok(())
}

#[tokio::test]
async fn locking_for_update_blocks_until_first_transaction_releases_lock() -> Result<(), dbkit::Error> {
    let db_a = Database::connect(&db_url()).await?;
    let db_b = Database::connect(&db_url()).await?;
    setup_locking_schema(&db_a).await?;

    let token = unique_lock_token();
    let row = seed_lock_row(&db_a, token, "row-1").await?;

    let tx1 = db_a.begin().await?;
    let locked = LockRow::query().filter(LockRow::id.eq(row.id)).for_update().one(&tx1).await?;
    assert!(locked.is_some());

    let row_id = row.id;
    let handle = tokio::spawn(async move {
        let tx2 = db_b.begin().await?;
        let row = LockRow::query().filter(LockRow::id.eq(row_id)).for_update().one(&tx2).await?;
        tx2.rollback().await?;
        Ok::<Option<LockRow>, dbkit::Error>(row)
    });

    sleep(Duration::from_millis(150)).await;
    assert!(!handle.is_finished(), "second transaction acquired lock before first released it");

    tx1.commit().await?;

    let locked_after_release = timeout(Duration::from_secs(2), handle)
        .await
        .expect("second transaction should complete once lock is released")
        .expect("join should succeed")?;
    assert!(locked_after_release.is_some());

    cleanup_lock_rows(&db_a, token).await?;
    Ok(())
}
#[tokio::test]
async fn locking_skip_locked_skips_rows_locked_by_another_transaction() -> Result<(), dbkit::Error> {
    let db_a = Database::connect(&db_url()).await?;
    let db_b = Database::connect(&db_url()).await?;
    setup_locking_schema(&db_a).await?;

    let token = unique_lock_token();
    let first = seed_lock_row(&db_a, token, "first").await?;
    let second = seed_lock_row(&db_a, token, "second").await?;

    let tx1 = db_a.begin().await?;
    let locked_first = LockRow::query().filter(LockRow::id.eq(first.id)).for_update().one(&tx1).await?;
    assert!(locked_first.is_some());

    let tx2 = db_b.begin().await?;
    let rows = LockRow::query()
        .filter(LockRow::token.eq(token))
        .order_by(dbkit::Order::asc(LockRow::id))
        .for_update()
        .skip_locked()
        .all(&tx2)
        .await?;
    tx2.rollback().await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, second.id);

    tx1.rollback().await?;
    cleanup_lock_rows(&db_a, token).await?;
    Ok(())
}

#[tokio::test]
async fn locking_nowait_errors_immediately_when_row_is_locked() -> Result<(), dbkit::Error> {
    let db_a = Database::connect(&db_url()).await?;
    let db_b = Database::connect(&db_url()).await?;
    setup_locking_schema(&db_a).await?;

    let token = unique_lock_token();
    let row = seed_lock_row(&db_a, token, "locked").await?;

    let tx1 = db_a.begin().await?;
    let _locked = LockRow::query().filter(LockRow::id.eq(row.id)).for_update().one(&tx1).await?;

    let tx2 = db_b.begin().await?;
    let err = LockRow::query()
        .filter(LockRow::id.eq(row.id))
        .for_update()
        .nowait()
        .one(&tx2)
        .await
        .expect_err("NOWAIT should fail when the row is already locked");
    assert!(
        is_lock_not_available(&err),
        "expected postgres lock_not_available (55P03), got: {err:?}"
    );
    tx2.rollback().await?;

    tx1.rollback().await?;
    cleanup_lock_rows(&db_a, token).await?;
    Ok(())
}

#[tokio::test]
async fn locking_nowait_succeeds_without_lock_contention() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    setup_locking_schema(&db).await?;

    let token = unique_lock_token();
    let row = seed_lock_row(&db, token, "free").await?;

    let tx = db.begin().await?;
    let selected = LockRow::query()
        .filter(LockRow::id.eq(row.id))
        .for_update()
        .nowait()
        .one(&tx)
        .await?;
    tx.rollback().await?;

    let selected = selected.expect("row should be selected");
    assert_eq!(selected.id, row.id);

    cleanup_lock_rows(&db, token).await?;
    Ok(())
}

#[tokio::test]
async fn locking_for_update_with_limit_locks_only_selected_rows() -> Result<(), dbkit::Error> {
    let db_a = Database::connect(&db_url()).await?;
    let db_b = Database::connect(&db_url()).await?;
    setup_locking_schema(&db_a).await?;

    let token = unique_lock_token();
    let first = seed_lock_row(&db_a, token, "first").await?;
    let second = seed_lock_row(&db_a, token, "second").await?;

    let tx1 = db_a.begin().await?;
    let selected = LockRow::query()
        .filter(LockRow::token.eq(token))
        .order_by(dbkit::Order::asc(LockRow::id))
        .limit(1)
        .for_update()
        .all(&tx1)
        .await?;
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].id, first.id);

    let tx2 = db_b.begin().await?;
    let second_row = LockRow::query()
        .filter(LockRow::id.eq(second.id))
        .for_update()
        .nowait()
        .one(&tx2)
        .await?;
    let second_row = second_row.expect("second row should not be locked by tx1");
    assert_eq!(second_row.id, second.id);

    let first_lock_err = LockRow::query()
        .filter(LockRow::id.eq(first.id))
        .for_update()
        .nowait()
        .one(&tx2)
        .await
        .expect_err("first row should be locked by tx1");
    assert!(
        is_lock_not_available(&first_lock_err),
        "expected postgres lock_not_available (55P03), got: {first_lock_err:?}"
    );

    tx2.rollback().await?;
    tx1.rollback().await?;
    cleanup_lock_rows(&db_a, token).await?;
    Ok(())
}
