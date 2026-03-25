use dbkit::model;

#[model(table = "flags")]
pub struct Flag {
    #[key]
    pub id: i64,
    pub enabled: bool,
}

fn main() {
    let _expr = dbkit::func::char_length(Flag::enabled); //~ E0277
}
