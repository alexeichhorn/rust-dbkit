use chrono::NaiveDateTime;
use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub label: String,
    pub recorded_at: NaiveDateTime,
}

fn main() {
    let _text = Metric::label.cast::<f64>(); //~ ERROR: cast
    let _timestamp = Metric::recorded_at.cast::<f64>(); //~ ERROR: cast
}
