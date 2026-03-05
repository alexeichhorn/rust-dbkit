use dbkit::model;

#[model(table = "schedules")]
pub struct Schedule {
    #[key]
    pub id: i64,
    pub base_interval_hours: i32,
}

fn main() {
    let _expr = dbkit::interval::hours(1.5_f64); //~ E0277
}
