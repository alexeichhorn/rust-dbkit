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

fn assert_f64(_: Expr<f64>) {}

fn assert_nullable_i32(_: Expr<Option<i32>>) {}

fn assert_nullable_i64(_: Expr<Option<i64>>) {}

fn assert_nullable_f32(_: Expr<Option<f32>>) {}

fn assert_nullable_f64(_: Expr<Option<f64>>) {}

fn assert_nullable_naive(_: Expr<Option<NaiveDateTime>>) {}

fn assert_nullable_utc(_: Expr<Option<DateTime<Utc>>>) {}

fn assert_interval(_: Expr<PgInterval>) {}

fn assert_nullable_interval(_: Expr<Option<PgInterval>>) {}

fn assert_bool(_: Expr<bool>) {}

fn assert_nullable_bool(_: Expr<Option<bool>>) {}

macro_rules! assert_division {
    ($output:ty, $required_lhs:expr, $nullable_lhs:expr, $required_rhs:expr, $nullable_rhs:expr) => {
        let _: Expr<$output> = $required_lhs / $required_rhs;
        let _: Expr<Option<$output>> = $nullable_lhs / $required_rhs;
        let _: Expr<Option<$output>> = $required_lhs / $nullable_rhs;
        let _: Expr<Option<$output>> = $nullable_lhs / $nullable_rhs;
    };
}

macro_rules! assert_float8_cast {
    ($required:expr, $nullable:expr) => {
        let _: Expr<f64> = $required.cast();
        let _: Expr<Option<f64>> = $nullable.cast();
    };
}

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

    // Division follows PostgreSQL's complete numeric promotion matrix and preserves NULL from either operand.
    use NullableArithmeticRow as R;
    assert_division!(i16, R::required_i16, R::nullable_i16, R::required_i16, R::nullable_i16);
    assert_division!(i32, R::required_i16, R::nullable_i16, R::required_i32, R::nullable_i32);
    assert_division!(i64, R::required_i16, R::nullable_i16, R::required_i64, R::nullable_i64);
    assert_division!(f64, R::required_i16, R::nullable_i16, R::required_f32, R::nullable_f32);
    assert_division!(f64, R::required_i16, R::nullable_i16, R::required_f64, R::nullable_f64);
    assert_division!(i32, R::required_i32, R::nullable_i32, R::required_i16, R::nullable_i16);
    assert_division!(i32, R::required_i32, R::nullable_i32, R::required_i32, R::nullable_i32);
    assert_division!(i64, R::required_i32, R::nullable_i32, R::required_i64, R::nullable_i64);
    assert_division!(f64, R::required_i32, R::nullable_i32, R::required_f32, R::nullable_f32);
    assert_division!(f64, R::required_i32, R::nullable_i32, R::required_f64, R::nullable_f64);
    assert_division!(i64, R::required_i64, R::nullable_i64, R::required_i16, R::nullable_i16);
    assert_division!(i64, R::required_i64, R::nullable_i64, R::required_i32, R::nullable_i32);
    assert_division!(i64, R::required_i64, R::nullable_i64, R::required_i64, R::nullable_i64);
    assert_division!(f64, R::required_i64, R::nullable_i64, R::required_f32, R::nullable_f32);
    assert_division!(f64, R::required_i64, R::nullable_i64, R::required_f64, R::nullable_f64);
    assert_division!(f64, R::required_f32, R::nullable_f32, R::required_i16, R::nullable_i16);
    assert_division!(f64, R::required_f32, R::nullable_f32, R::required_i32, R::nullable_i32);
    assert_division!(f64, R::required_f32, R::nullable_f32, R::required_i64, R::nullable_i64);
    assert_division!(f32, R::required_f32, R::nullable_f32, R::required_f32, R::nullable_f32);
    assert_division!(f64, R::required_f32, R::nullable_f32, R::required_f64, R::nullable_f64);
    assert_division!(f64, R::required_f64, R::nullable_f64, R::required_i16, R::nullable_i16);
    assert_division!(f64, R::required_f64, R::nullable_f64, R::required_i32, R::nullable_i32);
    assert_division!(f64, R::required_f64, R::nullable_f64, R::required_i64, R::nullable_i64);
    assert_division!(f64, R::required_f64, R::nullable_f64, R::required_f32, R::nullable_f32);
    assert_division!(f64, R::required_f64, R::nullable_f64, R::required_f64, R::nullable_f64);

    // Literals work on both sides, and computed expressions remain composable.
    assert_i32(NullableArithmeticRow::required_i32 / 2_i16);
    assert_f64(1.0_f64 / NullableArithmeticRow::required_i64);
    assert_nullable_f64(1.0_f32 / NullableArithmeticRow::nullable_i32);
    assert_f64(dbkit::func::power(2.0_f64, 3_i32) / NullableArithmeticRow::required_i64);
    assert_nullable_f64(dbkit::func::sum(NullableArithmeticRow::required_i32) / NullableArithmeticRow::required_f64);

    // Numeric columns and expressions cast to float8 while preserving nullability.
    assert_float8_cast!(R::required_i16, R::nullable_i16);
    assert_float8_cast!(R::required_i32, R::nullable_i32);
    assert_float8_cast!(R::required_i64, R::nullable_i64);
    assert_float8_cast!(R::required_f32, R::nullable_f32);
    assert_float8_cast!(R::required_f64, R::nullable_f64);
    let _: Expr<f64> = (R::required_i32 + R::required_i32).cast();
    let _: Expr<Option<f64>> = dbkit::func::sum(R::required_i32).cast();

    // Casting one integer operand enables fractional division without changing integer division semantics.
    let _: Expr<f64> = R::required_i32.cast::<f64>() / R::required_i64;
    let _: Expr<Option<f64>> = R::nullable_i32.cast::<f64>() / R::required_i64;

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

    // Numeric functions propagate NULL from either operand.
    assert_f64(dbkit::func::power(NullableArithmeticRow::required_f64, 2_f64));
    assert_nullable_f64(dbkit::func::power(NullableArithmeticRow::nullable_i16, 2_i16));
    assert_nullable_f64(dbkit::func::power(NullableArithmeticRow::nullable_i32, 2_i32));
    assert_nullable_f64(dbkit::func::power(NullableArithmeticRow::nullable_i64, 2_i64));
    assert_nullable_f64(dbkit::func::power(NullableArithmeticRow::nullable_f32, 2_f32));
    assert_nullable_f64(dbkit::func::power(NullableArithmeticRow::nullable_f64, 2_f64));
    assert_nullable_f64(dbkit::func::power(
        NullableArithmeticRow::required_i32,
        NullableArithmeticRow::nullable_i32,
    ));
    assert_nullable_f64(dbkit::func::power(
        NullableArithmeticRow::nullable_i32,
        NullableArithmeticRow::nullable_i32,
    ));

    // Interval constructors preserve the input's nullability.
    assert_interval(dbkit::interval::hours(NullableArithmeticRow::required_i32));
    assert_nullable_interval(dbkit::interval::days(NullableArithmeticRow::nullable_i32));
    assert_nullable_interval(dbkit::interval::hours(NullableArithmeticRow::nullable_i32));
    assert_nullable_interval(dbkit::interval::minutes(NullableArithmeticRow::nullable_i32));
    assert_nullable_interval(dbkit::interval::seconds(NullableArithmeticRow::nullable_i16));
    assert_nullable_interval(dbkit::interval::seconds(NullableArithmeticRow::nullable_i32));
    assert_nullable_interval(dbkit::interval::seconds(NullableArithmeticRow::nullable_i64));
    assert_nullable_interval(dbkit::interval::seconds(NullableArithmeticRow::nullable_f32));
    assert_nullable_interval(dbkit::interval::seconds(NullableArithmeticRow::nullable_f64));

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

    let nullable_numeric_expr = NullableArithmeticRow::nullable_i32 + NullableArithmeticRow::required_i32;
    assert_nullable_bool(nullable_numeric_expr.clone().eq(10_i32));
    assert_nullable_bool(nullable_numeric_expr.clone().eq(None));
    assert_nullable_bool(nullable_numeric_expr.clone().ne(10_i32));
    assert_nullable_bool(nullable_numeric_expr.clone().lt(10_i32));
    assert_nullable_bool(nullable_numeric_expr.clone().le(10_i32));
    assert_nullable_bool(nullable_numeric_expr.clone().gt(10_i32));
    assert_nullable_bool(nullable_numeric_expr.clone().ge(10_i32));
    assert_nullable_bool(nullable_numeric_expr.between(1_i32, 10_i32));

    let required_rhs_expr = NullableArithmeticRow::required_i32 + 1_i32;
    assert_bool(NullableArithmeticRow::required_i32.lt(required_rhs_expr.clone()));
    assert_bool(NullableArithmeticRow::required_i32.le(required_rhs_expr.clone()));
    assert_bool(NullableArithmeticRow::required_i32.gt(required_rhs_expr.clone()));
    assert_bool(NullableArithmeticRow::required_i32.ge(required_rhs_expr.clone()));

    let nullable_rhs_expr = NullableArithmeticRow::nullable_i32 + 1_i32;
    assert_nullable_bool(NullableArithmeticRow::required_i32.lt(nullable_rhs_expr.clone()));
    assert_nullable_bool(NullableArithmeticRow::required_i32.le(nullable_rhs_expr.clone()));
    assert_nullable_bool(NullableArithmeticRow::required_i32.gt(nullable_rhs_expr.clone()));
    assert_nullable_bool(NullableArithmeticRow::required_i32.ge(nullable_rhs_expr.clone()));
    assert_nullable_bool(NullableArithmeticRow::nullable_i32.lt(required_rhs_expr.clone()));
    assert_nullable_bool(NullableArithmeticRow::nullable_i32.ge(nullable_rhs_expr));
    assert_bool(NullableArithmeticRow::required_i32.lt(dbkit::func::coalesce(NullableArithmeticRow::nullable_i32, 0_i32)));

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

    // Timestamp literals on the left preserve NULL from interval expressions.
    let utc_literal = DateTime::from_timestamp(1_700_000_000, 0).expect("timestamp");
    let naive_literal = utc_literal.naive_utc();
    assert_nullable_naive(naive_literal + dbkit::interval::hours(NullableArithmeticRow::nullable_i32));
    assert_nullable_naive(naive_literal - dbkit::interval::hours(NullableArithmeticRow::nullable_i32));
    assert_nullable_utc(utc_literal + dbkit::interval::hours(NullableArithmeticRow::nullable_i32));
    assert_nullable_utc(utc_literal - dbkit::interval::hours(NullableArithmeticRow::nullable_i32));

    // Nullable arithmetic expressions compose with ordinary and NULL filters.
    let _query = NullableArithmeticRow::query()
        .filter((100_i32 - NullableArithmeticRow::nullable_i32).gt(50_i32))
        .filter((NullableArithmeticRow::nullable_i32 + NullableArithmeticRow::required_i32).eq(10_i32))
        .filter(NullableArithmeticRow::required_i32.lt(NullableArithmeticRow::nullable_i32 + 1_i32))
        .filter(NullableArithmeticRow::required_i32.ge(NullableArithmeticRow::nullable_i32 + 1_i32))
        .filter((NullableArithmeticRow::nullable_i32 * NullableArithmeticRow::nullable_i32).eq(None))
        .filter((NullableArithmeticRow::nullable_naive + NullableArithmeticRow::required_interval).is_not_null());
}
