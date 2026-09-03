//@check-pass
use dbkit::{model, Column, Expr, Order, Table, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PermissionBits(i64);

impl PermissionBits {
    const READ: Self = Self(1 << 0);
    const WRITE: Self = Self(1 << 1);
}

impl From<PermissionBits> for Value {
    fn from(value: PermissionBits) -> Self {
        Self::I64(value.0)
    }
}

impl From<PermissionBits> for i64 {
    fn from(value: PermissionBits) -> Self {
        value.0
    }
}

#[model(table = "bit_registers")]
struct BitRegister {
    #[key]
    id: i64,
    small_value: i16,
    nullable_small: Option<i16>,
    medium_value: i32,
    nullable_medium: Option<i32>,
    large_value: i64,
    nullable_large: Option<i64>,
    small_shift_count: i16,
    nullable_small_shift_count: Option<i16>,
    shift_count: i32,
    nullable_shift_count: Option<i32>,
}

fn permissions() -> Column<BitRegister, PermissionBits> {
    Column::new(Table::new("bit_registers"), "permissions")
}

fn nullable_permissions() -> Column<BitRegister, Option<PermissionBits>> {
    Column::new(Table::new("bit_registers"), "nullable_permissions")
}

fn assert_i16(_: Expr<i16>) {}
fn assert_i32(_: Expr<i32>) {}
fn assert_i64(_: Expr<i64>) {}
fn assert_permissions(_: Expr<PermissionBits>) {}
fn assert_nullable_i16(_: Expr<Option<i16>>) {}
fn assert_nullable_i32(_: Expr<Option<i32>>) {}
fn assert_nullable_i64(_: Expr<Option<i64>>) {}
fn assert_nullable_permissions(_: Expr<Option<PermissionBits>>) {}
fn assert_bool(_: Expr<bool>) {}
fn assert_nullable_bool(_: Expr<Option<bool>>) {}

fn main() {
    // Same-width binary operators preserve the integer type.
    assert_i16(BitRegister::small_value & 1_i16);
    assert_i16(BitRegister::small_value | BitRegister::small_value);
    assert_i16(BitRegister::small_value ^ 1_i16);
    assert_i32(BitRegister::medium_value & 1_i32);
    assert_i32(BitRegister::medium_value | BitRegister::medium_value);
    assert_i32(BitRegister::medium_value ^ 1_i32);
    assert_i64(BitRegister::large_value & 1_i64);
    assert_i64(BitRegister::large_value | BitRegister::large_value);
    assert_i64(BitRegister::large_value ^ 1_i64);

    // Mixed-width operations use the wider PostgreSQL integer type.
    assert_i32(BitRegister::small_value & BitRegister::medium_value);
    assert_i32(BitRegister::medium_value | BitRegister::small_value);
    assert_i64(BitRegister::small_value ^ BitRegister::large_value);
    assert_i64(BitRegister::large_value & BitRegister::medium_value);
    assert_i32(BitRegister::small_value | 1_i32);
    assert_i64(BitRegister::medium_value ^ 1_i64);

    // Built-in literals work on the left too.
    assert_i16(1_i16 & BitRegister::small_value);
    assert_i32(1_i16 | BitRegister::medium_value);
    assert_i64(1_i32 ^ BitRegister::large_value);
    assert_i64(1_i64 & BitRegister::small_value);

    // NOT and shifts preserve the left operand type. SMALLINT shift counts widen to INTEGER.
    assert_i16(!BitRegister::small_value);
    assert_i32(!BitRegister::medium_value);
    assert_i64(!BitRegister::large_value);
    assert_i16(BitRegister::small_value << 1_i32);
    assert_i64(BitRegister::large_value << 1_i16);
    assert_i64(BitRegister::large_value >> BitRegister::small_shift_count);
    assert_i32(BitRegister::medium_value >> BitRegister::shift_count);
    assert_i64(BitRegister::large_value << BitRegister::shift_count);
    assert_i64(1_i64 << BitRegister::shift_count);
    assert_i32(16_i32 >> BitRegister::shift_count);

    // Any nullable operand makes the result nullable.
    assert_nullable_i16(BitRegister::nullable_small & BitRegister::small_value);
    assert_nullable_i16(BitRegister::small_value | BitRegister::nullable_small);
    assert_nullable_i32(BitRegister::nullable_small ^ BitRegister::medium_value);
    assert_nullable_i32(BitRegister::small_value & BitRegister::nullable_medium);
    assert_nullable_i64(BitRegister::nullable_medium | BitRegister::large_value);
    assert_nullable_i64(BitRegister::large_value ^ BitRegister::nullable_medium);
    assert_nullable_i64(BitRegister::nullable_large & 1_i64);
    assert_nullable_i64(BitRegister::nullable_large | BitRegister::nullable_large);
    assert_nullable_i64(!BitRegister::nullable_large);
    assert_nullable_i64(BitRegister::nullable_large << BitRegister::shift_count);
    assert_nullable_i64(BitRegister::large_value >> BitRegister::nullable_small_shift_count);
    assert_nullable_i64(BitRegister::large_value >> BitRegister::nullable_shift_count);
    assert_nullable_i64(BitRegister::nullable_large << BitRegister::nullable_shift_count);
    assert_nullable_i64(1_i64 << BitRegister::nullable_shift_count);
    assert_i64(1_i64 << BitRegister::small_shift_count);

    // Expressions can be nested and reused as ordinary typed expressions.
    let folded = !(((BitRegister::large_value & 255_i64) ^ (BitRegister::large_value >> 4_i32)) | 1_i64);
    assert_i64(folded.clone());
    assert_bool(folded.clone().ne(0_i64));

    let nullable_folded = (BitRegister::nullable_large & 255_i64) ^ (BitRegister::large_value >> 4_i32);
    assert_nullable_i64(nullable_folded.clone());
    assert_nullable_bool(nullable_folded.ne(0_i64));

    // An integer-convertible custom type keeps its domain type throughout the expression.
    assert_permissions(permissions() & PermissionBits::READ);
    assert_permissions(permissions() | PermissionBits::WRITE);
    assert_permissions(permissions() ^ PermissionBits::READ);
    assert_permissions(!permissions());
    assert_permissions(permissions() << 2_i32);
    assert_permissions(permissions() >> BitRegister::shift_count);
    assert_nullable_permissions(nullable_permissions() & PermissionBits::READ);
    assert_nullable_permissions(permissions() | nullable_permissions());
    assert_nullable_permissions(!nullable_permissions());
    assert_nullable_permissions(nullable_permissions() << BitRegister::nullable_shift_count);

    let _query = BitRegister::query()
        .filter((permissions() & PermissionBits::READ).ne(PermissionBits(0)))
        .filter((BitRegister::nullable_large & 1_i64).eq(1_i64))
        .select_only()
        .column_as(BitRegister::medium_value | 8_i32, "enabled_value")
        .column_as(!permissions(), "inverted_permissions")
        .column_as(BitRegister::large_value << BitRegister::shift_count, "shifted_value")
        .order_by(Order::desc(BitRegister::large_value ^ 255_i64))
        .debug_sql();
}
