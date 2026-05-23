# Changelog

All notable changes to `dbkit` will be documented in this file.

## 0.2.1

This release adds more typed SQL expression support, row-value filters, correlated `EXISTS` queries, column rename attributes, transaction-local settings, and a set of SQL compiler fixes around subqueries, nullable expressions, and enum casts.

### New Features

#### Column rename attributes via `#[dbkit(column = "...")]`

Model fields can now map to database columns with different names. This is useful for reserved words, legacy schemas, or Rust naming conventions.

Minimal example:

```rust
#[dbkit::model(table = "events")]
struct Event {
    #[key]
    id: i64,

    #[dbkit(column = "type")]
    type_: String,

    #[dbkit(column = "external_ref")]
    external_reference: String,
}
```

Queries, inserts, updates, joined loading, and row decoding all use the configured database column name.

#### Row-value filters for composite lookups

`dbkit::row((...)).in_(...)` can now express multi-column `IN` filters with typed values.

Minimal example:

```rust
let rows = LookupRow::query()
    .filter(dbkit::row((
        LookupRow::scope,
        LookupRow::external_key,
        LookupRow::locale,
    )).in_([
        (LookupScope::Public, "alpha", "en"),
        (LookupScope::Internal, "beta", "de"),
    ]))
    .all(&db)
    .await?;
```

This also supports enum casts and reuses repeated bind values where possible.

#### Correlated `EXISTS` / `NOT EXISTS` filters

Select, update, and delete builders now support `where_exists(...)` and `where_not_exists(...)`.

Minimal example:

```rust
let projects = Project::query()
    .where_exists(
        Task::query()
            .select_only()
            .column(Task::id)
            .filter(Task::project_id.eq_col(Project::id))
            .filter(Task::state.eq("active")),
    )
    .all(&db)
    .await?;
```

The same pattern works for mutations:

```rust
let deleted = Task::delete()
    .where_exists(
        Project::query()
            .select_only()
            .column(Project::id)
            .filter(Project::id.eq_col(Task::project_id))
            .filter(Project::state.eq("archived")),
    )
    .execute(&db)
    .await?;
```

#### More typed SQL functions

This release adds typed helpers for common string, aggregate, comparison, and math expressions.

Available helpers include:

- `dbkit::func::trim`
- `dbkit::func::char_length`
- `dbkit::func::min`
- `dbkit::func::max`
- `dbkit::func::least`
- `dbkit::func::greatest`
- `dbkit::func::power`

Minimal example:

```rust
let rows = TextSample::query()
    .filter(dbkit::func::char_length(dbkit::func::trim(TextSample::body)).ge(5))
    .all(&db)
    .await?;
```

#### Dynamic interval math

Interval expressions now accept typed expressions, not only literal values. This makes retry windows, stale-row queries, and exponential backoff filters easier to express without raw SQL.

Minimal example:

```rust
let retry_seconds = dbkit::func::least(
    3600.0_f64,
    dbkit::func::power(2.0_f64, WorkRun::attempts - 1_i32) * 60.0_f64,
);

let rows = WorkRun::query()
    .filter(WorkRun::updated_at.le(now - dbkit::interval::seconds(retry_seconds)))
    .all(&db)
    .await?;
```

#### Transaction-local Postgres settings

`DbTransaction::set_local(...)` can now set PostgreSQL settings scoped to the current transaction.

Minimal example:

```rust
let tx = db.begin().await?;
tx.set_local("statement_timeout", "5s").await?;

let rows = User::query().all(&tx).await?;

tx.commit().await?;
```

`set_local` uses PostgreSQL `set_config(..., true)`, so settings do not leak across pooled connection reuse.

### Behavior And Safety Improvements

#### Safer subquery placeholder rebinding

Subquery SQL rebinding is now more robust around quoted identifiers, UTF-8 aliases, aliases containing `$`, and schema-qualified enum casts.

This improves generated SQL for nested and correlated queries, especially when subqueries include enum values or unusual table aliases.

#### Nullable aggregate typing for `min` and `max`

`min` and `max` now model PostgreSQL aggregate behavior more accurately by returning nullable output types.

Minimal example:

```rust
let row = Sale::query()
    .select_only()
    .column_as(dbkit::func::min(Sale::created_at), "first_sale_at")
    .column_as(dbkit::func::max(Sale::created_at), "last_sale_at")
    .into_model::<SaleExtrema>()
    .one(&db)
    .await?;
```

#### Nullable comparison value support

Ordered comparison helpers now work better with nullable expressions and typed expression values, which helps dynamic interval math and other composed filters.

#### Row-value SQL generation fixes

Row-value `IN` filters handle empty input, repeated values, enum casts, and mixed typed columns correctly.

### Upgrade Notes

- There are no intended user-facing breaking changes in this release.
- If you use only the high-level model/query APIs, this should be a normal dependency bump.
- Low-level expression internals gained variants to support the new SQL features; code matching directly on `ExprNode` or `BinaryOp` may need additional arms.

## 0.2.0

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
