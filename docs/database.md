# Database

## Connecting

```rust,ignore
let db = Database::connect("postgres://...").await?;
```

Customize `sqlx` pool options when needed:

```rust,ignore
let db = Database::connect_with_options(
    "postgres://...",
    dbkit::PgPoolOptions::new().max_connections(20),
)
.await?;
```

## Migrations

Migrations are optional and use `sqlx`:

```toml
# Cargo.toml
dbkit = { version = "0.2", features = ["migrations"] }
```

```rust,ignore
use dbkit::{Database, migrate::Migrator};

static MIGRATOR: Migrator = dbkit::sqlx::migrate!("./migrations");

let db = Database::connect("postgres://...").await?;
db.migrate(&MIGRATOR).await?;
```

`dbkit` keeps migration execution thin and delegates migration file parsing/running to `sqlx`.

## Transactions

```rust,ignore
let tx = db.begin().await?;
let users = User::query().all(&tx).await?;
tx.commit().await?;
```

## Transaction-Local Postgres Settings

```rust,ignore
let tx = db.begin().await?;
tx.set_local("statement_timeout", "5s").await?;

let users = User::query()
    .filter(User::email.like("%@example.com"))
    .all(&tx)
    .await?;

tx.commit().await?;
```

`set_local` uses PostgreSQL `set_config(..., true)`, so the setting is scoped to the current transaction
instead of leaking across pooled connection reuse.
