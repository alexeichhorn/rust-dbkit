# dbkit

A small, Postgres-first async ORM-ish library with type-level loaded/unloaded relations.

## Quick intro

Define models with `#[derive(Model)]` and use the generated query and relation APIs:

```rust
use dbkit::{Database, Model};

#[derive(Debug, Model)]
struct User {
    #[key]
    #[autoincrement]
    id: i64,
    name: String,
    #[unique]
    email: String,
    #[has_many]
    todos: dbkit::HasMany<Todo>,
}

#[derive(Debug, Model)]
struct Todo {
    #[key]
    id: i64,
    #[index]
    user_id: i64,
    #[belongs_to(key = user_id, references = id)]
    user: dbkit::BelongsTo<User>,
    title: String,
}

#[tokio::main]
async fn main() -> Result<(), dbkit::Error> {
    let db = Database::connect("postgres://...").await?;

    let users = User::query()
        .filter(User::email.eq("a@b.com"))
        .with(User::todos.selectin())
        .all(&mut &db)
        .await?;

    for u in &users {
        for t in u.todos_loaded() {
            println!("{}", t.title);
        }
    }

    let user = User::by_id(1).one(&mut &db).await?.unwrap();
    let user = user.load(User::todos, &mut &db).await?;
    println!("{}", user.todos_loaded().len());

    Ok(())
}
```

## TODOs and current deviations

- Many-to-many is not implemented yet. `#[many_to_many]` is parsed but no descriptors/loaders exist.
- Joined eager loading is currently a select-in fallback (no single-query join decoding yet).
- Locking options (`for_update`, `skip_locked`, `nowait`) are not implemented.
- Optional helpers like `count()`, `exists()`, `first()`, `paginate()` are not implemented.
- Type support is limited to primitives + `String` (no uuid/chrono/time/json yet).
- `NULL` bind values are not supported yet (binding `Option::None` fails).
- `#[unique]` / `#[index]` are parsed but not stored as metadata.
- Relation accessors are `*_loaded()` (e.g. `todos_loaded()`), not `todos()`, to avoid name clashes with relation descriptors.
- `load(...)` requires an executor argument: `user.load(User::todos, &mut ex)`.
- Relation state sealing is looser than spec (any `Vec<T>` / `Option<T>` satisfies the state trait).
