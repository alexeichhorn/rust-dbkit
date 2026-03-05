use dbkit::model;

#[model(table = "records")]
pub struct Record {
    #[key]
    pub id: i64,
    pub label: String,
}

fn main() {
    let _query = Record::query().filter((Record::label + 1_i64).eq("x")); //~ ERROR
}
