//@check-pass
#![allow(non_upper_case_globals)]

use dbkit::model;

#[model(table = "parents")]
struct Parent {
    #[key]
    id: i64,
    #[has_many]
    from_group: dbkit::HasMany<FromGroupChild>,
    #[has_many]
    middle: dbkit::HasMany<MiddleChild>,
    #[has_many]
    group: dbkit::HasMany<GroupChild>,
}

#[model(table = "from_group_children")]
struct FromGroupChild {
    #[key]
    id: i64,
    parent_id: i64,
    #[belongs_to(key = parent_id, references = id)]
    parent: dbkit::BelongsTo<Parent>,
}

#[model(table = "middle_children")]
struct MiddleChild {
    #[key]
    id: i64,
    parent_id: i64,
    #[belongs_to(key = parent_id, references = id)]
    parent: dbkit::BelongsTo<Parent>,
}

#[model(table = "group_children")]
struct GroupChild {
    #[key]
    id: i64,
    parent_id: i64,
    #[belongs_to(key = parent_id, references = id)]
    parent: dbkit::BelongsTo<Parent>,
}

fn main() {}
