use dbkit::model;
use dbkit::IntoExpr;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub label: String,
    pub attempts: i32,
}

fn main() {
    let _non_text_concat = dbkit::func::concat([Metric::attempts.into_expr()]); //~ E0277
    let _non_text_separator = dbkit::func::concat_with_separator(1_i32, [Metric::label.into_expr()]); //~ E0277
    let _non_text_split_input = dbkit::func::split(Metric::attempts, ","); //~ E0277
    let _non_integer_part_index = dbkit::func::split_part(Metric::label, ",", "1");
    //~^ E0277
}
