#![allow(non_upper_case_globals)]

use dbkit::executor::BoxFuture;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Error, Executor, SelectExt, Value};

#[model(table = "renamed_parents")]
struct RenamedParent {
    #[key]
    #[autoincrement]
    id: i64,
    #[dbkit(column = "type")]
    type_: String,
    #[dbkit(column = "external_ref")]
    external_reference: String,
    label: String,
    #[has_many]
    children: dbkit::HasMany<RenamedChild>,
}

#[model(table = "renamed_children")]
struct RenamedChild {
    #[key]
    #[autoincrement]
    id: i64,
    parent_id: i64,
    #[dbkit(column = "type")]
    type_: String,
    #[dbkit(column = "sort_key")]
    rank_key: i64,
    #[belongs_to(key = parent_id, references = id)]
    parent: dbkit::BelongsTo<RenamedParent>,
}

struct CaptureExecutor {
    sqls: std::sync::Mutex<Vec<String>>,
}

impl CaptureExecutor {
    fn new() -> Self {
        Self {
            sqls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Executor for CaptureExecutor {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn fetch_optional<'e, T>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(None) })
    }

    fn fetch_rows<'e>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Vec<dbkit::sqlx::postgres::PgRow>, Error>> {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn execute<'e>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(0) })
    }
}

#[test]
fn renamed_column_metadata_uses_database_name() {
    assert_eq!(RenamedParent::type_.as_ref().name, "type");
    assert_eq!(RenamedParent::COLUMNS[1].name, "type");
    assert_eq!(RenamedParent::external_reference.as_ref().name, "external_ref");
    assert_eq!(RenamedParent::COLUMNS[2].name, "external_ref");
    assert_eq!(RenamedChild::type_.as_ref().name, "type");
    assert_eq!(RenamedChild::COLUMNS[2].name, "type");
    assert_eq!(RenamedChild::rank_key.as_ref().name, "sort_key");
    assert_eq!(RenamedChild::COLUMNS[3].name, "sort_key");
}

#[test]
fn renamed_columns_are_used_in_select_sql() {
    let compiled = RenamedParent::query()
        .filter(RenamedParent::type_.eq("primary"))
        .filter(RenamedParent::external_reference.eq("ref-1"))
        .order_by(dbkit::Order::asc(RenamedParent::external_reference.as_ref()))
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT renamed_parents.* FROM renamed_parents WHERE (renamed_parents.type = $1) AND (renamed_parents.external_ref = $2) ORDER BY renamed_parents.external_ref ASC"
    );
    assert_eq!(
        compiled.binds,
        vec![Value::String("primary".to_string()), Value::String("ref-1".to_string()),]
    );
}

#[test]
fn renamed_columns_are_used_in_mutation_sql() {
    let insert = RenamedParent::insert(RenamedParentInsert {
        type_: "primary".to_string(),
        external_reference: "ref-1".to_string(),
        label: "Example".to_string(),
    })
    .on_conflict_do_update(RenamedParent::type_, (RenamedParent::external_reference, RenamedParent::label))
    .returning_all()
    .compile();

    assert_eq!(
        insert.sql,
        "INSERT INTO renamed_parents (type, external_ref, label) VALUES ($1, $2, $3) ON CONFLICT (type) DO UPDATE SET external_ref = EXCLUDED.external_ref, label = EXCLUDED.label RETURNING renamed_parents.*"
    );
    assert_eq!(
        insert.binds,
        vec![
            Value::String("primary".to_string()),
            Value::String("ref-1".to_string()),
            Value::String("Example".to_string()),
        ]
    );

    let update = RenamedParent::update()
        .set(RenamedParent::type_, "secondary")
        .set(RenamedParent::external_reference, "ref-2")
        .filter(RenamedParent::id.eq(1_i64))
        .returning_all()
        .compile();

    assert_eq!(
        update.sql,
        "UPDATE renamed_parents SET type = $1, external_ref = $2 WHERE (renamed_parents.id = $3) RETURNING renamed_parents.*"
    );
    assert_eq!(
        update.binds,
        vec![
            Value::String("secondary".to_string()),
            Value::String("ref-2".to_string()),
            Value::I64(1),
        ]
    );
}

#[tokio::test]
async fn joined_loading_sql_aliases_renamed_columns_by_database_name() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let _rows: Vec<RenamedParent<Vec<RenamedChild>>> = RenamedParent::query().with(RenamedParent::children.joined()).all(&ex).await?;

    let sqls = ex.sqls.lock().expect("lock");
    assert_eq!(sqls.len(), 1);
    let sql = &sqls[0];
    assert!(
        sql.contains("renamed_children.type AS __dbkit_j0__type"),
        "joined SQL should select renamed child column with DB-name alias: {sql}"
    );
    assert!(
        sql.contains("renamed_children.sort_key AS __dbkit_j0__sort_key"),
        "joined SQL should select arbitrary renamed child column with DB-name alias: {sql}"
    );
    assert!(
        !sql.contains("__dbkit_j0__type_"),
        "joined SQL should not alias renamed child column with Rust field name: {sql}"
    );
    assert!(
        !sql.contains("__dbkit_j0__rank_key"),
        "joined SQL should not alias renamed child column with Rust field name: {sql}"
    );

    Ok(())
}
