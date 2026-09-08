use dbkit::model;

#[model(table = "marker_fields")]
struct MarkerField {
    #[key]
    id: i64,
    __dbkit_marker: String, //~ ERROR: field names starting with `__dbkit_` are reserved for dbkit internals
}

#[model(table = "future_fields")]
struct FutureField {
    #[key]
    id: i64,
    __dbkit_future: bool, //~ ERROR: field names starting with `__dbkit_` are reserved for dbkit internals
}

#[model(table = "renamed_fields")]
struct RenamedField {
    #[key]
    id: i64,
    #[dbkit(column = "public_label")]
    __dbkit_label: String, //~ ERROR: field names starting with `__dbkit_` are reserved for dbkit internals
}

#[model(table = "raw_fields")]
struct RawField {
    #[key]
    id: i64,
    r#__dbkit_raw: String, //~ ERROR: field names starting with `__dbkit_` are reserved for dbkit internals
}

#[model(table = "reserved_keys")]
struct ReservedKey {
    #[key]
    __dbkit_id: i64, //~ ERROR: field names starting with `__dbkit_` are reserved for dbkit internals
}

#[model(table = "targets")]
struct Target {
    #[key]
    id: i64,
}

#[model(table = "reserved_relations")]
struct ReservedRelation {
    #[key]
    id: i64,
    owner_id: i64,
    #[belongs_to(key = owner_id, references = id)]
    __dbkit_owner: dbkit::BelongsTo<Target>, //~ ERROR: field names starting with `__dbkit_` are reserved for dbkit internals
}

fn main() {}
