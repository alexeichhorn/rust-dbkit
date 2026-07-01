# dbkit

A small, Postgres-first async ORM-ish library with type-level loaded/unloaded relations.

## Usage

```rust
use dbkit::prelude::*;
```

## Quick Intro

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

    let users: Vec<User<Vec<Todo>>> = User::query()
        .filter(User::email.ilike("%@example.com"))
        .with(User::todos.selectin())
        .all(&db)
        .await?;

    for user in &users {
        for todo in &user.todos {
            println!("{} / {}", user.name, todo.title);
        }
    }

    Ok(())
}
```

The loaded graph is part of the Rust type. If a relation is not requested, that field stays
`dbkit::NotLoaded`; once you add `.with(...)`, normal field access is available at the matching
depth.

If a Rust field needs a different DB column name, use `#[dbkit(column = "...")]`:

```rust
#[dbkit(column = "type")]
type_: String,
```

## Common Mutations

Insert with the generated insert type:

```rust
let created = User::insert(UserInsert {
    name: "Alex".to_string(),
    email: "a@b.com".to_string(),
})
.returning_all()
.one(&db)
.await?
.expect("inserted");
```

Update one loaded row with `into_active()`:

```rust
let mut active = created.into_active();
active.name = "Renamed".into();
let updated = active.update(&db).await?;
```

Delete one loaded row with the active model:

```rust
let deleted = created.into_active().delete(&db).await?;
```

Use query-builder updates/deletes for bulk or conditional mutations.

## More Docs

- [Querying](docs/querying.md)
- [Mutations](docs/mutations.md)
- [Relations](docs/relations.md)
- [Expressions and aggregation](docs/expressions.md)
- [Postgres types](docs/postgres-types.md)
- [Database](docs/database.md)

## TODOs

- [x] Implement true joined eager loading (single-query join decoding).
- [x] Add aggregation/projection support: `select_only`, `column_as`, `group_by`, `sum`, `count`, `min`, `max`, and mapping into custom result structs (e.g., `into_model::<T>()` for aggregates).
- [x] Add SQL function expressions in queries (e.g., `COALESCE`, `DATE_TRUNC`, `UPPER`).
- [x] Add JSON column support (`serde_json::Value`) for insert/update/filter.
- [x] Add Postgres array column support (e.g., `Vec<String>`) for insert/update/filter.
- [ ] Generalize Postgres array support beyond `Vec<String>` (e.g., `Vec<i64>`, `Vec<uuid::Uuid>`, `Vec<bool>`).
- [x] Add bulk insert support (multi-row `insert_many`).
- [x] Add dynamic condition builder helpers (e.g., `Condition::any` / `Condition::all`).
- [x] Allow `order_by` on expressions or aliases (e.g., `date_trunc(...)`, `total`).
- [x] Add `between(a, b)` convenience for columns/expressions.
- [x] Add locking options: `for_update`, `skip_locked`, `nowait`.
- [x] Add optional helpers: `count()`, `exists()`, `paginate()`.
- [x] Add typed conflict helpers: `on_conflict_do_nothing`, `on_conflict_do_update`.
- [x] Add active model `save()` that chooses insert vs update.
- [ ] Store `#[unique]` / `#[index]` as metadata (even if no-op).

## Deviations From Spec

- `load(...)` requires an executor argument: `user.load(User::todos, &ex)`.
- Relation state sealing is looser than spec (any `Vec<T>` / `Option<T>` satisfies the state trait).
