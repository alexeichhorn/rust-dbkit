//@check-pass
use chrono::{DateTime, NaiveDateTime, Utc};
use dbkit::{model, Expr, PgInterval};

#[model(table = "durations")]
pub struct Duration {
    #[key]
    pub id: i64,
    pub elapsed: PgInterval,
}

fn main() {
    let utc = DateTime::from_timestamp(1_700_000_000, 0).expect("utc");
    let naive = utc.naive_utc();

    let _naive_add: Expr<NaiveDateTime> = naive + dbkit::func::sum(Duration::elapsed);
    let _naive_sub: Expr<NaiveDateTime> = naive - dbkit::func::sum(Duration::elapsed);
    let _utc_add: Expr<DateTime<Utc>> = utc + dbkit::func::sum(Duration::elapsed);
    let _utc_sub: Expr<DateTime<Utc>> = utc - dbkit::func::sum(Duration::elapsed);
}
