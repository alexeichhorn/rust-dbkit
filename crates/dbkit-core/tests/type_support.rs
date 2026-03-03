use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use dbkit_core::{Column, Select, Table, Value};
use serde_json::json;
use uuid::Uuid;

#[test]
fn value_from_uuid() {
    let id = Uuid::nil();
    assert_eq!(Value::from(id), Value::Uuid(id));
}

#[test]
fn value_from_datetime_date_time() {
    let date = NaiveDate::from_ymd_opt(2024, 1, 2).expect("date");
    let time = NaiveTime::from_hms_opt(3, 4, 5).expect("time");
    let datetime = NaiveDateTime::new(date, time);

    assert_eq!(Value::from(datetime), Value::DateTime(datetime));
    assert_eq!(Value::from(date), Value::Date(date));
    assert_eq!(Value::from(time), Value::Time(time));
}

#[test]
fn value_from_utc_datetime() {
    let datetime: DateTime<Utc> = Utc
        .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
        .single()
        .expect("utc datetime");

    assert_eq!(Value::from(datetime), Value::DateTimeUtc(datetime));
}

#[test]
fn select_binds_uuid_datetime_date_time() {
    let table = Table::new("events");

    let uuid_col: Column<(), Uuid> = Column::new(table, "id");
    let datetime_col: Column<(), NaiveDateTime> = Column::new(table, "starts_at");
    let date_col: Column<(), NaiveDate> = Column::new(table, "day");
    let time_col: Column<(), NaiveTime> = Column::new(table, "starts_at_time");

    let date = NaiveDate::from_ymd_opt(2024, 1, 2).expect("date");
    let time = NaiveTime::from_hms_opt(3, 4, 5).expect("time");
    let datetime = NaiveDateTime::new(date, time);
    let id = Uuid::nil();

    let compiled = Select::<()>::new(table)
        .filter(uuid_col.eq(id))
        .filter(datetime_col.eq(datetime))
        .filter(date_col.eq(date))
        .filter(time_col.eq(time))
        .compile();

    assert_eq!(
        compiled.binds,
        vec![
            Value::Uuid(id),
            Value::DateTime(datetime),
            Value::Date(date),
            Value::Time(time),
        ]
    );
}

#[test]
fn select_binds_utc_datetime() {
    let table = Table::new("events_tz");
    let starts_at_col: Column<(), DateTime<Utc>> = Column::new(table, "starts_at");
    let datetime = Utc
        .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
        .single()
        .expect("utc datetime");

    let compiled = Select::<()>::new(table)
        .filter(starts_at_col.eq(datetime))
        .compile();

    assert_eq!(compiled.binds, vec![Value::DateTimeUtc(datetime)]);
}

#[test]
fn select_supports_utc_datetime_between_and_null_filters() {
    let table = Table::new("events_tz");
    let starts_at_col: Column<(), DateTime<Utc>> = Column::new(table, "starts_at");
    let low = Utc
        .with_ymd_and_hms(2024, 1, 2, 3, 0, 0)
        .single()
        .expect("utc low");
    let high = low + Duration::minutes(30);

    let compiled = Select::<()>::new(table)
        .filter(starts_at_col.eq(None::<DateTime<Utc>>))
        .filter(starts_at_col.between(low, high))
        .compile();

    assert!(compiled.sql.contains("IS NULL"));
    assert_eq!(
        compiled.binds,
        vec![Value::DateTimeUtc(low), Value::DateTimeUtc(high)]
    );
}

#[test]
fn value_from_json() {
    let payload = json!({"name": "alice", "active": true});
    assert_eq!(Value::from(payload.clone()), Value::Json(payload));
}

#[test]
fn select_binds_json() {
    let table = Table::new("json_rows");
    let data_col: Column<(), serde_json::Value> = Column::new(table, "data");
    let payload = json!({"name": "alice", "active": true});

    let compiled = Select::<()>::new(table)
        .filter(data_col.eq(payload.clone()))
        .compile();

    assert_eq!(compiled.binds, vec![Value::Json(payload)]);
}

#[test]
fn value_from_string_array() {
    let items = vec!["a".to_string(), "b".to_string()];
    assert_eq!(Value::from(items.clone()), Value::Array(items));
}

#[test]
fn select_binds_string_array() {
    let table = Table::new("profiles");
    let tags_col: Column<(), Vec<String>> = Column::new(table, "tags");
    let items = vec!["a".to_string(), "b".to_string()];

    let compiled = Select::<()>::new(table)
        .filter(tags_col.eq(items.clone()))
        .compile();

    assert_eq!(compiled.binds, vec![Value::Array(items)]);
}
