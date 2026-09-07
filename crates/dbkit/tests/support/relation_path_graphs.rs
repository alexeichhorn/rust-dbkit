#![allow(dead_code, non_upper_case_globals)]

use super::relation_paths::Member;
use dbkit::model;

#[model(table = "path_assignments")]
pub struct Assignment {
    #[key]
    pub id: i64,
    pub first_id: Option<i64>,
    pub second_code: Option<String>,
    #[belongs_to(key = first_id, references = id)]
    pub first: dbkit::BelongsTo<Member>,
    #[belongs_to(key = second_code, references = code)]
    pub second: dbkit::BelongsTo<Member>,
}

#[model(table = "path_nodes")]
pub struct Node {
    #[key]
    pub id: i64,
    pub parent_id: Option<i64>,
    pub label: String,
    #[belongs_to(key = parent_id, references = id)]
    pub parent: dbkit::BelongsTo<Node>,
}
