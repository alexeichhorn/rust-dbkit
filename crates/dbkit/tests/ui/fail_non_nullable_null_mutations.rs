use dbkit::model;

#[model(table = "nullability_rows")]
pub struct NullabilityRow {
    #[key]
    pub id: i64,
    pub required_text: String,
    pub nullable_text: Option<String>,
}

fn main() {
    let _insert_none = dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE).value(NullabilityRow::required_text, None::<String>); //~ E0277
    let _insert_some =
        dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE).value(NullabilityRow::required_text, Some("value".to_string())); //~ E0277
    let _insert_row_none =
        dbkit::Insert::<NullabilityRow>::new(NullabilityRow::TABLE).row(|row| row.value(NullabilityRow::required_text, None::<String>)); //~ E0277
    let _update_none = NullabilityRow::update().set(NullabilityRow::required_text, None::<String>); //~ E0277
    let _update_borrowed_some = NullabilityRow::update().set(NullabilityRow::required_text, Some("value")); //~ E0277
    let _update_some = NullabilityRow::update().set(NullabilityRow::required_text, Some("value".to_string())); //~ E0277

    let mut active = NullabilityRow::new_active();
    active.required_text = None::<String>.into(); //~ E0277
    active.required_text.set_null(); //~ E0599
}
