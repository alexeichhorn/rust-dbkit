//@check-pass
#![allow(non_upper_case_globals)]

use dbkit::model;

struct From;
struct Into;

#[model(table = "first_children")]
struct FirstChild {
    #[key]
    id: i64,
}

#[model(table = "second_children")]
struct SecondChild {
    #[key]
    id: i64,
}

#[model(table = "parents")]
struct Parent {
    #[key]
    id: i64,
    first_id: i64,
    second_id: i64,
    #[belongs_to(key = first_id, references = id)]
    first: dbkit::BelongsTo<FirstChild>,
    #[belongs_to(key = second_id, references = id)]
    second: dbkit::BelongsTo<SecondChild>,
}

fn main() {}
