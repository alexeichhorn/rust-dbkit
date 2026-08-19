use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub attempts: i32,
}

fn main() {
    let _lower = dbkit::func::lower(Metric::attempts); //~ E0277
    let _directional_trim = dbkit::func::trim_end(Metric::attempts); //~ E0277
    let _custom_trim = dbkit::func::trim_start_chars(Metric::attempts, "x"); //~ E0277
}
