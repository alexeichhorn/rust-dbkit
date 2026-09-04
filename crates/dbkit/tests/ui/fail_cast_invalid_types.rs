use chrono::NaiveDateTime;
use dbkit::{model, PgInterval};
use uuid::Uuid;

#[model(table = "cast_rows")]
pub struct CastRow {
    #[key]
    pub id: i64,
    pub enabled: bool,
    pub recorded_at: NaiveDateTime,
    pub external_id: Uuid,
    pub elapsed: PgInterval,
}

fn main() {
    let _timestamp_to_float = CastRow::recorded_at.cast::<f64>(); //~ ERROR: cast
    let _bool_to_bigint = CastRow::enabled.cast::<i64>(); //~ ERROR: cast
    let _uuid_to_timestamp = CastRow::external_id.cast::<NaiveDateTime>(); //~ ERROR: cast
    let _interval_to_integer = CastRow::elapsed.cast::<i32>(); //~ ERROR: cast
}
