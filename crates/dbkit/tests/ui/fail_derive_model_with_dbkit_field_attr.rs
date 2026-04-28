use dbkit::Model;

#[derive(Model)] //~ ERROR: dbkit: use #[model] instead of #[derive(Model)]
pub struct LegacyModel {
    #[dbkit(column = "type")]
    pub type_: String,
}

fn main() {}
