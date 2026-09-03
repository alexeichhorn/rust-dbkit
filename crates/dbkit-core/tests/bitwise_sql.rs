use dbkit_core::{Column, Expr, Order, Select, Table, Value};

#[derive(Debug)]
struct BitRegister;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PermissionBits(i64);

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

fn table() -> Table {
    Table::new("bit_registers")
}

fn small_value() -> Column<BitRegister, i16> {
    Column::new(table(), "small_value")
}

fn medium_value() -> Column<BitRegister, i32> {
    Column::new(table(), "medium_value")
}

fn large_value() -> Column<BitRegister, i64> {
    Column::new(table(), "large_value")
}

fn nullable_value() -> Column<BitRegister, Option<i64>> {
    Column::new(table(), "nullable_value")
}

fn shift_count() -> Column<BitRegister, i32> {
    Column::new(table(), "shift_count")
}

fn nullable_shift_count() -> Column<BitRegister, Option<i32>> {
    Column::new(table(), "nullable_shift_count")
}

fn permissions() -> Column<BitRegister, PermissionBits> {
    Column::new(table(), "permissions")
}

#[test]
fn compiles_every_bitwise_operator_with_stable_bind_order() {
    let compiled = Select::<BitRegister>::new(table())
        .select_only()
        .column_as(medium_value() & 12_i32, "anded")
        .column_as(medium_value() | 3_i32, "ored")
        .column_as(medium_value() ^ 10_i32, "xored")
        .column_as(!medium_value(), "inverted")
        .column_as(medium_value() << 4_i32, "shifted_left")
        .column_as(medium_value() >> 2_i16, "shifted_right")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT (bit_registers.medium_value & $1) AS anded, (bit_registers.medium_value | $2) AS ored, (bit_registers.medium_value # $3) AS xored, (~bit_registers.medium_value) AS inverted, (bit_registers.medium_value << $4) AS shifted_left, (bit_registers.medium_value >> $5) AS shifted_right FROM bit_registers"
    );
    assert_eq!(
        compiled.binds,
        vec![Value::I32(12), Value::I32(3), Value::I32(10), Value::I32(4), Value::I16(2)]
    );
}

#[test]
fn compiles_column_operands_and_mixed_integer_widths() {
    let small_and_medium: Expr<i32> = small_value() & medium_value();
    let medium_or_large: Expr<i64> = medium_value() | large_value();
    let large_xor_small: Expr<i64> = large_value() ^ small_value();

    let compiled = Select::<BitRegister>::new(table())
        .select_only()
        .column_as(small_and_medium, "small_and_medium")
        .column_as(medium_or_large, "medium_or_large")
        .column_as(large_xor_small, "large_xor_small")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT (bit_registers.small_value & bit_registers.medium_value) AS small_and_medium, (bit_registers.medium_value | bit_registers.large_value) AS medium_or_large, (bit_registers.large_value # bit_registers.small_value) AS large_xor_small FROM bit_registers"
    );
    assert!(compiled.binds.is_empty());
}

#[test]
fn compiles_nested_bitwise_expressions_with_explicit_parentheses() {
    let folded = !(((large_value() & 255_i64) ^ (large_value() >> 4_i32)) | 1_i64);
    let compiled = Select::<BitRegister>::new(table())
        .filter(folded.clone().ne(0_i64))
        .order_by(Order::desc(folded))
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT bit_registers.* FROM bit_registers WHERE ((~(((bit_registers.large_value & $1) # (bit_registers.large_value >> $2)) | $3)) <> $4) ORDER BY (~(((bit_registers.large_value & $1) # (bit_registers.large_value >> $2)) | $3)) DESC"
    );
    assert_eq!(compiled.binds, vec![Value::I64(255), Value::I32(4), Value::I64(1), Value::I64(0)]);
}

#[test]
fn compiles_literal_left_hand_operands() {
    let compiled = Select::<BitRegister>::new(table())
        .select_only()
        .column_as(15_i16 & small_value(), "anded")
        .column_as(8_i32 | medium_value(), "ored")
        .column_as(255_i64 ^ large_value(), "xored")
        .column_as(1_i64 << shift_count(), "shifted_left")
        .column_as(-8_i64 >> shift_count(), "shifted_right")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT ($1 & bit_registers.small_value) AS anded, ($2 | bit_registers.medium_value) AS ored, ($3 # bit_registers.large_value) AS xored, ($4 << bit_registers.shift_count) AS shifted_left, ($5 >> bit_registers.shift_count) AS shifted_right FROM bit_registers"
    );
    assert_eq!(
        compiled.binds,
        vec![Value::I16(15), Value::I32(8), Value::I64(255), Value::I64(1), Value::I64(-8)]
    );
}

#[test]
fn compiles_nullable_operands_without_changing_sql_shape() {
    let required_with_nullable: Expr<Option<i64>> = large_value() & nullable_value();
    let nullable_with_required: Expr<Option<i64>> = nullable_value() | large_value();
    let nullable_shift: Expr<Option<i64>> = nullable_value() << nullable_shift_count();
    let nullable_not: Expr<Option<i64>> = !nullable_value();

    let compiled = Select::<BitRegister>::new(table())
        .select_only()
        .column_as(required_with_nullable, "required_with_nullable")
        .column_as(nullable_with_required, "nullable_with_required")
        .column_as(nullable_shift, "nullable_shift")
        .column_as(nullable_not, "nullable_not")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT (bit_registers.large_value & bit_registers.nullable_value) AS required_with_nullable, (bit_registers.nullable_value | bit_registers.large_value) AS nullable_with_required, (bit_registers.nullable_value << bit_registers.nullable_shift_count) AS nullable_shift, (~bit_registers.nullable_value) AS nullable_not FROM bit_registers"
    );
    assert!(compiled.binds.is_empty());
}

#[test]
fn compiles_custom_integer_backed_type_without_exposing_its_storage_type() {
    let required = PermissionBits(0b0010);
    let compiled = Select::<BitRegister>::new(table())
        .filter((permissions() & required).ne(PermissionBits(0)))
        .select_only()
        .column_as(permissions() | PermissionBits(0b1000), "effective_permissions")
        .column_as(!permissions(), "inverted_permissions")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT (bit_registers.permissions | $1) AS effective_permissions, (~bit_registers.permissions) AS inverted_permissions FROM bit_registers WHERE ((bit_registers.permissions & $2) <> $3)"
    );
    assert_eq!(compiled.binds, vec![Value::I64(8), Value::I64(2), Value::I64(0)]);
}
