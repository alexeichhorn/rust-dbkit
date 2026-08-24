# Postgres Types

## Supported Types

Built-in typed query/insert/update bindings currently support:

- `bool`
- `i16`, `i32`, `i64`
- `f32`, `f64`
- `String` (and `&str` where string expressions are accepted)
- `uuid::Uuid`
- `chrono::NaiveDateTime` (`TIMESTAMP`)
- `chrono::DateTime<chrono::Utc>` (`TIMESTAMPTZ`)
- `chrono::NaiveDate` (`DATE`)
- `chrono::NaiveTime` (`TIME`)
- `dbkit::PgInterval` (`INTERVAL`)
- `serde_json::Value` (`JSON` / `JSONB`)
- `Vec<u8>` (`BYTEA`)
- `Vec<String>` (`TEXT[]`)
- `dbkit::PgVector<const N: usize>` (`vector`)
- custom Postgres enums via `#[derive(dbkit::DbEnum)]`
- `Option<T>` for nullable columns, where `T` is one of the above

Notes:
- `eq(None)` / `ne(None)` compile to `IS NULL` / `IS NOT NULL`.
- Interval expressions are available via `dbkit::interval::{days, hours, minutes, seconds}`.
- Comparison operators can take literal values or typed expressions on the right-hand side.
- Enum binds are emitted as typed placeholders (`$n::your_enum_type`) for Postgres enum columns.
- For types outside this list, use raw `sqlx` queries or add explicit dbkit support first.

## NULL Handling With `Option<T>`

The model macro keeps required and nullable fields distinct. Filters and mutations reject `None`
for required fields at compile time.

```rust,ignore
// assuming `NullableRow { note: Option<String> }`
let row = NullableRow::insert(NullableRowInsert { note: None })
    .returning_all()
    .one(&db)
    .await?;

let rows = NullableRow::query()
    .filter(NullableRow::note.eq(None))
    .all(&db)
    .await?;
```

Nullable columns also accept direct non-null values, so string literals do not need allocation:

```rust
NullableRow::query().filter(NullableRow::note.eq("present"));
NullableRow::update().set(NullableRow::note, "updated");
```

When a value is dynamically optional, use the column's owned type, such as `Option<String>`.
`Option<&str>` is intentionally not accepted for nullable string columns; this keeps bare `None`
unambiguous. Direct `&str` values remain supported.

SQL functions preserve nullability when PostgreSQL propagates NULL. See
[Expressions and aggregation](expressions.md) for practical `coalesce` and column-comparison
examples.

## PgVector Embeddings

```sql
CREATE EXTENSION IF NOT EXISTS vector;
```

```rust,ignore
#[model(table = "embedding_rows")]
#[derive(Debug)]
struct EmbeddingRow {
    #[key]
    id: i64,
    label: String,
    embedding: dbkit::PgVector<3>,
    embedding_optional: Option<dbkit::PgVector<3>>,
}

let query = dbkit::PgVector::<3>::new([1.0, 0.0, 0.0])?;

// ANN/index-friendly top-k retrieval
let ann_top_k = EmbeddingRow::query()
    .filter(EmbeddingRow::embedding_optional.is_not_null())
    .order_by(dbkit::Order::asc(dbkit::func::inner_product_distance(
        EmbeddingRow::embedding_optional,
        query.clone(),
    )))
    .limit(5)
    .all(&db)
    .await?;

// True inner product score (semantic ranking), may not use ANN index
let semantic_top_k = EmbeddingRow::query()
    .filter(EmbeddingRow::embedding_optional.is_not_null())
    .order_by(dbkit::Order::desc(dbkit::func::inner_product(
        EmbeddingRow::embedding_optional,
        query.clone(),
    )))
    .limit(5)
    .all(&db)
    .await?;

let high_similarity = EmbeddingRow::query()
    .filter(dbkit::func::cosine_distance(EmbeddingRow::embedding, query.clone()).lt(0.1_f32))
    .order_by(dbkit::Order::asc(dbkit::func::cosine_distance(
        EmbeddingRow::embedding,
        query,
    )))
    .all(&db)
    .await?;
```

Available vector distance/similarity functions:
- `dbkit::func::l2_distance`
- `dbkit::func::cosine_distance`
- `dbkit::func::inner_product`
- `dbkit::func::l1_distance`
- `dbkit::func::inner_product_distance`

Notes:
- Dimension is part of the Rust type (`PgVector<3>`, `PgVector<1536>`, etc.).
- Optional embeddings are supported via `Option<PgVector<N>>`.
- `cosine_distance` is a distance metric (lower means more similar), so use `.lt(...)` thresholds.
- Operator-based helpers (`l2_distance`, `cosine_distance`, `l1_distance`, `inner_product_distance`)
  are ANN-index compatible for `ORDER BY ... LIMIT` with pgvector indexes.
- `inner_product` preserves true score semantics (higher is better), but as a function expression it
  may not use pgvector ANN indexes for `ORDER BY ... LIMIT`.
- `inner_product_distance` uses negative inner-product distance, so `inner_product > 0.9`
  corresponds to `inner_product_distance < -0.9`.
- For CI, use a Postgres image with pgvector installed (for example `pgvector/pgvector:pg16`).

## Postgres Enums With `DbEnum`

`dbkit` supports first-class Postgres enums in models, filters, inserts, updates, and conflict updates.

Define the enum once:

```rust,ignore
#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "task_state", rename_all = "snake_case")]
pub enum TaskState {
    PendingReview,
    InProgress,
    Completed,
    Failed,
}
```

Use it directly in your model:

```rust,ignore
use dbkit::model;

#[model(table = "tasks")]
pub struct Task {
    #[key]
    pub id: i64,
    pub title: String,
    pub state: TaskState,
    pub previous_state: Option<TaskState>,
}
```

Use it in typed query/mutation APIs:

```rust,ignore
let rows = Task::query()
    .filter(Task::state.eq(TaskState::InProgress))
    .filter(Task::state.in_([TaskState::PendingReview, TaskState::InProgress]))
    .all(&db)
    .await?;

let updated = Task::update()
    .set(Task::state, TaskState::Completed)
    .set(Task::previous_state, Some(TaskState::InProgress))
    .filter(Task::id.eq(42_i64))
    .returning_all()
    .one(&db)
    .await?;

Task::update()
    .set(Task::previous_state, None)
    .filter(Task::id.eq(42_i64))
    .execute(&db)
    .await?;
```

Upsert with enum columns is also supported:

```rust,ignore
let row = Task::insert(TaskInsert {
    id: 42,
    title: "Ship enum support".to_string(),
    state: TaskState::PendingReview,
    previous_state: None,
})
.on_conflict_do_update(Task::id, (Task::state, Task::previous_state))
.returning_all()
.one(&db)
.await?;
```

### Enum Naming Controls

- `#[dbkit(type_name = "...")]` is required and should match your Postgres enum type.
- `#[dbkit(rename_all = "...")]` is optional and supports:
  - `snake_case`
  - `lowercase`
  - `UPPERCASE`
  - `SCREAMING_SNAKE_CASE`
- Override a single variant with `#[dbkit(rename = "...")]`:

```rust,ignore
#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "delivery_channel", rename_all = "snake_case")]
pub enum DeliveryChannel {
    Email,
    Sms,
    #[dbkit(rename = "http_webhook")]
    Webhook,
}
```
