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
        .for_update()
        .group_by(User::email) //~ E0599
        .having(User::id.gt(0_i64));
}
