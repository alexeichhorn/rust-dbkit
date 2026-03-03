use dbkit::{model, BelongsTo, HasMany};

#[model(table = "users")] //~ E0599
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
    pub user_id: i64,
    #[belongs_to(key = user_id, references = id)]
    pub user: BelongsTo<User>,
    pub title: String,
}

fn main() {
    let unloaded = UserModel {
        id: 1,
        todos: dbkit::NotLoaded,
    };
    let _should_fail = unloaded.todos_loaded();
}
