# Mutations

## Insert

Use the generated insert type. This is usually the clearest path because required fields stay
explicit:

```rust
let created = User::insert(UserInsert {
    name: "Alex".to_string(),
    email: "a@b.com".to_string(),
})
.returning_all()
.one(&db)
.await?
.expect("inserted");
```

## Update

For one loaded row, prefer `into_active()`. It marks existing fields as unchanged, so the update
only writes fields you explicitly set or null out:

```rust
let mut active = created.into_active();
active.name = "Renamed".into();
let updated = active.update(&db).await?;
```

Use the query-builder update for bulk updates or conditional updates where loading the row first
would be the wrong shape:

```rust
let updated = User::update()
    .set(User::name, "Updated")
    .filter(User::id.eq(created.id))
    .returning_all()
    .all(&db)
    .await?;
```

## Delete

For one loaded row, use the active model delete:

```rust
let deleted = created.into_active().delete(&db).await?;
```

Use the query builder for bulk deletes or conditional deletes:

```rust
let deleted = User::delete()
    .filter(User::id.eq(created.id))
    .execute(&db)
    .await?;
```

## Save

Active model save is available when code genuinely needs one path that inserts new rows and updates
loaded rows:

```rust
let mut active = User::new_active();
active.name = "Saved".into();
active.email = "saved@db.com".into();
let created = active.save(&db).await?;

let mut active = created.into_active();
active.name = "Renamed".into();
let updated = active.save(&db).await?;
```

## Bulk Insert

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

## Insert Conflict Handling

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
