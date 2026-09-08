#[path = "../support/relation_paths.rs"]
mod relation_paths;
use relation_paths::Record;

fn main() {
    // Forwarding an operator must retain the ordinary column's operand restrictions.
    let _ = 1_i32 + Record::owner.label; //~ E0277
    let _ = 1_i32 - Record::owner.enabled; //~ E0277
    let _ = 1_u32 + Record::owner.score; //~ E0277
    let _ = 1_i16 + Record::owner.score; //~ E0277
    let _ = 1_f64 * Record::owner.score; //~ E0277
    let _ = 1_f64 & Record::owner.score; //~ E0369
    let _ = true | Record::owner.enabled; //~ E0277
    let _ = 1_i32 ^ Record::owner.note; //~ E0277

    // PostgreSQL shift counts accept SMALLINT/INTEGER, not BIGINT or text.
    let _ = 1_i32 << Record::owner.id; //~ E0277
    let _ = 1_i64 >> Record::owner.organization.id; //~ E0277
    let _ = 1_i32 << Record::owner.label; //~ E0277

    // A scalar on the left cannot remove the path's outer-join nullability.
    let _: dbkit::Expr<i32> = 1_i32 + Record::owner.score; //~ E0308
}
