use chrono::NaiveDateTime;
use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub label: String,
    pub nullable_label: Option<String>,
    pub recorded_at: NaiveDateTime,
}

fn main() {
    let _text = dbkit::func::sum(Metric::label); //~ E0277
    let _nullable_text = dbkit::func::sum(Metric::nullable_label); //~ E0277
    let _timestamp = dbkit::func::sum(Metric::recorded_at); //~ E0277
}
