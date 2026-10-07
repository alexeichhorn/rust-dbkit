# Querying

## Basic Query + Ordering

```rust,ignore
use dbkit::prelude::*;

let users = User::query()
    .filter(User::email.ilike("%@example.com"))
    .order_by(dbkit::Order::asc(User::name.as_ref()))
    .limit(20)
    .all(&db)
    .await?;
```

## Row-Value Filters

```rust,ignore
let rows = LookupRow::query()
    .filter(dbkit::row((LookupRow::scope, LookupRow::external_key, LookupRow::locale)).in_([
        (LookupScope::Public, "alpha", "en"),
        (LookupScope::Internal, "beta", "de"),
    ]))
    .all(&db)
    .await?;
```

## Streaming

`.stream(&db)` yields rows one at a time instead of collecting a `Vec`:

```rust,ignore
use dbkit::prelude::*;
use futures_util::TryStreamExt;

let mut users = User::query()
    .filter(User::email.ilike("%@example.com"))
    .order_by(dbkit::Order::asc(User::id))
    .stream(&db);

while let Some(user) = users.try_next().await? {
    println!("{}", user.email);
}
```

The query runs once, on the first poll. Each item is a `Result`, and the stream
ends after the last row or the first error. Query-building methods work as with
`.all()`, but `.with(...)` eager loading isn't supported.

Rows arrive through a cursor in batches of 1,000, so PostgreSQL only computes
what you read, plus at most one batch. Dropping the stream early skips the rest
of the query. If the query fails mid-way, rows in the failing batch are not
yielded. On a pool, the stream runs inside its own transaction, committed once
the stream is exhausted.

The stream holds a connection until it's exhausted or dropped, so drop it after
an early `break`. If you only need the first rows, `.limit(n)` is still better:
PostgreSQL can plan for it and the connection is freed right away. For slow or
resumable jobs, fetch batches keyed on the last-seen ID instead.

Streams also work inside a transaction. Don't run other queries on it while the
stream is active, and drop the stream before committing:

```rust,ignore
let tx = db.begin().await?;
{
    let mut users = User::query().stream(&tx);
    while let Some(user) = users.try_next().await? {
        println!("{}", user.email);
    }
}
tx.commit().await?;
```

## Row Locking

```rust,ignore
let rows = User::query().for_update().all(&tx).await?;
let rows = User::query().for_update().skip_locked().all(&tx).await?;
let rows = User::query().for_update().nowait().all(&tx).await?;
```

## Count / Exists / Pagination

```rust,ignore
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

## Correlated EXISTS / NOT EXISTS

```rust,ignore
let active_projects = Project::query()
    .where_exists(
        Task::query()
            .select_only()
            .column(Task::id)
            .filter(Task::project_id.eq_col(Project::id))
            .filter(Task::state.eq("active")),
    )
    .order_by(dbkit::Order::asc(Project::id))
    .all(&db)
    .await?;

let projects_without_archived_tasks = Project::query()
    .where_not_exists(
        Task::query()
            .select_only()
            .column(Task::id)
            .filter(Task::project_id.eq_col(Project::id))
            .filter(Task::state.eq("archived")),
    )
    .all(&db)
    .await?;

let archived_tasks = Task::delete()
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

## Dynamic Conditions

```rust,ignore
let mut cond = dbkit::Condition::any()
    .add(User::region.eq("us"))
    .add(User::region.is_null().and(Creator::region.eq("us")));

if let Some(expr) = cond.into_expr() {
    query = query.filter(expr);
}
```

## Column-To-Column Comparisons

```rust,ignore
let changed = Job::query()
    .filter(Job::content_hash.ne_col(Job::last_content_hash))
    .all(&db)
    .await?;

let retryable = Job::query()
    .filter(Job::retry_count.lt_col(Job::max_retries))
    .all(&db)
    .await?;
```

Supported column comparison helpers:

- `eq_col`
- `ne_col`
- `is_distinct_from_col`
- `is_not_distinct_from_col`
- `lt_col`
- `le_col`
- `gt_col`
- `ge_col`

Stale-embedding predicate with nullable hashes:

```rust,ignore
let stale = Job::query()
    .filter(
        Job::embedding
            .is_null()
            .or(Job::embedding_hash.is_null())
            .or(dbkit::func::coalesce(Job::embedding_hash, "").ne_col(Job::content_hash)),
    )
    .all(&db)
    .await?;
```

Null-safe hash mismatch with Postgres `IS DISTINCT FROM` semantics:

```rust,ignore
let stale = Job::query()
    .filter(
        Job::embedding
            .is_null()
            .or(Job::embedding_hash.is_distinct_from_col(Job::content_hash)),
    )
    .all(&db)
    .await?;
```
