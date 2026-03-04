use dbkit::model;

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub email: String,
}

fn main() {
    let _query = User::query()
        .group_by(User::email)
        .having(User::id.gt(0_i64))
        .for_update(); //~ E0599
}
