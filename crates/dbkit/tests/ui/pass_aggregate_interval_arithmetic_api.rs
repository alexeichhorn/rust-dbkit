//@check-pass
use chrono::{DateTime, NaiveDateTime, Utc};
use dbkit::{model, Expr, PgInterval};

#[model(table = "durations")]
pub struct Duration {
    #[key]
    pub id: i64,
    pub elapsed: PgInterval,
    pub nullable_elapsed: Option<PgInterval>,
}

fn main() {
    let utc = DateTime::from_timestamp(1_700_000_000, 0).expect("utc");
    let naive = utc.naive_utc();

    let _interval_sum: dbkit::AggregateExpr<Option<PgInterval>> = dbkit::func::sum(Duration::elapsed);
    let _nullable_interval_sum: dbkit::AggregateExpr<Option<PgInterval>> = dbkit::func::sum(Duration::nullable_elapsed);
    let _naive_add: Expr<Option<NaiveDateTime>> = naive + dbkit::func::sum(Duration::elapsed);
    let _naive_sub: Expr<Option<NaiveDateTime>> = naive - dbkit::func::sum(Duration::elapsed);
    let _utc_add: Expr<Option<DateTime<Utc>>> = utc + dbkit::func::sum(Duration::elapsed);
    let _utc_sub: Expr<Option<DateTime<Utc>>> = utc - dbkit::func::sum(Duration::elapsed);
}
