#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::{model, Database};

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
    pub email: String,
    #[has_many]
    pub todos: dbkit::HasMany<Todo>,
}

#[model(table = "todos")]
pub struct Todo {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    #[belongs_to(key = user_id, references = id)]
    pub user: dbkit::BelongsTo<User>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
) -> Result<(), dbkit::Error> {
    dbkit::sqlx::query(
        "CREATE TEMP TABLE users (\
            id BIGSERIAL PRIMARY KEY,\
            name TEXT NOT NULL,\
            email TEXT NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE todos (\
            id BIGSERIAL PRIMARY KEY,\
            user_id BIGINT NOT NULL,\
            title TEXT NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn seed_user(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    name: &str,
    email: &str,
) -> Result<User, dbkit::Error> {
    let user = User::insert(UserInsert {
        name: name.to_string(),
        email: email.to_string(),
    })
    .returning_all()
    .one(&mut *tx)
    .await?
    .expect("inserted user");
    Ok(user)
}

async fn seed_todo(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    user_id: i64,
    title: &str,
) -> Result<Todo, dbkit::Error> {
    let todo = Todo::insert(TodoInsert {
        user_id,
        title: title.to_string(),
    })
    .returning_all()
    .one(&mut *tx)
    .await?
    .expect("inserted todo");
    Ok(todo)
}

#[tokio::test]
async fn insert_update_delete_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Alex", "a@b.com").await?;
    assert!(user.id > 0);
    assert_eq!(user.name, "Alex");

    let updated = User::update()
        .set(User::name, "Updated")
        .filter(User::id.eq(user.id))
        .returning_all()
        .all(&mut tx)
        .await?;
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].name, "Updated");

    let deleted = User::delete()
        .filter(User::id.eq(user.id))
        .execute(&mut tx)
        .await?;
    assert_eq!(deleted, 1);

    let remaining = User::query().all(&mut tx).await?;
    assert!(remaining.is_empty());

    Ok(())
}

#[tokio::test]
async fn selectin_has_many_loads_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Riley", "r@b.com").await?;
    let _todo1 = seed_todo(&mut tx, user.id, "Write tests").await?;
    let _todo2 = seed_todo(&mut tx, user.id, "Ship code").await?;

    let users: Vec<UserModel<Vec<Todo>>> = User::query()
        .filter(User::id.eq(user.id))
        .with(User::todos.selectin())
        .all(&mut tx)
        .await?;

    assert_eq!(users.len(), 1);
    let mut titles: Vec<String> = users[0]
        .todos
        .iter()
        .map(|todo| todo.title.clone())
        .collect();
    titles.sort();
    assert_eq!(titles, vec!["Ship code", "Write tests"]);

    Ok(())
}

#[tokio::test]
async fn selectin_belongs_to_loads_parent() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Dana", "d@b.com").await?;
    let todo = seed_todo(&mut tx, user.id, "Map relations").await?;

    let todos: Vec<TodoModel<Option<User>>> = Todo::query()
        .filter(Todo::id.eq(todo.id))
        .with(Todo::user.selectin())
        .all(&mut tx)
        .await?;

    assert_eq!(todos.len(), 1);
    let loaded = todos[0].user.as_ref().expect("loaded user");
    assert_eq!(loaded.id, user.id);
    assert_eq!(loaded.email, "d@b.com");

    Ok(())
}

#[tokio::test]
async fn nested_selectin_loads() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Jo", "jo@b.com").await?;
    let _todo = seed_todo(&mut tx, user.id, "Chain loads").await?;

    let users = User::query()  // should be Vec<UserModel<Vec<TodoModel<Option<User>>>>>
        .filter(User::id.eq(user.id))
        .with(User::todos.selectin().with(Todo::user.selectin()))
        .all(&mut tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].todos.len(), 1);
    let nested_user = users[0].todos[0].user.as_ref().expect("nested user");
    assert_eq!(nested_user.id, user.id);

    Ok(())
}

#[tokio::test]
async fn lazy_load_relation() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Ari", "ari@b.com").await?;
    let _todo = seed_todo(&mut tx, user.id, "Lazy load").await?;

    let loaded = user.load(User::todos, &mut tx).await?;
    assert_eq!(loaded.todos.len(), 1);
    assert_eq!(loaded.todos[0].title, "Lazy load");

    Ok(())
}
