//@check-pass
use chrono::{DateTime, NaiveDateTime, Utc};
use dbkit::{model, Expr, PgInterval};

#[model(table = "nullable_arithmetic_rows")]
pub struct NullableArithmeticRow {
    #[key]
    pub id: i64,
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
    pub required_naive: NaiveDateTime,
    pub nullable_naive: Option<NaiveDateTime>,
    pub required_utc: DateTime<Utc>,
    pub nullable_utc: Option<DateTime<Utc>>,
    pub required_interval: PgInterval,
    pub nullable_interval: Option<PgInterval>,
}

fn assert_i32(_: Expr<i32>) {}

fn assert_nullable_i32(_: Expr<Option<i32>>) {}

fn assert_nullable_i64(_: Expr<Option<i64>>) {}

fn assert_nullable_f32(_: Expr<Option<f32>>) {}

fn assert_nullable_f64(_: Expr<Option<f64>>) {}

fn assert_nullable_naive(_: Expr<Option<NaiveDateTime>>) {}

fn assert_nullable_utc(_: Expr<Option<DateTime<Utc>>>) {}

fn main() {
    // Required arithmetic remains required.
    assert_i32(NullableArithmeticRow::required_i32 + NullableArithmeticRow::required_i32);

    // NULL propagates from either or both operands for every numeric operator.
    assert_nullable_i32(NullableArithmeticRow::nullable_i32 + 1_i32);
    assert_nullable_i32(NullableArithmeticRow::nullable_i32 + NullableArithmeticRow::required_i32);
    assert_nullable_i32(NullableArithmeticRow::required_i32 + NullableArithmeticRow::nullable_i32);
    assert_nullable_i32(NullableArithmeticRow::nullable_i32 + NullableArithmeticRow::nullable_i32);

    assert_nullable_i32(NullableArithmeticRow::nullable_i32 - 1_i32);
    assert_nullable_i32(NullableArithmeticRow::nullable_i32 - NullableArithmeticRow::required_i32);
    assert_nullable_i32(NullableArithmeticRow::required_i32 - NullableArithmeticRow::nullable_i32);
    assert_nullable_i32(NullableArithmeticRow::nullable_i32 - NullableArithmeticRow::nullable_i32);

    assert_nullable_i32(NullableArithmeticRow::nullable_i32 * 2_i32);
    assert_nullable_i32(NullableArithmeticRow::nullable_i32 * NullableArithmeticRow::required_i32);
    assert_nullable_i32(NullableArithmeticRow::required_i32 * NullableArithmeticRow::nullable_i32);
    assert_nullable_i32(NullableArithmeticRow::nullable_i32 * NullableArithmeticRow::nullable_i32);

    // Literals on the left preserve nullability too, including non-commutative subtraction.
    assert_i32(1 - NullableArithmeticRow::required_i32);
    assert_nullable_i32(1_i32 + NullableArithmeticRow::nullable_i32);
    assert_nullable_i32(1 - NullableArithmeticRow::nullable_i32);
    assert_nullable_i32(2_i32 * NullableArithmeticRow::nullable_i32);

    // SMALLINT arithmetic still promotes to INTEGER when either operand is nullable.
    assert_nullable_i32(NullableArithmeticRow::nullable_i16 + NullableArithmeticRow::required_i16);
    assert_nullable_i32(NullableArithmeticRow::required_i16 - NullableArithmeticRow::nullable_i16);
    assert_nullable_i32(NullableArithmeticRow::nullable_i16 * NullableArithmeticRow::nullable_i16);

    // Every other supported numeric type preserves its own output type.
    assert_nullable_i64(NullableArithmeticRow::nullable_i64 + NullableArithmeticRow::required_i64);
    assert_nullable_f32(NullableArithmeticRow::required_f32 - NullableArithmeticRow::nullable_f32);
    assert_nullable_f64(NullableArithmeticRow::nullable_f64 * NullableArithmeticRow::nullable_f64);

    // Nullable arithmetic remains nullable when expressions are nested on either side.
    assert_nullable_i32(
        (NullableArithmeticRow::nullable_i32 + NullableArithmeticRow::required_i32)
            * (NullableArithmeticRow::required_i32 - NullableArithmeticRow::required_i32),
    );
    assert_nullable_i32(
        (NullableArithmeticRow::required_i32 + NullableArithmeticRow::required_i32)
            * (NullableArithmeticRow::nullable_i32 - NullableArithmeticRow::required_i32),
    );
    assert_nullable_i32(100_i32 - (NullableArithmeticRow::nullable_i32 + NullableArithmeticRow::required_i32));

    // Timestamp arithmetic propagates NULL from the timestamp, interval, or both.
    assert_nullable_naive(NullableArithmeticRow::nullable_naive + NullableArithmeticRow::required_interval);
    assert_nullable_naive(NullableArithmeticRow::required_naive + NullableArithmeticRow::nullable_interval);
    assert_nullable_naive(NullableArithmeticRow::nullable_naive + NullableArithmeticRow::nullable_interval);
    assert_nullable_naive(NullableArithmeticRow::nullable_naive - NullableArithmeticRow::required_interval);
    assert_nullable_naive(NullableArithmeticRow::required_naive - NullableArithmeticRow::nullable_interval);
    assert_nullable_naive(NullableArithmeticRow::nullable_naive - NullableArithmeticRow::nullable_interval);

    assert_nullable_utc(NullableArithmeticRow::nullable_utc + NullableArithmeticRow::required_interval);
    assert_nullable_utc(NullableArithmeticRow::required_utc + NullableArithmeticRow::nullable_interval);
    assert_nullable_utc(NullableArithmeticRow::nullable_utc + NullableArithmeticRow::nullable_interval);
    assert_nullable_utc(NullableArithmeticRow::nullable_utc - NullableArithmeticRow::required_interval);
    assert_nullable_utc(NullableArithmeticRow::required_utc - NullableArithmeticRow::nullable_interval);
    assert_nullable_utc(NullableArithmeticRow::nullable_utc - NullableArithmeticRow::nullable_interval);

    // Nullable arithmetic expressions compose with ordinary and NULL filters.
    let _query = NullableArithmeticRow::query()
        .filter((100_i32 - NullableArithmeticRow::nullable_i32).gt(50_i32))
        .filter((NullableArithmeticRow::nullable_i32 + NullableArithmeticRow::required_i32).eq(10_i32))
        .filter((NullableArithmeticRow::nullable_i32 * NullableArithmeticRow::nullable_i32).eq(None))
        .filter((NullableArithmeticRow::nullable_naive + NullableArithmeticRow::required_interval).is_not_null());
}
