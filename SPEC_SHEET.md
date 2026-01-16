# Rust ORM (SQLModel-like) — Spec Sheet

This document specifies a Postgres-first, async, SQLModel-inspired ORM-ish library for Rust. It focuses on:

- **Model definition ergonomics** (derive + attributes)
- **Typed relations** (loaded/unloaded at the type level)
- **Query ergonomics** (SQLModel-like fluent API)
- **Async Postgres runtime** (SQLx-based)

The goal is to be practical and shippable, not academically perfect.

---

## 1) Goals

### Primary goals

- **SQLModel-like authoring**: define models once with fields + relationship annotations.
- **Async-first**: everything DB-related is `async`.
- **Postgres-only** (for v1): avoid cross-DB abstraction complexity.
- **Great relationship loading ergonomics**:
  - `selectinload` (2-query strategy) and `joinedload` (single-query join)
  - compile-time guarantees for “relation is loaded” vs “not loaded”
- **Strong typing**:
  - typed column expressions and filters
  - generated structs for query result shapes

### Non-goals (v1)

- Automatic schema migrations (may be integrated later)
- Cross-DB support (MySQL/SQLite)
- Full active-record style mutation on models (optional)
- Magic runtime reflection

---

## 2) Core UX (what using the library should feel like)

### Model definitions

Users define models in a single Rust `struct` with a derive macro:

```rust
#[derive(Debug, Model)]
struct User {
    #[key]
    #[autoincrement]
    id: i64,

    name: String,

    #[unique]
    email: String,

    #[has_many]
    todos: db::HasMany<Todo>,

    motto: Option<String>,
}

#[derive(Debug, Model)]
struct Todo {
    #[key]
    id: String,

    #[index]
    user_id: i64,

    #[belongs_to(key = user_id, references = id)]
    user: db::BelongsTo<User>,

    title: String,
}
```

### Querying

SQLModel-like fluent API:

```rust
let users = User::query()
    .filter(User::email.eq("a@b.com"))
    .limit(10)
    .all(&db)
    .await?;
```

Eager loading:

```rust
let users = User::query()
    .with(User::todos.selectin())
    .all(&db)
    .await?;

for u in &users {
    // `todos()` is sync because they are loaded
    for t in u.todos() {
        println!("{}", t.title);
    }
}
```

Joined load:

```rust
let users = User::query()
    .with(User::todos.joined())
    .all(&db)
    .await?;
```

Loading on-demand:

```rust
let user = User::by_id(1).one(&db).await?.unwrap();

// `todos()` not available; relation is NotLoaded
let user = user.load(User::todos).await?;

let titles: Vec<_> = user.todos().iter().map(|t| t.title.clone()).collect();
```

---

## 3) High-level design

### Crate layout

- `db` (runtime)
  - connection pool, transactions
  - query execution on Postgres
  - row mapping and relation loaders
- `db-derive` (proc-macro)
  - `#[derive(Model)]`
  - parses field attributes (`#[key]`, `#[has_many]`, etc.)
  - generates schema metadata and strongly typed query API
- `db-core` (shared)
  - core types: `NotLoaded`, `HasMany<T>`, `BelongsTo<T>`, `ManyToMany<T>`
  - expression AST (typed SQL builder)
  - query plan types: `Select`, `Join`, `Filter`, etc.

### Runtime backend

- **SQLx** for Postgres (async)
- Prefer `sqlx::FromRow` for row mapping
- Query string + bind values built at runtime (typed AST -> SQL + bind list)

---

## 4) Type system strategy for relations (loaded/unloaded)

### State marker

```rust
pub struct NotLoaded;
```

### Relation state types (preferred)

For each relation field, the generated runtime model uses a generic parameter that is either:

- `NotLoaded` (not fetched / unknown)
- the **loaded payload type** (e.g. `Vec<Todo>` for `HasMany`, `Option<User>` for `BelongsTo`)

This keeps the “loaded” representation clean and direct (`Vec<Todo>`, `Option<User>`) without any wrapper.

### Restricting allowed relation state types (required)

Because `UserModel<Todos = NotLoaded>` could otherwise be instantiated with *any* `Todos` type, the macro should generate a **relation-specific sealed trait** per relation.

Example (`User.todos: HasMany<Todo>`):

```rust
pub struct NotLoaded;

mod user_todos_state {
    use super::{NotLoaded, Todo};

    mod sealed {
        pub trait Sealed {}
        impl Sealed for NotLoaded {}
        impl Sealed for Vec<Todo> {}
    }

    pub trait State: sealed::Sealed {}
    impl State for NotLoaded {}
    impl State for Vec<Todo> {}
}

pub struct UserModel<
    Todos: user_todos_state::State = NotLoaded,
> {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub motto: Option<String>,

    pub todos: Todos,
}

pub type User = UserModel;
```

`BelongsTo<User>` would generate a similar sealed trait, but the loaded payload would typically be `Option<User>` (so it can represent “loaded but NULL”).

### Methods that exist only when loaded

```rust
impl UserModel<Vec<Todo>> {
    pub fn todos(&self) -> &[Todo] {
        &self.todos
    }
}
```

### Unloaded state: async loader method exists

```rust
impl UserModel<NotLoaded> {
    pub async fn load_todos(
        self,
        db: &db::Database,
    ) -> Result<UserModel<Vec<Todo>>, db::Error> {
        // selectin load via pk/fk
        todo!()
    }
}
```

### Nested/sub-loaded

Loaded payload types can themselves be type-state models, so nesting works naturally:

- `UserModel<Todos = Vec<TodoModel<UserRel = Option<User>>>>` (example)

No extra wrapper type is required beyond `NotLoaded` and the actual loaded payload container.

---

## 5) Relation field types in the user-facing schema

In the user-facing schema:

- `db::HasMany<T>` and `db::BelongsTo<T>` are declarative relation markers.
- The derive macro replaces them in generated runtime model as relation state parameters.

Example mapping:

```rust
// User schema input
struct User {
  #[has_many]
  todos: db::HasMany<Todo>,
}

// Generated model
struct UserModel<Todos = NotLoaded> {
  todos: Todos,
}
```

### Many-to-many (preferred client UX)

Many-to-many relations should be supported with a marker like:

```rust
#[derive(Debug, Model)]
struct Todo {
    #[key]
    id: i64,

    #[many_to_many(through = TodoTag, left_key = todo_id, right_key = tag_id)]
    tags: db::ManyToMany<Tag>,
}
```

Notes:

- The join table (e.g. `TodoTag`) is still a real table/model in the database schema, but client code should not need to reference it for common operations.
- `join(Todo::tags)` expands to two joins internally (`todo_tags` then `tags`).
- `with(Todo::tags.selectin())` loads tags via a select-in strategy over the join table.

### Relation descriptors

Derive generates relation descriptors as associated constants:

```rust
impl User {
    pub const todos: db::rel::HasMany<User, Todo> = /* ... */;
}
```

These descriptors are used for `.with(User::todos.selectin())` and `user.load(User::todos)`.

For many-to-many:

```rust
impl Todo {
    pub const tags: db::rel::ManyToMany<Todo, Tag> = /* ... */;
}
```

So client queries can stay clean:

```rust
let tags: Vec<Tag> = Tag::query()
    .join(Tag::todos)
    .join(Todo::user)
    .filter(User::is_admin.eq(true))
    .distinct()
    .all(&db)
    .await?;
```

In the user-facing schema:

- `db::HasMany<Todo>` and `db::BelongsTo<User>` are *declarative markers*.
- The derive macro replaces them in generated runtime model as relation state parameters.

Example mapping:

```rust
// User schema input
struct User {
  #[has_many]
  todos: db::HasMany<Todo>,
}

// Generated model
struct UserModel<Todos = NotLoaded> {
  todos: Todos,
}
```

### Relation descriptors

Derive generates relation descriptors as associated constants:

```rust
impl User {
    pub const todos: db::rel::HasMany<User, Todo> = /* ... */;
}
```

These descriptors are used for `.with(User::todos.selectin())` and `user.load(User::todos)`.

---

## 6) Foreign keys and primary keys (v1 simplification)

For v1, **do not use strongly typed ****Id\<Model>**** wrappers**.

- Primary keys and foreign keys are represented using their underlying scalar types (e.g. `i64`, `uuid::Uuid`, `String`).
- Relationships are still type-safe via relation descriptors generated by the macro (`#[has_many]`, `#[belongs_to]`).

Typed IDs can be added later as an optional feature once the core ORM and relation-loading ergonomics are stable.

---

## 7) Query API (SQLModel-ish)

### Design principles

- Fluent builder
- Typed column expressions
- Works without proc-macro DB-introspection (no build-time DB required)
- Optional advanced mode later using SQLx macros

### Basic query builder

Generated per model:

```rust
impl User {
    pub fn query() -> db::query::Select<User> {
        db::query::Select::new(User::TABLE)
    }

    pub fn by_id(id: i64) -> db::query::Select<User> {
        Self::query().filter(User::id.eq(id)).limit(1)
    }
}
```

### Columns

Derive generates typed columns as associated constants:

```rust
impl User {
    pub const TABLE: db::Table = db::Table::new("users");

    pub const id: db::Column<User, i64> = db::Column::new("id");
    pub const name: db::Column<User, String> = db::Column::new("name");
    pub const email: db::Column<User, String> = db::Column::new("email");
}
```

### Expressions

```rust
let expr = User::email.eq("a@b.com");
let expr2 = User::name.ilike("%alex%"));
let expr3 = User::id.in_([1,2,3]);
let expr4 = User::motto.is_null();
```

Expression traits:

- `Eq`, `Ne`, `Lt`, `Le`, `Gt`, `Ge`
- `Like`, `ILike`
- `In`
- `IsNull`, `IsNotNull`
- logical composition: `.and()`, `.or()`, `.not()`

### Executing

```rust
let all: Vec<User> = User::query().all(&db).await?;
let one: Option<User> = User::query().filter(...).one(&db).await?;
```

Return types:

- `.all()` returns `Vec<ModelDefaultState>` unless `.with()` changes output type
- `.one()` returns `Option<ModelDefaultState>`

### Joins vs eager loading (`join()` vs `with()`)

Two separate concepts are required for good ergonomics and correct typing:

1. ``

- Adds a SQL `JOIN` clause primarily for **filtering / ordering / existence checks**.
- **Does not** load relations into the returned model type.
- Output type typically stays the base model (e.g. `User`).

2. ``

- Declares an eager-load plan and **changes the output type** to a state where relations are loaded.

Example: join for filtering only (returns base `User`, todos are not loaded):

```rust
let users: Vec<User> = User::query()
    .join(User::todos)
    .filter(Todo::title.ilike("%milk%"))
    .distinct()
    .all(&db)
    .await?;
```

Example: join for filtering + eager load for typed output:

```rust
let users: Vec<UserModel<Vec<Todo>>> = User::query()
    .join(User::todos)
    .filter(Todo::title.ilike("%milk%"))
    .distinct()
    .with(User::todos.selectin())
    .all(&db)
    .await?;
```

---

## 8) Eager loading API

### Loader strategies

Support two strategies:

1. **Select-in load** (recommended default)

- Query parents
- Query children with `WHERE fk IN (...)`
- Attach results in memory

2. **Joined load**

- `LEFT JOIN` or `INNER JOIN`
- Decode flattened rows into tree

### User API

```rust
User::query().with(User::todos.selectin())
User::query().with(User::todos.joined())
```

Multiple loads:

```rust
User::query()
  .with(User::todos.selectin())
  .with(User::profile.joined())
```

### Nested / multi-level eager loading (key feature)

A relation load spec can contain nested load specs for the child model.

This must support:

- `HasMany -> BelongsTo`
- `HasMany -> HasMany`
- **HasMany -> ManyToMany**
- **ManyToMany -> BelongsTo/HasMany/ManyToMany**

The “double join” case (loading multiple nested relations for the same child) is a first-class feature.

A relation load spec can contain **nested load specs** for the child model.

Example: load `User -> todos` and for each `Todo` also load its `user` relation:

```rust
let users: Vec<UserModel<Vec<TodoModel<Option<User>>>>> = User::query()
    .with(User::todos.selectin().with(Todo::user.joined()))
    .all(&db)
    .await?;
```

Example: load `User -> todos` and for each `Todo` also load `tags`:

```rust
let users: Vec<UserModel<Vec<TodoModel<Vec<Tag>>>>> = User::query()
    .with(User::todos.selectin().with(Todo::tags.selectin()))
    .all(&db)
    .await?;
```

Example: load **multiple nested relations** into `Todo` ("double join"):

```rust
let users: Vec<UserModel<Vec<TodoModel<Option<User>, Vec<Tag>>>>> = User::query()
    .with(
        User::todos.selectin()
            .with(Todo::user.joined())
            .with(Todo::tags.selectin())
    )
    .all(&db)
    .await?;
```

Notes:

- Nested `.with(...)` applies to the **child output type**, not the parent.
- For best performance, prefer **select-in** for `HasMany` at deeper levels.
- Nested `joined()` for multiple `HasMany` can cause row explosion; it may still be supported but should be discouraged or guarded.

### Output typing

`with()` changes the output type parameters.

Example: `User::query().with(User::todos.selectin())` returns:

```rust
Vec<UserModel<Vec<Todo>>>
```

Nested example: `User::query().with(User::todos.selectin().with(Todo::tags.selectin()))` returns:

```rust
Vec<UserModel<Vec<TodoModel<Vec<Tag>>>>> 
```

Implementation detail:

- Query builder is generic over output type:

```rust
Select<Out>
```

- Each load spec `L` implements a trait like `ApplyLoad<Out>` with an associated `Out2`.
- Nested load specs are composed so the compiler can infer the correct final `Out2`.

---

## 9) Lazy loading via explicit upgrade

Provide an ergonomic explicit loader on instances:

```rust
let u0: User = ...; // NotLoaded
let u1 = u0.load(User::todos).await?; // Loaded
```

This should work by consuming `self` (so the type changes) or by returning a new value.

### Signature style

```rust
impl<P> UserModel<P> {
  pub async fn load(
      self,
      rel: db::rel::HasMany<User, Todo>,
  ) -> Result<UserModel<Vec<Todo>>, db::Error>;
}
```

For multiple relations:

```rust
let u = u.load(User::todos).await?;
let u = u.load(User::item).await?;
```

Later: consider tuple loading:

```rust
let u = u.load((User::todos, User::item)).await?;
```

---

## 10) Insert / Update / Delete

### Insert

```rust
let user = User::insert(UserInsert {
    name: "Alex".into(),
    email: "a@b.com".into(),
    motto: None,
})
.returning_all()
.one(&db)
.await?;
```

Derive generates `UserInsert` (no key, no relations):

- `id` excluded if autoincrement
- relation markers excluded

### Update

Prefer explicit update builder:

```rust
User::update()
  .set(User::name, "New".to_string())
  .filter(User::id.eq(1))
  .execute(&db)
  .await?;
```

Optionally: patch struct `UserPatch` with `Option<T>` fields.

### Delete

```rust
User::delete().filter(User::id.eq(1)).execute(&db).await?;
```

---

## 11) Constraints and indexes

From attributes:

- `#[key]`: primary key
- `#[autoincrement]`: sequence-backed identity
- `#[unique]`: unique constraint
- `#[index]`: **no-op metadata in v1** (kept for future migrations / linting)

In v1, these attributes primarily:

- influence insert/update APIs (e.g., key/autoincrement)
- feed metadata (e.g., `unique`, `index`) for future tooling

**Important (v1):** the library does **not** create or modify database indexes. `#[index]` is stored only as schema metadata.

Later: optional migration generator and/or schema validation tooling.

---

## 12) Postgres types

Support out-of-the-box:

- primitives: i16/i32/i64, f32/f64, bool
- String
- chrono / time (feature-gated)
- uuid (feature-gated)
- json/jsonb via `serde_json::Value` (feature-gated)

SQLx integration:

- require traits for encode/decode

---

## 13) Transactions and locking

### Transaction handle

```rust
let mut tx = db.begin().await?;
let user = User::by_id(1).for_update().one(&mut tx).await?;
...
tx.commit().await?;
```

Where `Executor` is generic:

```rust
async fn all<E: db::Executor>(self, ex: E) -> Result<Vec<T>, Error>;
```

Use SQLx executor patterns:

- `&Pool<Postgres>`
- `&mut Transaction<'_, Postgres>`

Add options:

- `FOR UPDATE`
- `SKIP LOCKED`
- `NOWAIT`

---

## 14) Error model

Single error enum:

```rust
pub enum Error {
  Sqlx(sqlx::Error),
  Decode(String),
  ConstraintViolation { constraint: String },
  NotFound,
  RelationMismatch,
}
```

- Provide `From<sqlx::Error>`
- Provide helpers for common errors

---

## 15) Macro output contract

For each model `X`, derive generates:

### Types

- `XModel<...defaults...>` (the runtime type-state model)
- `type X = XModel` alias for default state
- `XInsert` (insert struct)
- optional `XPatch` (patch struct)

### Schema metadata

- `X::TABLE`
- `X::id`, `X::name`, ... columns
- `X::PRIMARY_KEY`

### Relations

- `X::todos` relation descriptor(s)

### Query builder entrypoints

- `X::query()`
- `X::insert()` / `X::update()` / `X::delete()`
- `X::by_id()` convenience

### Internal mapping

- `impl sqlx::FromRow for XModel<...>` for base fields
- join decoding helpers for eager-loading

---

## 16) Expression / query AST (core types)

### Core traits

- `Expr<T>`: typed SQL expression producing `T`
- `Column<M, T>`: expression referencing a table column
- `Select<Out>`: select query builder

### SQL compilation

- AST -> `(String, Vec<BindValue>)`
- binder assigns `$1, $2, ...`
- must support nested boolean expressions

BindValue can be implemented using SQLx `Arguments`:

- compile into `sqlx::postgres::PgArguments`

---

## 17) Eager load implementation details

### Select-in load (HasMany)

Plan:

1. parent query: `SELECT users.* FROM users WHERE ...`
2. collect parent IDs
3. child query: `SELECT todos.* FROM todos WHERE user_id = ANY($1)`
4. group by `user_id`
5. attach into each `UserModel<Vec<Todo>>`

### Joined load

Plan:

- query: `SELECT u.*, t.* FROM users u LEFT JOIN todos t ON t.user_id = u.id`
- decode repeated parent columns
- group rows by parent primary key
- build `Vec<UserModel<Vec<Todo>>>`

Joined load should be optional because it can inflate row count dramatically.

---

## 18) Query result typing & combinators

### Key idea

The query builder is generic over the output model type.

```rust
struct Select<Out> { /* ... */ }

impl<Out> Select<Out> {
   pub fn filter(self, expr: db::expr::BoolExpr) -> Self { ... }
}

impl Select<User> {
   pub fn with(self, rel: db::load::TodosSelectin) -> Select<UserModel<Vec<Todo>>> { ... }
}
```

`with()` changes `Out`.

For multiple relations, chain recalls `with()` repeatedly.

### How `with()` ensures the correct output type

Each load spec `L` implements a trait that maps `Out -> Out2` at compile time:

```rust
trait ApplyLoad<Out> {
    type Out2;
    fn apply(self, q: Select<Out>) -> Select<Self::Out2>;
}

impl<Out> Select<Out> {
    pub fn with<L>(self, load: L) -> Select<L::Out2>
    where
        L: ApplyLoad<Out>,
    {
        load.apply(self)
    }
}
```

The derive macro generates these `ApplyLoad` impls for each relation + strategy, e.g.:

- `User::todos.selectin()` implements `ApplyLoad<User, Out2 = UserModel<Vec<Todo>>>`
- `Todo::tags.selectin()` implements `ApplyLoad<Todo, Out2 = TodoModel<Vec<Tag>>>`

Nested loads are composed by having the parent load spec carry a nested load spec for the child output type. This is what enables safe multi-level loads and “double join” relations at the type level.

---

## 19) Performance considerations

- Avoid allocations in query compilation where possible
- Prefer `selectinload` default to prevent explosion
- Provide `.with_joined()` explicitly
- Allow `.columns(...)` to limit returned columns
- Allow `.where_in_chunks(max = N)` for huge IN lists

---

## 20) Safety & correctness guarantees

### Compile-time

- `u.todos()` cannot compile unless loaded
- `with(User::todos)` changes output type accordingly

### Runtime

- relation attachment asserts parent key exists
- detect missing keys for loader

---

## 21) Minimal viable implementation plan

### Milestone 1: Base querying

- `Model` derive
- table + columns generation
- filter expressions
- `Select<ModelDefault>` + `.all/.one`

### Milestone 2: Insert/update/delete

- `Insert` builder + `InsertStruct`
- `Update` builder + simple sets
- `Delete` builder

### Milestone 3: Relations (HasMany + BelongsTo)

- relation descriptors
- selectin eager load
- per-instance `.load()`

### Milestone 3.5: Many-to-many (recommended early)

- `#[many_to_many(...)]` attribute + `db::ManyToMany<T>` marker

- `join(Model::rel)` expands through join table

- `with(Model::rel.selectin())` loads via select-in

- nested eager load support (`with(User::todos.selectin().with(Todo::tags.selectin()))`)

- relation descriptors

- selectin eager load

- per-instance `.load()`

### Milestone 4: Joined eager loading

- join SQL generation
- decode tree

### Milestone 5: Nested load typing

- `TodoModel<AuthorState>` style
- `with(User::todos.with(Todo::author))` API (optional)

---

## 22) Optional “nice-to-have” APIs

- `.first()` as alias for `.limit(1).one()`
- `.count()`
- `.exists()`
- `.order_by(User::name.asc())`
- `.distinct()`
- `.returning(User::id)`
- `.on_conflict_do_nothing()` / `.upsert()`
- `.paginate(page, per_page)` (offset or cursor-based)

---

## 23) Example end-to-end usage (target ergonomics)

```rust
let db = Database::connect("postgres://...").await?;

// basic
let u: Option<User> = User::query()
    .filter(User::email.eq("a@b.com"))
    .one(&db)
    .await?;

// eager load
let users = User::query()
    .with(User::todos.selectin())
    .all(&db)
    .await?;

for u in users {
    println!("{} has {} todos", u.name, u.todos().len());
}

// lazy load upgrade
let u = User::by_id(1).one(&db).await?.unwrap();
let u = u.load(User::todos).await?;
println!("{}", u.todos()[0].title);
```

---

## 24) Open design decisions (decide early)

1. **Naming**: `User` as default state type alias vs `UserRow`/`UserModel` naming.
2. **Default state meaning**: `User = UserModel<NotLoaded...>`.
3. **Expression system complexity**: minimal (filters + order) vs full SQL.
4. **DB type mapping**: sqlx `FromRow` derive vs custom decode.
5. **Macro design**:
   - generate big impl blocks per model
   - or generate separate modules (`mod user { ... }`).
6. **Join strategy as default**: `selectinload` default recommended.

---

## 25) Recommended naming conventions

- Marker types:

  - `NotLoaded`

- Relation descriptors:

  - `HasMany<User, Todo>`
  - `BelongsTo<Todo, User>`

- Loader strategies:

  - `.selectin()`
  - `.joined()`

- Loader methods:

  - `.load(User::todos)`

---

## 26) Notes for the implementing model/team

- Keep v1 scope small: HasMany + BelongsTo + selectinload first.
- Prefer explicit SQL compilation that’s easy to debug.
- Ensure generated SQL is viewable (e.g. `.debug_sql()` on query builder).
- Make error messages readable (macro-generated types can get huge).
- Provide `#[model(table = "users")]` override for naming.

---

### Appendix A: Attribute grammar (proposed)

Model-level:

- `#[model(table = "users", schema = "public")]`

Field-level:

- `#[key]`
- `#[autoincrement]`
- `#[unique]`
- `#[index]` *(no-op metadata in v1)*
- `#[has_many]`
- `#[belongs_to(key = user_id, references = id)]`

---

### Appendix B: “Unknown join state” convenience trait

If you want ergonomics like `fn f(u: &UserModel)` but for any state, generate:

```rust
trait UserBase {
    fn id(&self) -> i64;
    fn name(&self) -> &str;
}

impl<P, I> UserBase for UserModel<P, I> {
    fn id(&self) -> i64 { self.id }
    fn name(&self) -> &str { &self.name }
}

fn f(u: &impl UserBase) { ... }
```

This avoids “generic params everywhere” without losing generality.

