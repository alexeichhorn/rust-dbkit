#![allow(dead_code, non_upper_case_globals)]

use dbkit::model;

#[model(table = "path_organizations")]
pub struct Organization {
    #[key]
    pub id: i64,
    pub label: String,
}

#[model(table = "path_members")]
pub struct Member {
    #[key]
    pub id: i64,
    pub organization_id: i64,
    pub label: String,
    pub enabled: bool,
    pub score: i32,
    pub note: Option<String>,
    #[dbkit(column = "external_ref")]
    pub code: String,
    #[belongs_to(key = organization_id, references = id)]
    pub organization: dbkit::BelongsTo<Organization>,
}

#[model(table = "path_records")]
pub struct Record {
    #[key]
    pub id: i64,
    pub owner_id: Option<i64>,
    pub label: String,
    pub enabled: bool,
    #[belongs_to(key = owner_id, references = id)]
    pub owner: dbkit::BelongsTo<Member>,
}
