use dbkit::model;

#[model(table = "users")]
pub struct User {
    #[key]
    pub id: i64,
}

fn main() {
    let _query = User::query().skip_locked(); //~ E0599
}
