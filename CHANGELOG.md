# Changelog

All notable changes to `dbkit` will be documented in this file.

## 0.2.0 - Unreleased

This is the first substantial release since `0.1.1`. It includes first-class Postgres enums, `pgvector` support, row locking, migrations, arithmetic and interval expressions, column-to-column comparisons, wider typed `ON CONFLICT` support, configurable pool options, and `sqlx` 0.8.

### Breaking Changes

#### `sqlx` was upgraded from 0.7 to 0.8

`dbkit` now depends on and re-exports `sqlx` 0.8. If your application uses `dbkit::sqlx` directly, or mixes `dbkit` types with direct `sqlx` APIs, you should upgrade both together.

Minimal example:

```toml
[dependencies]
dbkit = "0.2"
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio-rustls", "postgres"] }
```

If you only use the high-level `dbkit` APIs, this will usually be a straightforward dependency bump. If you implement custom `sqlx` encoders/decoders, build raw `sqlx` queries beside `dbkit`, or import `dbkit::sqlx` symbols directly, expect a normal `sqlx` 0.8 migration.

### New Features

#### Native Postgres enums via `#[derive(dbkit::DbEnum)]`

`dbkit` now supports first-class Postgres enums in models, filters, inserts, updates, and conflict updates. The derive validates enum metadata at compile time, supports `type_name`, `rename_all`, and per-variant `rename`, and keeps enum binds typed for Postgres.

Minimal example:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "task_state", rename_all = "snake_case")]
enum TaskState {
    PendingReview,
    InProgress,
    Completed,
}

#[dbkit::model(table = "tasks")]
struct Task {
    #[key]
    id: i64,
    state: TaskState,
}

let rows = Task::query()
    .filter(Task::state.eq(TaskState::InProgress))
    .all(&db)
    .await?;
```

#### `pgvector` support with typed `PgVector<N>`

`dbkit` now provides `PgVector<const N: usize>` for storing and querying embeddings. Dimensions are encoded in the Rust type, invalid dimensions are rejected, and non-finite floats are validated early. Distance and similarity helpers cover both ANN-friendly operators and true inner-product scoring.

Minimal example:

```rust
#[dbkit::model(table = "embedding_rows")]
struct EmbeddingRow {
    #[key]
    id: i64,
    embedding: dbkit::PgVector<3>,
}

let query = dbkit::PgVector::<3>::new([1.0, 0.0, 0.0])?;

let rows = EmbeddingRow::query()
    .order_by(dbkit::Order::asc(dbkit::func::cosine_distance(
        EmbeddingRow::embedding,
        query,
    )))
    .limit(5)
    .all(&db)
    .await?;
```

Available helpers include:

- `dbkit::func::l2_distance`
- `dbkit::func::cosine_distance`
- `dbkit::func::inner_product`
- `dbkit::func::l1_distance`
- `dbkit::func::inner_product_distance`

#### Row locking with `for_update`, `skip_locked`, and `nowait`

Select queries now support row-level locking. Locking clauses are scoped safely for left joins, and invalid method combinations are rejected at compile time instead of producing questionable SQL.

Minimal example:

```rust
let rows = Job::query()
    .filter(Job::status.eq("pending"))
    .for_update()
    .skip_locked()
    .all(&tx)
    .await?;
```

This release also adds compile-time guards for invalid combinations such as:

- `distinct().for_update()`
- `for_update().distinct()`
- `group_by(...).for_update()`
- `skip_locked()` or `nowait()` without `for_update()`

#### Optional migrations via `dbkit`'s `migrations` feature

`dbkit` can now run `sqlx` migrations directly through `Database::migrate(...)`. Migration support is opt-in so existing users do not pay the dependency cost unless they need it.

Minimal example:

```toml
[dependencies]
dbkit = { version = "0.2", features = ["migrations"] }
```

```rust
use dbkit::{migrate::Migrator, Database};

static MIGRATOR: Migrator = dbkit::sqlx::migrate!("./migrations");

let db = Database::connect("postgres://...").await?;
db.migrate(&MIGRATOR).await?;
```

#### Configurable connection pools via `Database::connect_with_options`

`Database::connect(...)` remains the zero-config default, but callers that need pool tuning can now build their own `PgPoolOptions` without importing `sqlx` directly.

Minimal example:

```rust
let db = dbkit::Database::connect_with_options(
    "postgres://...",
    dbkit::PgPoolOptions::new().max_connections(20),
)
.await?;
```

#### Typed `ON CONFLICT` helpers

Insert builders now support typed `ON CONFLICT DO NOTHING` and `ON CONFLICT DO UPDATE`. Composite conflict targets and update column tuples are supported, and the tuple arity for updates now extends up to 32 columns.

Minimal example:

```rust
let row = OrderLine::insert(OrderLineInsert {
    order_id: 7,
    line_id: 8,
    note: "Updated via upsert".to_string(),
})
.on_conflict_do_update(
    (OrderLine::order_id, OrderLine::line_id),
    OrderLine::note,
)
.returning_all()
.one(&db)
.await?;
```

#### Arithmetic expressions in filters, ordering, and projections

Numeric and temporal expressions can now participate in typed SQL generation. This makes it possible to express arithmetic directly in `filter`, `order_by`, and `select_only` flows without dropping to raw SQL.

Minimal example:

```rust
let rows = Record::query()
    .filter((Record::left_value + 1_i64).lt_col(Record::baseline_value))
    .order_by(dbkit::Order::desc(Record::baseline_value + Record::left_value))
    .all(&db)
    .await?;
```

#### Interval expressions and `PgInterval`

`dbkit` now supports Postgres interval values and builders such as `days`, `hours`, `minutes`, and `seconds`. Interval expressions can be compared, ordered, and composed with other typed expressions.

Minimal example:

```rust
let rows = Schedule::query()
    .filter(dbkit::interval::hours(Schedule::base_interval_hours).eq_col(Schedule::lease_window))
    .all(&db)
    .await?;
```

#### Column-to-column comparisons, including null-safe comparisons

Queries can now compare one column to another without raw SQL. This includes regular comparisons and Postgres null-safe `IS DISTINCT FROM` semantics.

Minimal example:

```rust
let stale = Job::query()
    .filter(Job::embedding_hash.is_distinct_from_col(Job::content_hash))
    .all(&db)
    .await?;
```

Available helpers include:

- `eq_col`
- `ne_col`
- `lt_col`
- `le_col`
- `gt_col`
- `ge_col`
- `is_distinct_from_col`
- `is_not_distinct_from_col`

#### `chrono::DateTime<Utc>` / `TIMESTAMPTZ` support

`dbkit` now supports `chrono::DateTime<Utc>` as a first-class typed value for filters, inserts, updates, and result decoding.

Minimal example:

```rust
let rows = Event::query()
    .filter(Event::published_at.gt(since))
    .all(&db)
    .await?;
```

### Behavior And Safety Improvements

#### Active model updates now write only changed fields

Active updates no longer blindly touch unrelated columns. `Set`, `Null`, and unchanged states are handled more precisely, which reduces accidental overwrite risk and makes partial updates safer.

Minimal example:

```rust
let mut active = user.into_active();
active.name = "Updated".to_string().into();
active.update(&db).await?;
```

Only the changed field is written back.

#### Better compile-time diagnostics around enum usage

`DbEnum` derive now rejects duplicate wire names, validates required enum metadata, and improves acronym-aware snake case generation for enum value mapping.

Minimal example:

```rust
#[derive(dbkit::DbEnum)]
#[dbkit(type_name = "delivery_channel", rename_all = "snake_case")]
enum DeliveryChannel {
    Email,
    HTTPWebhook,
}
```

This maps `HTTPWebhook` to `http_webhook` instead of producing awkward acronym splits.

#### Safer locking SQL around joins

When `FOR UPDATE` is used with left joins, the generated SQL scopes the lock to the base table to avoid over-locking joined rows unintentionally.

Minimal example:

```rust
let rows = User::query()
    .left_join(User::todos)
    .for_update()
    .nowait()
    .all(&tx)
    .await?;
```

### Upgrade Notes

- If you depend on `sqlx` directly, upgrade it to `0.8` alongside `dbkit`.
- If you want migrations, enable `dbkit`'s `migrations` feature explicitly.
- If you want custom pool sizing or connection tuning, switch to `Database::connect_with_options(...)` with `dbkit::PgPoolOptions`.
- If you are adding enum support, define the Postgres enum type in the database first and then match it with `#[dbkit(type_name = "...")]`.
- If you are adding vector support, ensure the `vector` extension is installed in Postgres.
