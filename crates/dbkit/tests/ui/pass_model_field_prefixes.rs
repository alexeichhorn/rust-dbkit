//@check-pass
#![allow(non_upper_case_globals)]
use dbkit::model;

#[model(table = "accepted_fields")]
struct Accepted {
    #[key]
    id: i64,
    dbkit_value: i32,
    _dbkit_value: i32,
    __dbkit: i32,
    __dbkitx_value: i32,
    // Only Rust field names are reserved; existing database column names stay usable.
    #[dbkit(column = "__dbkit_marker")]
    label: String,
}

#[model(table = "sources")]
struct Source {
    #[key]
    id: i64,
    target_id: i64,
    #[belongs_to(key = target_id, references = id)]
    target: dbkit::BelongsTo<Accepted>,
}

fn main() {
    let _ = Accepted::query()
        .filter(Accepted::dbkit_value.eq(1_i32))
        .filter(Accepted::_dbkit_value.eq(2_i32))
        .filter(Accepted::__dbkit.eq(3_i32))
        .filter(Accepted::__dbkitx_value.eq(4_i32))
        .filter(Accepted::label.eq("value"));
    let _ = Source::query()
        .filter(Source::target.__dbkit.eq(3_i32))
        .filter(Source::target.label.eq("value"));
}
