use dbkit_core::{expr::Value, func, Column, Expr, Select, Table};

#[derive(Debug)]
struct Job;

fn jobs_table() -> Table {
    Table::new("jobs")
}

fn job_id() -> Column<Job, i64> {
    Column::new(jobs_table(), "id")
}

fn job_retry_count() -> Column<Job, i64> {
    Column::new(jobs_table(), "retry_count")
}

fn job_max_retries() -> Column<Job, i64> {
    Column::new(jobs_table(), "max_retries")
}

fn job_content_hash() -> Column<Job, String> {
    Column::new(jobs_table(), "content_hash")
}

fn job_last_content_hash() -> Column<Job, String> {
    Column::new(jobs_table(), "last_content_hash")
}

fn job_embedding() -> Column<Job, String> {
    Column::new(jobs_table(), "embedding")
}

fn job_embedding_hash() -> Column<Job, String> {
    Column::new(jobs_table(), "embedding_hash")
}

#[test]
fn compiles_ne_col_between_non_nullable_columns() {
    let expr = job_content_hash().ne_col(job_last_content_hash());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.content_hash <> jobs.last_content_hash)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_lt_col_between_numeric_columns() {
    let expr = job_retry_count().lt_col(job_max_retries());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.retry_count < jobs.max_retries)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_le_col_between_numeric_columns() {
    let expr = job_retry_count().le_col(job_max_retries());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.retry_count <= jobs.max_retries)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_gt_col_between_numeric_columns() {
    let expr = job_retry_count().gt_col(job_max_retries());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.retry_count > jobs.max_retries)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_ge_col_between_numeric_columns() {
    let expr = job_retry_count().ge_col(job_max_retries());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.retry_count >= jobs.max_retries)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_ne_col_between_potentially_nullable_columns() {
    let expr = job_embedding_hash().ne_col(job_embedding());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.embedding_hash <> jobs.embedding)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_is_distinct_from_col_between_potentially_nullable_columns() {
    let expr = job_embedding_hash().is_distinct_from_col(job_embedding());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.embedding_hash IS DISTINCT FROM jobs.embedding)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_is_not_distinct_from_col_between_potentially_nullable_columns() {
    let expr = job_embedding_hash().is_not_distinct_from_col(job_embedding());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (jobs.embedding_hash IS NOT DISTINCT FROM jobs.embedding)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_expr_ne_col_with_coalesce_for_nullable_vs_nonnullable() {
    let expr = func::coalesce_col(job_embedding_hash(), "").ne_col(job_content_hash());
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (COALESCE(jobs.embedding_hash, $1) <> jobs.content_hash)"
    );
    assert_eq!(sql.binds, vec![Value::String("".to_string())]);
}

#[test]
fn compiles_stale_embedding_predicate_with_null_checks_and_hash_mismatch() {
    let expr = job_embedding()
        .is_null()
        .or(job_embedding_hash().is_distinct_from_col(job_content_hash()));
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE ((jobs.embedding IS NULL) OR (jobs.embedding_hash IS DISTINCT FROM jobs.content_hash))"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_stale_embedding_predicate_with_coalesce_and_hash_mismatch() {
    let expr = job_embedding()
        .is_null()
        .or(job_embedding_hash().is_null())
        .or(func::coalesce_col(job_embedding_hash(), "").ne_col(job_content_hash()));
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (((jobs.embedding IS NULL) OR (jobs.embedding_hash IS NULL)) OR (COALESCE(jobs.embedding_hash, $1) <> jobs.content_hash))"
    );
    assert_eq!(sql.binds, vec![Value::String("".to_string())]);
}

#[test]
fn compiles_mixed_boolean_condition_using_col_comparison_helpers() {
    let expr = job_retry_count()
        .lt_col(job_max_retries())
        .and(job_content_hash().ne_col(job_last_content_hash()))
        .and(job_id().gt(0_i64));
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT jobs.* FROM jobs WHERE (((jobs.retry_count < jobs.max_retries) AND (jobs.content_hash <> jobs.last_content_hash)) AND (jobs.id > $1))"
    );
    assert_eq!(sql.binds, vec![Value::I64(0)]);
}

fn expr_sql(expr: Expr<bool>) -> dbkit_core::CompiledSql {
    let query: Select<Job> = Select::new(jobs_table()).filter(expr);
    query.compile()
}
