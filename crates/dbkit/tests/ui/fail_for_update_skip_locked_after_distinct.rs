use dbkit::model;

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub email: String,
}

fn main() {
    let _query = User::query().distinct().for_update().skip_locked(); //~ E0599
}
