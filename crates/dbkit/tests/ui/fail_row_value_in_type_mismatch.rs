#[allow(non_upper_case_globals)]
use dbkit::model;

#[model(table = "lookup_items")]
pub struct LookupItem {
    #[key]
    pub id: i64,
    pub namespace: String,
    pub external_key: String,
    pub locale: String,
}

fn main() {
    let _query =
        LookupItem::query().filter(dbkit::row((LookupItem::namespace, LookupItem::external_key)).in_([(1_i64, "alpha"), (2_i64, "beta")])); //~ ERROR: ColumnValue
}
