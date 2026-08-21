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
    let _non_text_static = dbkit::func::concat!([Metric::attempts]); //~ E0277
    let _mixed_non_text_static = dbkit::func::concat!([Metric::label, Metric::attempts]); //~ E0277
    let _non_text_separator = dbkit::func::concat_with_separator!(Metric::attempts, [Metric::label]); //~ E0277
    let _non_text_value = dbkit::func::concat_with_separator!("::", [Metric::attempts]); //~ E0277

    let dynamic_non_text_values = vec![Metric::attempts.into_expr()];
    let _non_text_dynamic = dbkit::func::concat!(dynamic_non_text_values); //~ E0277
}
