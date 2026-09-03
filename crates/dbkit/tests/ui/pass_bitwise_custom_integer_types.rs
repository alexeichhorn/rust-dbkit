//@check-pass
use dbkit::{Column, Expr, Table, Value};

// Into<i64> proves integer compatibility; Into<Value> keeps the real SQL width.
#[derive(Debug, Clone, Copy)]
struct SmallBits(i16);

#[derive(Debug, Clone, Copy)]
struct MediumBits(i32);

#[derive(Debug, Clone, Copy)]
struct LargeBits(i64);

macro_rules! impl_integer_bits {
    ($type:ty, $value_variant:ident) => {
        impl From<$type> for i64 {
            fn from(value: $type) -> Self {
                value.0.into()
            }
        }

        impl From<$type> for Value {
            fn from(value: $type) -> Self {
                Value::$value_variant(value.0)
            }
        }
    };
}

impl_integer_bits!(SmallBits, I16);
impl_integer_bits!(MediumBits, I32);
impl_integer_bits!(LargeBits, I64);

struct BitRegister;

fn small_bits() -> Column<BitRegister, SmallBits> {
    Column::new(Table::new("bit_registers"), "small_bits")
}

fn medium_bits() -> Column<BitRegister, MediumBits> {
    Column::new(Table::new("bit_registers"), "medium_bits")
}

fn large_bits() -> Column<BitRegister, LargeBits> {
    Column::new(Table::new("bit_registers"), "large_bits")
}

fn small_shift_count() -> Column<BitRegister, i16> {
    Column::new(Table::new("bit_registers"), "small_shift_count")
}

fn assert_small(_: Expr<SmallBits>) {}
fn assert_medium(_: Expr<MediumBits>) {}
fn assert_large(_: Expr<LargeBits>) {}

fn main() {
    assert_small(small_bits() & SmallBits(1));
    assert_small(small_bits() | SmallBits(2));
    assert_small(small_bits() ^ SmallBits(3));
    assert_small(!small_bits());
    assert_small(small_bits() << 1_i32);
    assert_small(small_bits() >> small_shift_count());

    assert_medium(medium_bits() & MediumBits(1));
    assert_medium(medium_bits() | MediumBits(2));
    assert_medium(medium_bits() ^ MediumBits(3));
    assert_medium(!medium_bits());
    assert_medium(medium_bits() << 1_i32);
    assert_medium(medium_bits() >> small_shift_count());

    assert_large(large_bits() & LargeBits(1));
    assert_large(large_bits() | LargeBits(2));
    assert_large(large_bits() ^ LargeBits(3));
    assert_large(!large_bits());
    assert_large(large_bits() << 1_i32);
    assert_large(large_bits() >> small_shift_count());

    let _filter = (large_bits() & LargeBits(1)).ne(LargeBits(0));
}
