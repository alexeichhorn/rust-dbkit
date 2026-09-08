#[path = "../support/relation_paths.rs"]
mod relation_paths;

use dbkit::{Expr, IntoExpr};
use relation_paths::Record;

fn main() {
    let _: Expr<Option<bool>> = Record::owner.enabled.eq(true);
    let _: Expr<bool> = Record::owner.enabled.eq(true); //~ E0308
    let _: Expr<String> = Record::owner.label.into_expr(); //~ E0308
    let _: Expr<Option<Option<String>>> = Record::owner.note.into_expr(); //~ E0308
}
