#![cfg(feature = "migrations")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use dbkit::sqlx::{self, migrate::Migrator};
use dbkit::{Database, Error};

struct TempMigrationDir {
    path: PathBuf,
}

impl TempMigrationDir {
    fn new(test_name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock moved backwards")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("dbkit_migrations_{test_name}_{now}"));
        fs::create_dir_all(&dir).expect("failed to create temporary migration directory");
        Self { path: dir }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, filename: &str, sql: &str) {
        let file = self.path.join(filename);
        fs::write(&file, sql).expect("failed to write migration file");
    }
}

impl Drop for TempMigrationDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for migration tests")
}

fn unique_version() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock moved backwards")
        .as_nanos() as i64
}

fn unique_table(prefix: &str) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock moved backwards")
        .as_nanos();
    format!("{prefix}_{now}")
}

fn migration_test_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

async fn connect_db() -> Database {
    let url = db_url();
    Database::connect(&url)
        .await
        .expect("failed to connect to postgres for migration tests")
}

async fn make_test_migrator(path: &Path) -> Migrator {
    let mut migrator = Migrator::new(path).await.expect("failed to create sqlx migrator");
    migrator.set_ignore_missing(true);
    migrator
}

async fn table_exists(db: &Database, table_name: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = $1
        )",
    )
    .bind(table_name)
    .fetch_one(db.pool())
    .await
    .expect("failed to query table existence")
}

async fn column_exists(db: &Database, table_name: &str, column_name: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1
            FROM information_schema.columns
            WHERE table_schema = 'public'
              AND table_name = $1
              AND column_name = $2
        )",
    )
    .bind(table_name)
    .bind(column_name)
    .fetch_one(db.pool())
    .await
    .expect("failed to query column existence")
}

#[tokio::test]
async fn migrate_applies_simple_migration_file() {
    let _guard = migration_test_lock().lock().await;
    let db = connect_db().await;

    let dir = TempMigrationDir::new("apply_simple");
    let version = unique_version();
    let table = unique_table("dbkit_mig_apply");
    dir.write(
        &format!("{version}_create_{table}.sql"),
        &format!("CREATE TABLE {table} (id BIGSERIAL PRIMARY KEY, note TEXT NOT NULL);"),
    );

    let migrator = make_test_migrator(dir.path()).await;

    db.migrate(&migrator).await.expect("migration should apply");

    assert!(table_exists(&db, &table).await, "table was not created");
}

#[tokio::test]
async fn migrate_is_idempotent_when_no_new_migrations_exist() {
    let _guard = migration_test_lock().lock().await;
    let db = connect_db().await;

    let dir = TempMigrationDir::new("idempotent");
    let version = unique_version();
    let table = unique_table("dbkit_mig_idempotent");
    dir.write(
        &format!("{version}_create_{table}.sql"),
        &format!("CREATE TABLE {table} (id BIGSERIAL PRIMARY KEY);"),
    );

    let migrator = make_test_migrator(dir.path()).await;

    db.migrate(&migrator).await.expect("first migration run should apply");
    db.migrate(&migrator).await.expect("second migration run should be a no-op");

    let applied_count: i64 = sqlx::query_scalar("SELECT COUNT(*)::BIGINT FROM _sqlx_migrations WHERE version = $1")
        .bind(version)
        .fetch_one(db.pool())
        .await
        .expect("failed to query migration ledger");

    assert_eq!(applied_count, 1, "migration should be recorded exactly once");
}

#[tokio::test]
async fn migrate_respects_migration_order_by_version() {
    let _guard = migration_test_lock().lock().await;
    let db = connect_db().await;

    let dir = TempMigrationDir::new("ordering");
    let base = unique_version();
    let table = unique_table("dbkit_mig_ordering");
    dir.write(
        &format!("{base}_create_{table}.sql"),
        &format!("CREATE TABLE {table} (id BIGSERIAL PRIMARY KEY);"),
    );
    dir.write(
        &format!("{}_alter_{table}.sql", base + 1),
        &format!("ALTER TABLE {table} ADD COLUMN after_create TEXT NOT NULL DEFAULT '';",),
    );

    let migrator = make_test_migrator(dir.path()).await;

    db.migrate(&migrator).await.expect("migrations should apply in order");

    assert!(
        column_exists(&db, &table, "after_create").await,
        "second migration did not run after table creation"
    );
}

#[tokio::test]
async fn migrate_applies_reversible_up_down_migrations() {
    let _guard = migration_test_lock().lock().await;
    let db = connect_db().await;

    let dir = TempMigrationDir::new("reversible");
    let version = unique_version();
    let table = unique_table("dbkit_mig_reversible");
    dir.write(
        &format!("{version}_create_{table}.up.sql"),
        &format!("CREATE TABLE {table} (id BIGSERIAL PRIMARY KEY);"),
    );
    dir.write(&format!("{version}_create_{table}.down.sql"), &format!("DROP TABLE {table};"));

    let migrator = make_test_migrator(dir.path()).await;

    db.migrate(&migrator).await.expect("reversible migration should apply");

    assert!(table_exists(&db, &table).await, "table was not created by .up.sql migration");
}

#[tokio::test]
async fn migrate_maps_invalid_sql_to_migration_error_variant() {
    let _guard = migration_test_lock().lock().await;
    let db = connect_db().await;

    let dir = TempMigrationDir::new("invalid_sql");
    let version = unique_version();
    dir.write(&format!("{version}_invalid.sql"), "THIS IS NOT VALID SQL;");

    let migrator = make_test_migrator(dir.path()).await;

    let err = db.migrate(&migrator).await.expect_err("invalid SQL migration should fail");

    assert!(
        matches!(err, Error::Migrate(_)),
        "expected Error::Migrate for migration failures, got: {err:?}"
    );
}
