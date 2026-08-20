use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub label: String,
    pub attempts: i32,
    pub nullable_attempts: Option<i32>,
}

// TODO(#32): Replace this once generated columns preserve nullability.
fn nullable_attempts() -> dbkit::Column<Metric, Option<i32>> {
    dbkit::Column::new(Metric::TABLE, "nullable_attempts")
}

fn main() {
    let _non_text_unary = dbkit::func::title_case(Metric::attempts); //~ E0277
    let _non_text_replace_argument = dbkit::func::replace(Metric::label, Metric::attempts, "x"); //~ E0277
    let _nullable_range_count = dbkit::func::replace_range(Metric::label, "x", 1_i32, nullable_attempts());
    //~^ E0277
    let _non_text_translation_input = dbkit::func::translate_chars(Metric::attempts, "a", "b");
    //~^ E0277
}
