use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use dbkit::executor::build_arguments;
use dbkit::Value;
use serde_json::json;
use uuid::Uuid;

#[test]
fn build_arguments_accepts_uuid_datetime_date_time() {
    let date = NaiveDate::from_ymd_opt(2024, 1, 2).expect("date");
    let time = NaiveTime::from_hms_opt(3, 4, 5).expect("time");
    let datetime = NaiveDateTime::new(date, time);
    let id = Uuid::nil();

    let values = vec![
        Value::from(id),
        Value::from(datetime),
        Value::from(date),
        Value::from(time),
    ];

    let result = build_arguments(&values);
    assert!(result.is_ok());
}

#[test]
fn build_arguments_accepts_utc_datetime() {
    let datetime: DateTime<Utc> = Utc
        .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
        .single()
        .expect("utc datetime");

    let values = vec![Value::from(datetime)];
    let result = build_arguments(&values);
    assert!(result.is_ok());
}

#[test]
fn build_arguments_accepts_json() {
    let payload = json!({"name": "alice", "active": true});
    let values = vec![Value::from(payload)];
    let result = build_arguments(&values);
    assert!(result.is_ok());
}
