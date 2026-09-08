//@check-pass
#![allow(non_upper_case_globals)]
use dbkit::{model, Expr, IntoExpr};

#[model(table = "numeric_samples")]
struct Sample {
    #[key]
    id: i64,
    small: i16,
    integer: i32,
    real: f32,
    double: f64,
    nullable: Option<i32>,
    parent_id: Option<i64>,
    #[belongs_to(key = parent_id, references = id)]
    parent: dbkit::BelongsTo<Sample>,
}

fn nullable<T>(_: Expr<Option<T>>) {}

macro_rules! arithmetic {
    ($literal:expr, $column:expr, $output:ty) => {
        arithmetic!($literal, $column, $output, $output);
    };
    ($literal:expr, $column:expr, $output:ty, $quotient:ty) => {
        nullable::<$output>($literal + $column);
        nullable::<$output>($literal - $column);
        nullable::<$output>($literal * $column);
        nullable::<$quotient>($literal / $column);
    };
}

macro_rules! bitwise {
    ($literal:expr, $column:expr, $output:ty) => {
        nullable::<$output>($literal & $column);
        nullable::<$output>($literal | $column);
        nullable::<$output>($literal ^ $column);
    };
}

macro_rules! shifts {
    ($literal:expr, $output:ty) => {
        nullable::<$output>($literal << Sample::parent.small);
        nullable::<$output>($literal >> Sample::parent.small);
        nullable::<$output>($literal << Sample::parent.integer);
        nullable::<$output>($literal >> Sample::parent.integer);
    };
}

fn main() {
    // Every built-in scalar type supported with ordinary columns also accepts paths.
    // SMALLINT +, -, and * widen to INTEGER; SMALLINT / SMALLINT stays SMALLINT.
    arithmetic!(12_i16, Sample::parent.small, i32, i16);
    arithmetic!(12_i32, Sample::parent.integer, i32);
    arithmetic!(12_i64, Sample::parent.id, i64);
    arithmetic!(12_f32, Sample::parent.real, f32);
    arithmetic!(12_f64, Sample::parent.double, f64);
    // Division supports mixed numeric types; +, -, and * retain their existing restrictions.
    nullable::<i32>(12_i16 / Sample::parent.integer);
    nullable::<i64>(12_i32 / Sample::parent.id);
    nullable::<f64>(12_f32 / Sample::parent.integer);
    nullable::<f64>(12_i32 / Sample::parent.real);

    bitwise!(7_i16, Sample::parent.small, i16);
    bitwise!(7_i32, Sample::parent.integer, i32);
    bitwise!(7_i64, Sample::parent.id, i64);
    bitwise!(7_i16, Sample::parent.integer, i32);
    bitwise!(7_i32, Sample::parent.id, i64);
    bitwise!(7_i64, Sample::parent.small, i64);
    shifts!(1_i16, i16);
    shifts!(1_i32, i32);
    shifts!(1_i64, i64);

    // Existing nullable fields stay singly nullable through nested paths and operators.
    let column = Sample::parent.parent.nullable;
    arithmetic!(12_i32, column, i32);
    bitwise!(7_i32, column, i32);
    nullable::<i64>(1_i64 << column);
    nullable::<i64>(64_i64 >> column);

    // Scalar, column, expression, and relation operands remain composable.
    nullable::<i32>((1_i32 + column) * (Sample::parent.integer + 2_i32));
    nullable::<i32>(Sample::integer + column);
    nullable::<i32>(Sample::integer.into_expr() - column);
    nullable::<i32>(Sample::parent.integer / column);
    nullable::<i32>(Sample::integer & column);
    nullable::<i32>(Sample::integer.into_expr() | column);
    nullable::<i32>(Sample::parent.integer ^ column);
}
