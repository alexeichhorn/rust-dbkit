use dbkit::model;

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub email: String,
}

fn main() {
    let _row = UserModel {
        //~^ ERROR: cannot find struct, variant or union type `UserModel`
        id: 1,
        email: "a@b.com".to_string(),
    };
}
