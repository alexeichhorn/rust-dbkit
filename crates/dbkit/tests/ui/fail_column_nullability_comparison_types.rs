use dbkit::model;

#[model(table = "nullability_rows")]
pub struct NullabilityRow {
    #[key]
    pub id: i64,
    pub required_text: String,
    pub nullable_text: Option<String>,
    pub required_count: i32,
    pub nullable_count: Option<i32>,
}

fn main() {
    let _required_to_nullable_base_mismatch = NullabilityRow::required_text.eq_col(NullabilityRow::nullable_count); //~ E0277
    let _nullable_to_required_base_mismatch = NullabilityRow::nullable_text.ne_col(NullabilityRow::required_count); //~ E0277
    let _ordered_base_mismatch = NullabilityRow::required_count.lt_col(NullabilityRow::nullable_text); //~ E0277
    let _expression_base_mismatch = dbkit::func::lower(NullabilityRow::nullable_text).eq_col(NullabilityRow::required_count);
    //~^ E0277
}
