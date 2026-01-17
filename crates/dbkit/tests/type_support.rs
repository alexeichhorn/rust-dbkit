use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::executor::build_arguments;
use dbkit::Value;
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
