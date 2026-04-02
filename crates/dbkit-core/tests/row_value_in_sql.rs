use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dbkit_core::{expr::Value, row, Column, Select, Table};

#[derive(Debug)]
struct LookupEntry;

#[derive(Debug)]
struct RevisionSnapshot;

fn lookup_entries_table() -> Table {
    Table::new("lookup_entries")
}

fn lookup_entry_namespace() -> Column<LookupEntry, String> {
    Column::new(lookup_entries_table(), "namespace")
}

fn lookup_entry_name() -> Column<LookupEntry, String> {
    Column::new(lookup_entries_table(), "name")
}

fn lookup_entry_locale() -> Column<LookupEntry, String> {
    Column::new(lookup_entries_table(), "locale")
}

fn revision_snapshots_table() -> Table {
    Table::new("revision_snapshots")
}

fn revision_snapshot_series_id() -> Column<RevisionSnapshot, i64> {
    Column::new(revision_snapshots_table(), "series_id")
}

fn revision_snapshot_revision() -> Column<RevisionSnapshot, i32> {
    Column::new(revision_snapshots_table(), "revision")
}

fn revision_snapshot_captured_at() -> Column<RevisionSnapshot, NaiveDateTime> {
    Column::new(revision_snapshots_table(), "captured_at")
}

#[test]
fn compiles_row_value_in_expression_for_three_columns() {
    let compiled = Select::<LookupEntry>::new(lookup_entries_table())
        .filter(
            row((lookup_entry_namespace(), lookup_entry_name(), lookup_entry_locale()))
                .in_([("cities", "berlin", "en"), ("countries", "germany", "de")]),
        )
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT lookup_entries.* FROM lookup_entries WHERE ((lookup_entries.namespace, lookup_entries.name, lookup_entries.locale) IN (($1, $2, $3), ($4, $5, $6)))"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::String("cities".to_string()),
            Value::String("berlin".to_string()),
            Value::String("en".to_string()),
            Value::String("countries".to_string()),
            Value::String("germany".to_string()),
            Value::String("de".to_string()),
        ]
    );
}

#[test]
fn row_value_in_empty_compiles_to_false_without_binds() {
    let compiled = Select::<LookupEntry>::new(lookup_entries_table())
        .filter(row((lookup_entry_namespace(), lookup_entry_name(), lookup_entry_locale())).in_(std::iter::empty::<(&str, &str, &str)>()))
        .compile();

    assert_eq!(compiled.sql, "SELECT lookup_entries.* FROM lookup_entries WHERE (FALSE)");
    assert!(compiled.binds.is_empty());
}

#[test]
fn row_value_in_reuses_identical_binds_across_rows() {
    let compiled = Select::<LookupEntry>::new(lookup_entries_table())
        .filter(
            row((lookup_entry_namespace(), lookup_entry_name(), lookup_entry_locale()))
                .in_([("cities", "berlin", "en"), ("cities", "munich", "de")]),
        )
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT lookup_entries.* FROM lookup_entries WHERE ((lookup_entries.namespace, lookup_entries.name, lookup_entries.locale) IN (($1, $2, $3), ($1, $4, $5)))"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::String("cities".to_string()),
            Value::String("berlin".to_string()),
            Value::String("en".to_string()),
            Value::String("munich".to_string()),
            Value::String("de".to_string()),
        ]
    );
}

#[test]
fn row_value_in_supports_mixed_integer_and_timestamp_types() {
    let first = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 1, 2).expect("date"),
        NaiveTime::from_hms_opt(3, 4, 5).expect("time"),
    );
    let second = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 1, 3).expect("date"),
        NaiveTime::from_hms_opt(6, 7, 8).expect("time"),
    );

    let compiled = Select::<RevisionSnapshot>::new(revision_snapshots_table())
        .filter(
            row((
                revision_snapshot_series_id(),
                revision_snapshot_revision(),
                revision_snapshot_captured_at(),
            ))
            .in_([(11_i64, 1_i32, first), (11_i64, 2_i32, second)]),
        )
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT revision_snapshots.* FROM revision_snapshots WHERE ((revision_snapshots.series_id, revision_snapshots.revision, revision_snapshots.captured_at) IN (($1, $2, $3), ($1, $4, $5)))"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::I64(11),
            Value::I32(1),
            Value::DateTime(first),
            Value::I32(2),
            Value::DateTime(second),
        ]
    );
}
