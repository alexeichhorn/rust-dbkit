//@check-pass
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use dbkit::{model, Expr, PgInterval};
use uuid::Uuid;

#[model(table = "cast_rows")]
pub struct CastRow {
    #[key]
    pub id: i64,
    pub required_bool: bool,
    pub nullable_bool: Option<bool>,
    pub required_i16: i16,
    pub nullable_i16: Option<i16>,
    pub required_i32: i32,
    pub nullable_i32: Option<i32>,
    pub required_i64: i64,
    pub nullable_i64: Option<i64>,
    pub required_f32: f32,
    pub nullable_f32: Option<f32>,
    pub required_f64: f64,
    pub nullable_f64: Option<f64>,
    pub required_text: String,
    pub nullable_text: Option<String>,
    pub required_uuid: Uuid,
    pub nullable_uuid: Option<Uuid>,
    pub required_timestamp: NaiveDateTime,
    pub nullable_timestamp: Option<NaiveDateTime>,
    pub required_timestamptz: DateTime<Utc>,
    pub nullable_timestamptz: Option<DateTime<Utc>>,
    pub required_date: NaiveDate,
    pub nullable_date: Option<NaiveDate>,
    pub required_time: NaiveTime,
    pub nullable_time: Option<NaiveTime>,
    pub required_interval: PgInterval,
    pub nullable_interval: Option<PgInterval>,
}

macro_rules! assert_cast {
    ($target:ty, $required:expr, $nullable:expr) => {
        let _: Expr<$target> = $required.cast();
        let _: Expr<Option<$target>> = $nullable.cast();
    };
}

macro_rules! assert_numeric_casts {
    ($required:expr, $nullable:expr) => {
        assert_cast!(i16, $required, $nullable);
        assert_cast!(i32, $required, $nullable);
        assert_cast!(i64, $required, $nullable);
        assert_cast!(f32, $required, $nullable);
        assert_cast!(f64, $required, $nullable);
    };
}

fn main() {
    use CastRow as R;

    // PostgreSQL supports explicit casts between every built-in numeric type.
    assert_numeric_casts!(R::required_i16, R::nullable_i16);
    assert_numeric_casts!(R::required_i32, R::nullable_i32);
    assert_numeric_casts!(R::required_i64, R::nullable_i64);
    assert_numeric_casts!(R::required_f32, R::nullable_f32);
    assert_numeric_casts!(R::required_f64, R::nullable_f64);

    // PostgreSQL's boolean/integer cast exists specifically for INTEGER.
    assert_cast!(bool, R::required_i32, R::nullable_i32);
    assert_cast!(i32, R::required_bool, R::nullable_bool);

    // Explicit text input casts use each target type's PostgreSQL input function.
    assert_cast!(bool, R::required_text, R::nullable_text);
    assert_numeric_casts!(R::required_text, R::nullable_text);
    assert_cast!(Uuid, R::required_text, R::nullable_text);
    assert_cast!(NaiveDateTime, R::required_text, R::nullable_text);
    assert_cast!(DateTime<Utc>, R::required_text, R::nullable_text);
    assert_cast!(NaiveDate, R::required_text, R::nullable_text);
    assert_cast!(NaiveTime, R::required_text, R::nullable_text);
    assert_cast!(PgInterval, R::required_text, R::nullable_text);

    // Every unambiguous scalar type can use PostgreSQL's output conversion to TEXT.
    assert_cast!(String, R::required_bool, R::nullable_bool);
    assert_cast!(String, R::required_i16, R::nullable_i16);
    assert_cast!(String, R::required_i32, R::nullable_i32);
    assert_cast!(String, R::required_i64, R::nullable_i64);
    assert_cast!(String, R::required_f32, R::nullable_f32);
    assert_cast!(String, R::required_f64, R::nullable_f64);
    assert_cast!(String, R::required_text, R::nullable_text);
    assert_cast!(String, R::required_uuid, R::nullable_uuid);
    assert_cast!(String, R::required_timestamp, R::nullable_timestamp);
    assert_cast!(String, R::required_timestamptz, R::nullable_timestamptz);
    assert_cast!(String, R::required_date, R::nullable_date);
    assert_cast!(String, R::required_time, R::nullable_time);
    assert_cast!(String, R::required_interval, R::nullable_interval);

    // Direct temporal casts follow PostgreSQL 16's built-in cast catalog.
    assert_cast!(NaiveDateTime, R::required_date, R::nullable_date);
    assert_cast!(DateTime<Utc>, R::required_date, R::nullable_date);
    assert_cast!(PgInterval, R::required_time, R::nullable_time);
    assert_cast!(NaiveDate, R::required_timestamp, R::nullable_timestamp);
    assert_cast!(NaiveTime, R::required_timestamp, R::nullable_timestamp);
    assert_cast!(DateTime<Utc>, R::required_timestamp, R::nullable_timestamp);
    assert_cast!(NaiveDate, R::required_timestamptz, R::nullable_timestamptz);
    assert_cast!(NaiveTime, R::required_timestamptz, R::nullable_timestamptz);
    assert_cast!(NaiveDateTime, R::required_timestamptz, R::nullable_timestamptz);
    assert_cast!(NaiveTime, R::required_interval, R::nullable_interval);
}
