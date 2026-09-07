#[path = "../support/relation_paths.rs"]
mod relation_paths;

use relation_paths::Record;

fn main() {
    let _ = Record::owner.enabled.eq(true);
    let _ = Record::owner.enabled.eq("enabled"); //~ E0277
    let _ = Record::owner.score.gt("high"); //~ E0277
    let _ = dbkit::func::lower(Record::owner.score); //~ E0277
}
