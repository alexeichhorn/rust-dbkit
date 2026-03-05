//@check-pass
use dbkit::model;

#[model(table = "schedules")]
pub struct Schedule {
    #[key]
    pub id: i64,
    pub base_interval_hours: i32,
    pub backoff_minutes: Option<i32>,
}

fn main() {
    let base = dbkit::interval::hours(6_i32);
    let jitter = dbkit::interval::seconds(0.5_f64);
    let retry_hours = dbkit::interval::hours(Schedule::base_interval_hours);
    let retry_minutes = dbkit::interval::minutes(dbkit::func::coalesce(Schedule::backoff_minutes, 15_i32));

    let _query = Schedule::query()
        .select_only()
        .column_as(base.clone(), "base_interval")
        .column_as(jitter, "jitter")
        .column_as(retry_hours.clone(), "retry_hours")
        .column_as(retry_minutes.clone(), "retry_minutes")
        .order_by(dbkit::Order::asc(retry_hours))
        .order_by(dbkit::Order::asc(retry_minutes));
}
