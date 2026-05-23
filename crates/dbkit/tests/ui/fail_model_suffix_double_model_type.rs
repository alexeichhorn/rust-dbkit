use dbkit::model;

#[model(table = "users")]
pub struct UserModel {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub email: String,
}

fn main() {
    let _row = UserModelModel {
        //~ ERROR: cannot find struct, variant or union type `UserModelModel`
        id: 1,
        email: "a@b.com".to_string(),
    };
}
