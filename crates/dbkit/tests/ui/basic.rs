use dbkit::{model, BelongsTo, Executor, HasMany};

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
    pub user_id: i64,
    #[belongs_to(key = user_id, references = id)]
    pub user: BelongsTo<User>,
    pub title: String,
}

fn main() {
    let _table = User::TABLE;
    let _col = User::email;
    let query = User::query().filter(User::email.eq("a@b.com")).limit(1);
    let _sql = query.debug_sql();
    let _insert = User::insert(UserInsert {
        name: "Alex".to_string(),
        email: "a@b.com".to_string(),
    });
    let _update = User::update();
    let _delete = User::delete();

    let _rel = User::todos;
    let _rel2 = Todo::user;
    let _load = User::todos.selectin();
    let _loaded_query: dbkit::Select<UserModel<Vec<Todo>>, _> =
        User::query().with(User::todos.selectin());
    let _nested_query: dbkit::Select<UserModel<Vec<TodoModel<Option<User>>>>, _> = User::query()
        .with(User::todos.selectin().with(Todo::user.joined()));

    let loaded = UserModel::<Vec<Todo>> {
        id: 1,
        name: "Alex".to_string(),
        email: "a@b.com".to_string(),
        todos: vec![],
    };
    let _slice = loaded.todos_loaded();

    struct Dummy;
    impl Executor for Dummy {
        fn fetch_all<'e, T>(
            &'e self,
            _sql: &'e str,
            _args: dbkit::sqlx::postgres::PgArguments,
        ) -> dbkit::executor::BoxFuture<'e, Result<Vec<T>, dbkit::Error>>
        where
            T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
        {
            Box::pin(async { unimplemented!() })
        }

        fn fetch_optional<'e, T>(
            &'e self,
            _sql: &'e str,
            _args: dbkit::sqlx::postgres::PgArguments,
        ) -> dbkit::executor::BoxFuture<'e, Result<Option<T>, dbkit::Error>>
        where
            T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
        {
            Box::pin(async { unimplemented!() })
        }

        fn fetch_rows<'e>(
            &'e self,
            _sql: &'e str,
            _args: dbkit::sqlx::postgres::PgArguments,
        ) -> dbkit::executor::BoxFuture<'e, Result<Vec<dbkit::sqlx::postgres::PgRow>, dbkit::Error>> {
            Box::pin(async { unimplemented!() })
        }

        fn execute<'e>(
            &'e self,
            _sql: &'e str,
            _args: dbkit::sqlx::postgres::PgArguments,
        ) -> dbkit::executor::BoxFuture<'e, Result<u64, dbkit::Error>> {
            Box::pin(async { unimplemented!() })
        }
    }

    let unloaded = UserModel {
        id: 1,
        name: "Alex".to_string(),
        email: "a@b.com".to_string(),
        todos: dbkit::NotLoaded,
    };
    let dummy = Dummy;
    let _future = unloaded.load(User::todos, &dummy);

    let _insert_struct = UserInsert {
        name: "Alex".to_string(),
        email: "a@b.com".to_string(),
    };
}
