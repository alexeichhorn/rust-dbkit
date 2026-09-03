// Types without integer conversion remain excluded from bitwise expressions.
use dbkit::model;

#[model(table = "measurements")]
struct Measurement {
    #[key]
    id: i64,
    label: String,
    ratio: f64,
}

fn main() {
    let _text = Measurement::label & "mask"; //~ E0369
    let _float = Measurement::ratio | 1.0_f64; //~ E0369
}
