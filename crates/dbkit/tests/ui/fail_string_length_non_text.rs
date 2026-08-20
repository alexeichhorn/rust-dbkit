use dbkit::model;

#[model(table = "metrics")]
pub struct Metric {
    #[key]
    pub id: i64,
    pub attempts: i32,
}

fn main() {
    let _bytes = dbkit::func::byte_length(Metric::attempts); //~ E0277
}
