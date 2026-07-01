# Expressions And Aggregation

## Arithmetic Expressions

```rust
let rows = Record::query()
    .filter((Record::left_value + 1_i64).lt_col(Record::baseline_value))
    .filter((Record::right_value - Record::left_value).gt(0_i64))
    .order_by(dbkit::Order::desc(Record::baseline_value + Record::left_value))
    .all(&db)
    .await?;
```

Arithmetic expressions also support `*` and compose with typed SQL helpers like
`dbkit::func::least`, `dbkit::func::greatest`, and `dbkit::func::power`.

## Interval Expressions

```rust
let rows = Schedule::query()
    .filter(dbkit::interval::hours(Schedule::base_interval_hours).eq_col(Schedule::lease_window))
    .order_by(dbkit::Order::asc(dbkit::interval::minutes(dbkit::func::coalesce(
        Schedule::backoff_minutes,
        15_i32,
    ))))
    .all(&db)
    .await?;
```

Dynamic interval math is supported too, including `dbkit::interval::seconds(expr)` and
timestamp comparisons like `Schedule::updated_at.le(now - dbkit::interval::seconds(retry_seconds))`.

## Aggregation And Projections

```rust
use dbkit::prelude::*;

#[derive(sqlx::FromRow, Debug)]
struct RegionTotal {
    region: String,
    total: dbkit::sqlx::types::BigDecimal,
}

let totals: Vec<RegionTotal> = Sale::query()
    .select_only()
    .column_as(Sale::region, "region")
    .column_as(dbkit::func::sum(Sale::amount), "total")
    .group_by(Sale::region)
    .having(dbkit::func::sum(Sale::amount).gt(0_i64))
    .into_model()
    .all(&db)
    .await?;
```

## SQL Functions And Expression-Based Grouping

```rust
#[derive(sqlx::FromRow, Debug)]
struct BucketTotal {
    bucket: chrono::NaiveDateTime,
    total: dbkit::sqlx::types::BigDecimal,
}

let buckets: Vec<BucketTotal> = Sale::query()
    .select_only()
    .column_as(dbkit::func::date_trunc("day", Sale::created_at), "bucket")
    .column_as(dbkit::func::sum(Sale::amount), "total")
    .group_by(dbkit::func::date_trunc("day", Sale::created_at))
    .into_model()
    .all(&db)
    .await?;
```

## Join + Aggregation

```rust
#[derive(sqlx::FromRow, Debug)]
struct UserTodoAgg {
    name: String,
    todo_count: i64,
}

let rows: Vec<UserTodoAgg> = User::query()
    .select_only()
    .column_as(User::name, "name")
    .column_as(dbkit::func::count(Todo::id), "todo_count")
    .join(User::todos)
    .group_by(User::name)
    .order_by(dbkit::Order::desc(User::name.as_ref()))
    .into_model()
    .all(&db)
    .await?;
```

Notes:
- `select_only()` switches from `SELECT *` to projections via `column(...)` or `column_as(...)`.
- Use `into_model::<T>()` to map into a custom `sqlx::FromRow` struct.
- `dbkit` re-exports `sqlx` for types, but `#[derive(sqlx::FromRow)]` expands to `::sqlx::...`; crates using the derive need `sqlx` available as a direct dependency.
- Aggregate helpers include `sum`, `count`, `min`, and `max`.
- `SUM` over integer columns returns `NUMERIC` in Postgres; use `BigDecimal` (or cast) for totals.
- Aggregations work across joins; order-by currently expects a real column/expr rather than an alias.
