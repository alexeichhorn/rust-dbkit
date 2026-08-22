use dbkit::model;

#[model(table = "nullability_rows")]
pub struct NullabilityRow {
    #[key]
    pub id: i64,
    pub required_text: String,
    pub nullable_text: Option<String>,
    pub required_count: i32,
}

fn main() {
    let _eq_none = NullabilityRow::required_text.eq(None::<String>); //~ E0277
    let _eq_borrowed_some = NullabilityRow::required_text.eq(Some("value")); //~ E0277
    let _eq_some = NullabilityRow::required_text.eq(Some("value".to_string())); //~ E0277
    let _ne_none = NullabilityRow::required_text.ne(None::<String>); //~ E0277
    let _lt_none = NullabilityRow::required_text.lt(None::<String>); //~ E0277
    let _lt_some = NullabilityRow::required_count.lt(Some(5_i32)); //~ E0277
    let _le_none = NullabilityRow::required_count.le(None::<i32>); //~ E0277
    let _gt_some = NullabilityRow::required_count.gt(Some(5_i32)); //~ E0277
    let _ge_none = NullabilityRow::required_count.ge(None::<i32>); //~ E0277
    let _between_none = NullabilityRow::required_text.between("a", None::<String>); //~ E0277
    let _like_none = NullabilityRow::required_text.like(None::<String>); //~ E0277
    let _in_none = NullabilityRow::required_text.in_([None::<String>]); //~ E0277
    let _expression_eq_none = dbkit::func::lower(NullabilityRow::required_text).eq(None::<String>);
    //~^ E0277
}
