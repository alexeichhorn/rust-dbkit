use dbkit::{model, HasMany};

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
    pub email: String,
    #[has_many]
    pub todos: HasMany<Todo>,
}

#[model(table = "todos")]
pub struct Todo {
    #[key]
    pub id: i64,
    pub title: String,
}

fn main() {
    let _table = User::TABLE;
    let _col = User::email;
    let query = User::query().filter(User::email.eq("a@b.com")).limit(1);
    let _sql = query.debug_sql();
    let _insert = User::insert();
    let _update = User::update();
    let _delete = User::delete();

    let loaded = UserModel::<Vec<Todo>> {
        id: 1,
        name: "Alex".to_string(),
        email: "a@b.com".to_string(),
        todos: vec![],
    };
    let _slice = loaded.todos();

    let _insert_struct = UserInsert {
        name: "Alex".to_string(),
        email: "a@b.com".to_string(),
    };
}
