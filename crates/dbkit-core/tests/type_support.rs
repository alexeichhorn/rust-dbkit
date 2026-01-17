use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dbkit_core::{Column, Select, Table, Value};
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
