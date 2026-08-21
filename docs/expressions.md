# Expressions And Aggregation

## Nullability

Expressions follow the nullability of the model fields they use:

```rust
#[dbkit::model(table = "articles")]
struct Article {
    #[key]
    id: i64,
    title: String,
    subtitle: Option<String>,
}

let normalized_title = dbkit::func::lower(Article::title);
let normalized_subtitle = dbkit::func::lower(Article::subtitle);
let subtitle_with_fallback = dbkit::func::coalesce(Article::subtitle, "No subtitle");

let articles = Article::query()
    .filter(normalized_title.eq("rust"))
    .filter(normalized_subtitle.eq(None))
    .order_by(dbkit::Order::asc(subtitle_with_fallback));
```

Functions that propagate PostgreSQL NULL keep optional inputs optional. Use `coalesce` with a
non-null fallback when downstream code needs a guaranteed value.

Nullable columns accept direct values and `Option<T>` values. `eq(None)` and `ne(None)` compile to
`IS NULL` and `IS NOT NULL`. Required columns reject `None` at compile time.

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
        15,
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

## Filtered Aggregates

```rust
#[derive(sqlx::FromRow, Debug)]
struct SaleSummary {
    sale_count: i64,
    us_sale_count: i64,
    first_us_sale_at: Option<chrono::NaiveDateTime>,
}

let us_sale = Sale::region.eq("us");
let summary: SaleSummary = Sale::query()
    .select_only()
    .column_as(dbkit::func::count(Sale::id), "sale_count")
    .column_as(dbkit::func::count(Sale::id).filter(us_sale.clone()), "us_sale_count")
    .column_as(dbkit::func::min(Sale::created_at).filter(us_sale), "first_us_sale_at")
    .filter(Sale::amount.gt(0_i64))
    .into_model()
    .one(&db)
    .await?
    .expect("aggregate without GROUP BY returns one row");
```

Query-level `.filter(...)` limits the input to every aggregate. An aggregate's `.filter(...)`
limits only that aggregate and generates PostgreSQL `FILTER (WHERE ...)` syntax. When every
selected expression is an aggregate, PostgreSQL returns one summary row without `GROUP BY`.

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
- Aggregate helpers include `sum`, `count`, `min`, and `max`; each supports PostgreSQL `FILTER (WHERE ...)` via `.filter(...)`.
- `SUM` over integer columns returns `NUMERIC` in Postgres; use `BigDecimal` (or cast) for totals.
- Aggregations work across joins; order-by currently expects a real column/expr rather than an alias.
