# dbkit

A small, Postgres-first async ORM-ish library with type-level loaded/unloaded relations.

## Quick intro

Define models with `#[model]` and use the generated query and relation APIs:

```rust
use dbkit::{model, Database};

#[model(table = "users")]
#[derive(Debug)]
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

#[model(table = "todos")]
#[derive(Debug)]
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
        for t in &u.todos {
            println!("{}", t.title);
        }
    }

    let user = User::by_id(1).one(&mut &db).await?.unwrap();
    let user = user.load(User::todos, &mut &db).await?;
    println!("{}", user.todos.len());

    Ok(())
}
```

## More examples

Basic query + ordering:

```rust
use dbkit::prelude::*;

let users = User::query()
    .filter(User::email.ilike("%@example.com"))
    .order_by(dbkit::Order::asc(User::name.as_ref()))
    .limit(20)
    .all(&mut &db)
    .await?;
```

Insert / update / delete:

```rust
let created = User::insert(UserInsert {
    name: "Alex".to_string(),
    email: "a@b.com".to_string(),
})
.returning_all()
.one(&mut &db)
.await?
.expect("inserted");

let updated = User::update()
    .set(User::name, "Updated")
    .filter(User::id.eq(created.id))
    .returning_all()
    .all(&mut &db)
    .await?;

let deleted = User::delete()
    .filter(User::id.eq(created.id))
    .execute(&mut &db)
    .await?;
```

Active model insert / update (change-tracked):

```rust
let mut active = User::new_active();
active.name = "Active".into();
active.email = "active@db.com".into();

let created = active.insert(&mut &db).await?;

let mut active = created.into_active();
active.name = "Updated".into();
let updated = active.update(&mut &db).await?;
```

Eager loading and join filtering:

```rust
let users: Vec<UserModel<Vec<Todo>>> = User::query()
    .with(User::todos.selectin())
    .all(&mut &db)
    .await?;

let filtered = User::query()
    .join(User::todos)
    .filter(Todo::title.eq("Keep me"))
    .distinct()
    .all(&mut &db)
    .await?;
```

Type-level loaded relations:

```rust
// `User` is the "bare row" alias: all relations are `NotLoaded`.
fn accepts_unloaded(user: &User) {
    println!("{}", user.name);
}

// Use the generic model type to require loaded relations in APIs.
fn needs_loaded(user: &UserModel<Vec<Todo>>) {
    // safe: todos are guaranteed to be loaded
    println!("todos: {}", user.todos.len());
}

// For multiple relations, generic params follow relation-field order.
// In this repo, `Todo` declares `user` then `tags`, so:
// - user loaded, tags not loaded => TodoModel<Option<User>, dbkit::NotLoaded>
// - user loaded, tags loaded     => TodoModel<Option<User>, Vec<Tag>>
//
// Nested loaded relations compose too:
// `UserModel<Vec<TodoModel<Option<User>, Vec<Tag>>>>`
// (i.e., users with todos loaded, and each todo has its user + tags loaded)
```

Lazy loading:

```rust
let user = User::by_id(1).one(&mut &db).await?.unwrap();
let user = user.load(User::todos, &mut &db).await?;
println!("todos: {}", user.todos.len());
```

NULL handling with `Option<T>`:

```rust
// assuming `NullableRow { note: Option<String> }`
let row = NullableRow::insert(NullableRowInsert { note: None })
    .returning_all()
    .one(&mut &db)
    .await?;

let rows = NullableRow::query()
    .filter(NullableRow::note.eq(None))
    .all(&mut &db)
    .await?;
```

Transactions:

```rust
let mut tx = db.begin().await?;
let users = User::query().all(&mut tx).await?;
tx.commit().await?;
```

## TODOs

- [ ] Implement true joined eager loading (single-query join decoding).
- [ ] Add locking options: `for_update`, `skip_locked`, `nowait`.
- [ ] Add optional helpers: `count()`, `exists()`, `first()`, `paginate()`.
- [ ] Expand type support (json feature gate).
- [ ] Store `#[unique]` / `#[index]` as metadata (even if no-op).

## Deviations from spec

- `load(...)` requires an executor argument: `user.load(User::todos, &mut ex)`.
- Relation state sealing is looser than spec (any `Vec<T>` / `Option<T>` satisfies the state trait).
