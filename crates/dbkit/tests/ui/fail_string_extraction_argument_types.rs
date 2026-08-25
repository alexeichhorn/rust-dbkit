use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub label: String,
    pub attempts: i32,
}

fn main() {
    let _non_text_input = dbkit::func::left(Metric::attempts, 2_i32); //~ E0277
    let _string_count = dbkit::func::right(Metric::label, "2"); //~ E0277
    let _float_start = dbkit::func::substring(Metric::label, 1.5_f32, 2_i32); //~ E0277
    let _bigint_count = dbkit::func::repeat(Metric::label, 2_i64); //~ E0277
    let _non_text_fill = dbkit::func::pad_start(Metric::label, 8_i32, 1_i32); //~ E0277
    let _non_text_end_fill = dbkit::func::pad_end(Metric::label, 8_i32, 1_i32); //~ E0277
}
