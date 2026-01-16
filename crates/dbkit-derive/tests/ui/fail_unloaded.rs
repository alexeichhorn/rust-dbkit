use dbkit::{model, HasMany};

#[model(table = "users")]
pub struct User {
    #[key]
    pub id: i64,
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
    let unloaded = UserModel {
        id: 1,
        todos: dbkit::NotLoaded,
    };
    let _should_fail = unloaded.todos();
}
