# dbkit

`dbkit` is a Postgres-first async ORM-ish library with type-level loaded and unloaded relations.

## Installation

```toml
[dependencies]
dbkit = "0.3"
```

Enable SQL migrations when needed:

```toml
[dependencies]
dbkit = { version = "0.3", features = ["migrations"] }
```

## Example

```rust,no_run
use dbkit::{model, Database, SelectExt};

#[model(table = "users")]
struct User {
    #[key]
    #[autoincrement]
    id: i64,
    email: String,
}

async fn example(db: &Database) -> Result<Vec<User>, dbkit::Error> {
    User::query()
        .filter(User::email.ilike("%@example.com"))
        .all(db)
        .await
}
```

The `#[model]` macro generates typed columns, queries, inserts, updates, deletes, and relation
loading. Import `dbkit::prelude::*` to bring the common query and mutation extension traits into
scope.
