// Binding a value is not enough to make its PostgreSQL type bitwise-compatible.
use dbkit::model;

#[model(table = "measurements")]
struct Measurement {
    #[key]
    id: i64,
    label: String,
    ratio: f64,
    active: bool,
}

fn main() {
    let _text = Measurement::label & "mask"; //~ E0369
    let _float = Measurement::ratio | 1.0_f64; //~ E0369
    let _bool = !Measurement::active; //~ E0600
}
