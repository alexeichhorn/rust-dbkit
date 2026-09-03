use dbkit::model;

#[model(table = "bit_registers")]
struct BitRegister {
    #[key]
    id: i64,
    value: i64,
    large_shift: i64,
}

fn main() {
    // PostgreSQL does not implicitly narrow a BIGINT literal to its INTEGER shift-count type.
    let _large_literal = BitRegister::value >> 2_i64; //~ E0277

    // The same restriction applies when the BIGINT count comes from a column.
    let _large_column = BitRegister::value >> BitRegister::large_shift; //~ E0277
}
