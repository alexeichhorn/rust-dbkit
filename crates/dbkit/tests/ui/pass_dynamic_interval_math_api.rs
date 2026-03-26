//@check-pass
use chrono::NaiveDateTime;
use dbkit::{func, interval, model, Expr, Order};

#[model(table = "work_runs")]
pub struct WorkRun {
    #[key]
    pub id: i64,
    pub status: String,
    pub attempts: i32,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

fn main() {
    let now = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("now")
        .naive_utc();

    let retry_exponent: Expr<i32> = func::least(func::greatest(WorkRun::attempts - 1_i32, 0_i32), 10_i32);
    let retry_seconds: Expr<f64> =
        func::least(3_600.0_f64, func::power(2.0_f64, retry_exponent.clone()) * 60.0_f64);
    let _stale_window = interval::seconds(300_i64);
    let _retry_window = interval::seconds(retry_seconds.clone());

    let _query = WorkRun::query()
        .filter(
            WorkRun::status
                .eq("pending")
                .or(
                    WorkRun::status
                        .eq("running")
                        .and(WorkRun::updated_at.le(now - interval::seconds(300_i64))),
                )
                .or(
                    WorkRun::status
                        .eq("failed")
                        .and(WorkRun::updated_at.le(now - interval::seconds(retry_seconds))),
                ),
        )
        .order_by(Order::asc(WorkRun::attempts))
        .order_by(Order::asc(WorkRun::created_at))
        .for_update()
        .skip_locked()
        .limit(1)
        .debug_sql();
}
