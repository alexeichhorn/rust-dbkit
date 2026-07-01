# Querying

## Basic Query + Ordering

```rust
use dbkit::prelude::*;

let users = User::query()
    .filter(User::email.ilike("%@example.com"))
    .order_by(dbkit::Order::asc(User::name.as_ref()))
    .limit(20)
    .all(&db)
    .await?;
```

## Row-Value Filters

```rust
let rows = LookupRow::query()
    .filter(dbkit::row((LookupRow::scope, LookupRow::external_key, LookupRow::locale)).in_([
        (LookupScope::Public, "alpha", "en"),
        (LookupScope::Internal, "beta", "de"),
    ]))
    .all(&db)
    .await?;
```

## Row Locking

```rust
let rows = User::query().for_update().all(&tx).await?;
let rows = User::query().for_update().skip_locked().all(&tx).await?;
let rows = User::query().for_update().nowait().all(&tx).await?;
```

## Count / Exists / Pagination

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

## Correlated EXISTS / NOT EXISTS

```rust
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

```rust
let mut cond = dbkit::Condition::any()
    .add(User::region.eq("us"))
    .add(User::region.is_null().and(Creator::region.eq("us")));

if let Some(expr) = cond.into_expr() {
    query = query.filter(expr);
}
```

## Column-To-Column Comparisons

```rust
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

```rust
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

```rust
let stale = Job::query()
    .filter(
        Job::embedding
            .is_null()
            .or(Job::embedding_hash.is_distinct_from_col(Job::content_hash)),
    )
    .all(&db)
    .await?;
```
