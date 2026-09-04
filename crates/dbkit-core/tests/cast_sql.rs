use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use dbkit_core::{Column, PgInterval, Select, Table};
use uuid::Uuid;

struct CastRow;

fn cast_rows_table() -> Table {
    Table::new("cast_rows")
}

fn source() -> Column<CastRow, String> {
    Column::new(cast_rows_table(), "source")
}

#[test]
fn compiles_every_supported_cast_target_with_postgresql_type_names() {
    let query: Select<CastRow> = Select::new(cast_rows_table())
        .select_only()
        .column_as(source().cast::<bool>(), "as_bool")
        .column_as(source().cast::<i16>(), "as_i16")
        .column_as(source().cast::<i32>(), "as_i32")
        .column_as(source().cast::<i64>(), "as_i64")
        .column_as(source().cast::<f32>(), "as_f32")
        .column_as(source().cast::<f64>(), "as_f64")
        .column_as(source().cast::<String>(), "as_text")
        .column_as(source().cast::<Uuid>(), "as_uuid")
        .column_as(source().cast::<NaiveDateTime>(), "as_timestamp")
        .column_as(source().cast::<DateTime<Utc>>(), "as_timestamptz")
        .column_as(source().cast::<NaiveDate>(), "as_date")
        .column_as(source().cast::<NaiveTime>(), "as_time")
        .column_as(source().cast::<PgInterval>(), "as_interval");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT CAST(cast_rows.source AS BOOLEAN) AS as_bool, CAST(cast_rows.source AS SMALLINT) AS as_i16, CAST(cast_rows.source AS INTEGER) AS as_i32, CAST(cast_rows.source AS BIGINT) AS as_i64, CAST(cast_rows.source AS REAL) AS as_f32, CAST(cast_rows.source AS DOUBLE PRECISION) AS as_f64, CAST(cast_rows.source AS TEXT) AS as_text, CAST(cast_rows.source AS UUID) AS as_uuid, CAST(cast_rows.source AS TIMESTAMP) AS as_timestamp, CAST(cast_rows.source AS TIMESTAMPTZ) AS as_timestamptz, CAST(cast_rows.source AS DATE) AS as_date, CAST(cast_rows.source AS TIME) AS as_time, CAST(cast_rows.source AS INTERVAL) AS as_interval FROM cast_rows"
    );
    assert!(sql.binds.is_empty());
}
