use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub attempts: i32,
}

fn main() {
    let _expr = dbkit::func::trim(Metric::attempts); //~ E0277
}
