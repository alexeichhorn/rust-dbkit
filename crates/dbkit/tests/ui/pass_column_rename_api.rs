//@check-pass
#![allow(non_upper_case_globals)]

use dbkit::{model, BelongsTo, HasMany};

#[model(table = "renamed_parents")]
pub struct RenamedParent {
    #[key]
    #[autoincrement]
    pub id: i64,
    #[dbkit(column = "type")]
    pub type_: String,
    #[dbkit(column = "external_ref")]
    pub external_reference: String,
    pub label: String,
    #[has_many]
    pub children: HasMany<RenamedChild>,
}

#[model(table = "renamed_children")]
pub struct RenamedChild {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub parent_id: i64,
    #[dbkit(column = "type")]
    pub type_: String,
    #[dbkit(column = "sort_key")]
    pub rank_key: i64,
    #[belongs_to(key = parent_id, references = id)]
    pub parent: BelongsTo<RenamedParent>,
}

fn main() {
    let _renamed_column = RenamedParent::type_;
    let _renamed_external_column = RenamedParent::external_reference;
    let _renamed_child_column = RenamedChild::type_;
    let _renamed_child_rank_column = RenamedChild::rank_key;

    let _filter_sql = RenamedParent::query()
        .filter(RenamedParent::type_.eq("primary"))
        .filter(RenamedParent::external_reference.eq("ref-1"))
        .debug_sql();

    let _insert = RenamedParent::insert(RenamedParentInsert {
        type_: "primary".to_string(),
        external_reference: "ref-1".to_string(),
        label: "Example".to_string(),
    });

    let _update = RenamedParent::update()
        .set(RenamedParent::type_, "secondary")
        .set(RenamedParent::external_reference, "ref-2");

    let loaded = RenamedParentModel::<Vec<RenamedChild>> {
        id: 1,
        type_: "primary".to_string(),
        external_reference: "ref-1".to_string(),
        label: "Example".to_string(),
        children: vec![],
    };
    let _children = loaded.children_loaded();
}
