#[allow(non_upper_case_globals)]
use dbkit::model;

#[model(table = "lookup_items")]
pub struct LookupItem {
    #[key]
    pub id: i64,
    pub namespace: String,
    pub optional_namespace: Option<String>,
    pub external_key: String,
    pub locale: String,
}

fn main() {
    let _query =
        LookupItem::query().filter(dbkit::row((LookupItem::namespace, LookupItem::external_key)).in_([(1_i64, "alpha"), (2_i64, "beta")])); //~ ERROR: ColumnValue

    let _optional_value_for_required_column =
        dbkit::row((LookupItem::namespace, LookupItem::external_key)).in_([(Some("public".to_string()), "alpha")]); //~ ERROR: ColumnValue

    let _wrong_optional_base_type = dbkit::row((LookupItem::optional_namespace, LookupItem::external_key)).in_([(Some(1_i64), "alpha")]);
    //~^ ERROR: ColumnValue
}
