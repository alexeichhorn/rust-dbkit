# Relations

## Model Relations

```rust,ignore
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
    #[many_to_many(through = TodoTag, left_key = todo_id, right_key = tag_id)]
    tags: dbkit::ManyToMany<Tag>,
}

#[model(table = "tags")]
#[derive(Debug)]
struct Tag {
    #[key]
    id: i64,
    name: String,
    #[many_to_many(through = TodoTag, left_key = tag_id, right_key = todo_id)]
    todos: dbkit::ManyToMany<Todo>,
}

#[model(table = "todo_tags")]
#[derive(Debug)]
struct TodoTag {
    #[key]
    todo_id: i64,
    #[key]
    tag_id: i64,
}
```

## Eager Loading And Join Filtering

```rust,ignore
let users: Vec<User<Vec<Todo>>> = User::query()
    .with(User::todos.selectin())
    .all(&db)
    .await?;

let users: Vec<User<Vec<Todo>>> = User::query()
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

## Nested Eager Loading

Parent -> children -> grandchildren loading is reflected in the result type:

```rust,ignore
let users: Vec<User<Vec<Todo<dbkit::NotLoaded, Vec<Tag>>>>> = User::query()
    .filter(User::email.ilike("%@example.com"))
    .with(User::todos.selectin().with(Todo::tags.selectin()))
    .all(&db)
    .await?;

for user in &users {
    for todo in &user.todos {
        for tag in &todo.tags {
            println!("{} / {} / {}", user.name, todo.title, tag.name);
        }
    }
}
```

Child-to-parent loading works the same way, and the parent can load its graph too:

```rust,ignore
let todos: Vec<Todo<Option<User<Vec<Todo>>>, dbkit::NotLoaded>> = Todo::query()
    .with(Todo::user.selectin().with(User::todos.selectin()))
    .all(&db)
    .await?;

for todo in &todos {
    if let Some(owner) = &todo.user {
        println!(
            "{} belongs to {} with {} todos",
            todo.title,
            owner.name,
            owner.todos.len()
        );
    }
}
```

## Select-In vs Joined

```rust,ignore
// selectin = 1 query for parents, then 1 query per relation (per level)
let users: Vec<User<Vec<Todo>>> = User::query()
    .limit(10)
    .with(User::todos.selectin())
    .all(&db)
    .await?;

// joined = single SQL query with LEFT JOINs + row decoding
let users: Vec<User<Vec<Todo>>> = User::query()
    .with(User::todos.joined())
    .all(&db)
    .await?;
```

Notes:
- `selectin()` is best when you need stable parent pagination (`LIMIT`/`OFFSET`) or large child fan-out.
- `joined()` is best when you want a single query and you can tolerate row multiplication.
- If you filter on joined tables (e.g. `filter(Todo::title.eq("foo"))`), `joined()` will only load
  the matching child rows because the filter is part of the join query.

## Type-Level Loaded Relations

```rust,ignore
// `User` with default generic params is the bare row: all relations are `NotLoaded`.
fn accepts_unloaded(user: &User) {
    println!("{}", user.name);
}

// Use the model type's generic params to require loaded relations in APIs.
fn needs_loaded(user: &User<Vec<Todo>>) {
    // safe: todos are guaranteed to be loaded
    println!("todos: {}", user.todos.len());
}

// For multiple relations, generic params follow relation-field order.
// In this repo, `Todo` declares `user` then `tags`, so:
// - user loaded, tags not loaded => Todo<Option<User>, dbkit::NotLoaded>
// - user loaded, tags loaded     => Todo<Option<User>, Vec<Tag>>
//
// Nested loaded relations compose too:
// `User<Vec<Todo<Option<User>, Vec<Tag>>>>`
// (i.e., users with todos loaded, and each todo has its user + tags loaded)
```

## Explicit Load Afterwards

`dbkit` does not auto-fetch relations on field access. Load an unloaded relation explicitly with
`.load(...)`:

```rust,ignore
let user = User::by_id(1).one(&db).await?.unwrap();
let user = user.load(User::todos, &db).await?;
println!("todos: {}", user.todos.len());
```

## Unloading Relations

Use `.into()` when an API needs a model with fewer loaded relations. Scalar fields and relations
present in the target type are preserved; the other loaded relations are discarded:

```rust,ignore
let todo: Todo<Option<User>, Vec<Tag>> = load_todo(&db).await?;
let todo: Todo<Option<User>> = todo.into();
```
