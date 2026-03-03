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
        .all(&db)
        .await?;

    for u in &users {
        for t in &u.todos {
            println!("{}", t.title);
        }
    }

    let user = User::by_id(1).one(&db).await?.unwrap();
    let user = user.load(User::todos, &db).await?;
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
    .all(&db)
    .await?;
```

Migrations (optional, via `sqlx`):

```toml
# Cargo.toml
dbkit = { version = "0.1", features = ["migrations"] }
```

```rust
use dbkit::{Database, migrate::Migrator};

static MIGRATOR: Migrator = dbkit::sqlx::migrate!("./migrations");

let db = Database::connect("postgres://...").await?;
db.migrate(&MIGRATOR).await?;
```

`dbkit` keeps migration execution thin and delegates migration file parsing/running to `sqlx`.

Count / exists / pagination:

```rust
let total = User::query().count(&db).await?;
let exists = User::query()
    .filter(User::email.eq("a@b.com"))
    .exists(&db)
    .await?;

let page = User::query()
    .order_by(dbkit::Order::asc(User::id.as_ref()))
    .paginate(1, 20, &db)
    .await?;
println!("page {} of {}", page.page, page.total_pages());
```

Insert / update / delete:

```rust
let created = User::insert(UserInsert {
    name: "Alex".to_string(),
    email: "a@b.com".to_string(),
})
.returning_all()
.one(&db)
.await?
.expect("inserted");

let updated = User::update()
    .set(User::name, "Updated")
    .filter(User::id.eq(created.id))
    .returning_all()
    .all(&db)
    .await?;

let deleted = User::delete()
    .filter(User::id.eq(created.id))
    .execute(&db)
    .await?;
```

Bulk insert:

```rust
let inserted = User::insert_many(vec![
    UserInsert {
        name: "Alpha".to_string(),
        email: "alpha@db.com".to_string(),
    },
    UserInsert {
        name: "Beta".to_string(),
        email: "beta@db.com".to_string(),
    },
])
.execute(&db)
.await?;
assert_eq!(inserted, 2);
```

Insert conflict handling (`ON CONFLICT`):

```rust
let ignored = User::insert(UserInsert {
    name: "Alex".to_string(),
    email: "a@b.com".to_string(),
})
.on_conflict_do_nothing(User::email)
.execute(&db)
.await?;

let updated = OrderLine::insert(OrderLineInsert {
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

Active model insert / update (change-tracked):

```rust
let mut active = User::new_active();
active.name = "Active".into();
active.email = "active@db.com".into();

let created = active.insert(&db).await?;

let mut active = created.into_active();
active.name = "Updated".into();
let updated = active.update(&db).await?;
```

Note: `into_active()` marks fields as unchanged. Updates only include fields you explicitly set
(`ActiveValue::Set`) or null out (`ActiveValue::Null`), so existing values aren’t overwritten.

Active model save (insert vs update):

```rust
let mut active = User::new_active();
active.name = "Saved".into();
active.email = "saved@db.com".into();
let created = active.save(&db).await?;

let mut active = created.into_active();
active.name = "Renamed".into();
let updated = active.save(&db).await?;
```

Eager loading and join filtering:

```rust
let users: Vec<UserModel<Vec<Todo>>> = User::query()
    .with(User::todos.selectin())
    .all(&db)
    .await?;

let users: Vec<UserModel<Vec<Todo>>> = User::query()
    .with(User::todos.joined())
    .all(&db)
    .await?;

let filtered = User::query()
    .join(User::todos)
    .filter(Todo::title.eq("Keep me"))
    .distinct()
    .all(&db)
    .await?;
```

Select-in vs joined eager loading:

```rust
// selectin = 1 query for parents, then 1 query per relation (per level)
let users: Vec<UserModel<Vec<Todo>>> = User::query()
    .limit(10)
    .with(User::todos.selectin())
    .all(&db)
    .await?;

// joined = single SQL query with LEFT JOINs + row decoding
let users: Vec<UserModel<Vec<Todo>>> = User::query()
    .with(User::todos.joined())
    .all(&db)
    .await?;
```

Notes:
- `selectin()` is best when you need stable parent pagination (`LIMIT`/`OFFSET`) or large child fan-out.
- `joined()` is best when you want a single query and you can tolerate row multiplication.
- If you filter on joined tables (e.g. `filter(Todo::title.eq("foo"))`), `joined()` will only load
  the matching child rows because the filter is part of the join query.

Dynamic conditions:

```rust
let mut cond = dbkit::Condition::any()
    .add(User::region.eq("us"))
    .add(User::region.is_null().and(Creator::region.eq("us")));

if let Some(expr) = cond.into_expr() {
    query = query.filter(expr);
}
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
let user = User::by_id(1).one(&db).await?.unwrap();
let user = user.load(User::todos, &db).await?;
println!("todos: {}", user.todos.len());
```

Aggregation and projections:

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

SQL functions and expression-based grouping:

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

Join + aggregation:

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
- `SUM` over integer columns returns `NUMERIC` in Postgres; use `BigDecimal` (or cast) for totals.
- Aggregations work across joins; order-by currently expects a real column/expr rather than an alias.

NULL handling with `Option<T>`:

```rust
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

Transactions:

```rust
let tx = db.begin().await?;
let users = User::query().all(&tx).await?;
tx.commit().await?;
```

## TODOs

- [x] Implement true joined eager loading (single-query join decoding).
- [x] Add aggregation/projection support: `select_only`, `column_as`, `group_by`, `sum`, `count`, and mapping into custom result structs (e.g., `into_model::<T>()` for aggregates).
- [x] Add SQL function expressions in queries (e.g., `COALESCE`, `DATE_TRUNC`, `UPPER`).
- [x] Add JSON column support (`serde_json::Value`) for insert/update/filter.
- [x] Add Postgres array column support (e.g., `Vec<String>`) for insert/update/filter.
- [x] Add bulk insert support (multi-row `insert_many`).
- [x] Add dynamic condition builder helpers (e.g., `Condition::any` / `Condition::all`).
- [x] Allow `order_by` on expressions or aliases (e.g., `date_trunc(...)`, `total`).
- [x] Add `between(a, b)` convenience for columns/expressions.
- [ ] Add locking options: `for_update`, `skip_locked`, `nowait`.
- [x] Add optional helpers: `count()`, `exists()`, `paginate()`.
- [x] Add typed conflict helpers: `on_conflict_do_nothing`, `on_conflict_do_update`.
- [x] Add ActiveModel `save()` that chooses insert vs update.
- [ ] Store `#[unique]` / `#[index]` as metadata (even if no-op).

## Deviations from spec

- `load(...)` requires an executor argument: `user.load(User::todos, &ex)`.
- Relation state sealing is looser than spec (any `Vec<T>` / `Option<T>` satisfies the state trait).
