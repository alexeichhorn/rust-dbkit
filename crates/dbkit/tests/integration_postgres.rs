#![allow(non_upper_case_globals)]

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::prelude::*;
use dbkit::{model, Database};
use serde_json::json;
use uuid::Uuid;

#[model(table = "users")]
pub struct User {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
    pub email: String,
    #[has_many]
    pub todos: dbkit::HasMany<Todo>,
}

#[model(table = "todos")]
pub struct Todo {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    #[belongs_to(key = user_id, references = id)]
    pub user: dbkit::BelongsTo<User>,
    #[many_to_many(through = TodoTag, left_key = todo_id, right_key = tag_id)]
    pub tags: dbkit::ManyToMany<Tag>,
}

#[model(table = "nullable_rows")]
pub struct NullableRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub note: Option<String>,
}

#[model(table = "events")]
pub struct Event {
    #[key]
    pub id: Uuid,
    pub name: String,
    pub starts_at: NaiveDateTime,
    pub day: NaiveDate,
    pub starts_at_time: NaiveTime,
}

#[model(table = "tags")]
pub struct Tag {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
    #[many_to_many(through = TodoTag, left_key = tag_id, right_key = todo_id)]
    pub todos: dbkit::ManyToMany<Todo>,
}

#[model(table = "profiles")]
pub struct Profile {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub tags: Vec<String>,
}

#[model(table = "json_rows")]
pub struct JsonRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub data: serde_json::Value,
}

#[model(table = "func_rows")]
pub struct FuncRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub email: Option<String>,
    pub backup_email: Option<String>,
    pub region: Option<String>,
    pub starts_at: NaiveDateTime,
}

#[model(table = "sales")]
pub struct Sale {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub region: String,
    pub amount: i64,
    pub created_at: NaiveDateTime,
}

#[model(table = "todo_tags")]
pub struct TodoTag {
    #[key]
    pub todo_id: i64,
    #[key]
    pub tag_id: i64,
}

#[model(table = "order_lines")]
pub struct OrderLine {
    #[key]
    pub order_id: i64,
    #[key]
    pub line_id: i64,
    pub note: String,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
) -> Result<(), dbkit::Error> {
    dbkit::sqlx::query(
        "CREATE TEMP TABLE users (\
            id BIGSERIAL PRIMARY KEY,\
            name TEXT NOT NULL,\
            email TEXT NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE todos (\
            id BIGSERIAL PRIMARY KEY,\
            user_id BIGINT NOT NULL,\
            title TEXT NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE tags (\
            id BIGSERIAL PRIMARY KEY,\
            name TEXT NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE profiles (\
            id BIGSERIAL PRIMARY KEY,\
            tags TEXT[] NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE json_rows (\
            id BIGSERIAL PRIMARY KEY,\
            data JSONB NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE func_rows (\
            id BIGSERIAL PRIMARY KEY,\
            email TEXT,\
            backup_email TEXT,\
            region TEXT,\
            starts_at TIMESTAMP NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE sales (\
            id BIGSERIAL PRIMARY KEY,\
            region TEXT NOT NULL,\
            amount BIGINT NOT NULL,\
            created_at TIMESTAMP NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE todo_tags (\
            todo_id BIGINT NOT NULL,\
            tag_id BIGINT NOT NULL,\
            PRIMARY KEY (todo_id, tag_id)\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE events (\
            id UUID PRIMARY KEY,\
            name TEXT NOT NULL,\
            starts_at TIMESTAMP NOT NULL,\
            day DATE NOT NULL,\
            starts_at_time TIME NOT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE nullable_rows (\
            id BIGSERIAL PRIMARY KEY,\
            note TEXT NULL\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    dbkit::sqlx::query(
        "CREATE TEMP TABLE order_lines (\
            order_id BIGINT NOT NULL,\
            line_id BIGINT NOT NULL,\
            note TEXT NOT NULL,\
            PRIMARY KEY (order_id, line_id)\
        )",
    )
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

async fn seed_user(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    name: &str,
    email: &str,
) -> Result<User, dbkit::Error> {
    let user = User::insert(UserInsert {
        name: name.to_string(),
        email: email.to_string(),
    })
    .returning_all()
    .one(&mut *tx)
    .await?
    .expect("inserted user");
    Ok(user)
}

async fn seed_todo(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    user_id: i64,
    title: &str,
) -> Result<Todo, dbkit::Error> {
    let todo = Todo::insert(TodoInsert {
        user_id,
        title: title.to_string(),
    })
    .returning_all()
    .one(&mut *tx)
    .await?
    .expect("inserted todo");
    Ok(todo)
}

async fn seed_tag(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    name: &str,
) -> Result<Tag, dbkit::Error> {
    let tag = Tag::insert(TagInsert {
        name: name.to_string(),
    })
    .returning_all()
    .one(&mut *tx)
    .await?
    .expect("inserted tag");
    Ok(tag)
}

async fn seed_todo_tag(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    todo_id: i64,
    tag_id: i64,
) -> Result<TodoTag, dbkit::Error> {
    let row = TodoTag::insert(TodoTagInsert { todo_id, tag_id })
        .returning_all()
        .one(&mut *tx)
        .await?
        .expect("inserted todo_tag");
    Ok(row)
}

async fn seed_event(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    id: Uuid,
    name: &str,
    starts_at: NaiveDateTime,
    day: NaiveDate,
    starts_at_time: NaiveTime,
) -> Result<Event, dbkit::Error> {
    let event = Event::insert(EventInsert {
        id,
        name: name.to_string(),
        starts_at,
        day,
        starts_at_time,
    })
    .returning_all()
    .one(&mut *tx)
    .await?
    .expect("inserted event");
    Ok(event)
}

async fn seed_nullable_row(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    note: Option<String>,
) -> Result<NullableRow, dbkit::Error> {
    let row = NullableRow::insert(NullableRowInsert { note })
        .returning_all()
        .one(&mut *tx)
        .await?
        .expect("inserted nullable row");
    Ok(row)
}

async fn seed_order_line(
    tx: &mut dbkit::sqlx::Transaction<'_, dbkit::sqlx::Postgres>,
    order_id: i64,
    line_id: i64,
    note: &str,
) -> Result<OrderLine, dbkit::Error> {
    let row = OrderLine::insert(OrderLineInsert {
        order_id,
        line_id,
        note: note.to_string(),
    })
    .returning_all()
    .one(&mut *tx)
    .await?
    .expect("inserted order line");
    Ok(row)
}

#[tokio::test]
async fn insert_update_delete_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Alex", "a@b.com").await?;
    assert!(user.id > 0);
    assert_eq!(user.name, "Alex");

    let updated = User::update()
        .set(User::name, "Updated")
        .filter(User::id.eq(user.id))
        .returning_all()
        .all(&mut tx)
        .await?;
    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].name, "Updated");

    let deleted = User::delete()
        .filter(User::id.eq(user.id))
        .execute(&mut tx)
        .await?;
    assert_eq!(deleted, 1);

    let remaining = User::query().all(&mut tx).await?;
    assert!(remaining.is_empty());

    Ok(())
}

#[tokio::test]
async fn insert_many_inserts_multiple_rows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

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
    .execute(&mut tx)
    .await?;
    assert_eq!(inserted, 2);

    let users = User::query()
        .order_by(dbkit::Order::asc(User::id.as_ref()))
        .all(&mut tx)
        .await?;
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Alpha");
    assert_eq!(users[1].name, "Beta");

    Ok(())
}

#[tokio::test]
async fn selectin_has_many_loads_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Riley", "r@b.com").await?;
    let _todo1 = seed_todo(&mut tx, user.id, "Write tests").await?;
    let _todo2 = seed_todo(&mut tx, user.id, "Ship code").await?;

    let users: Vec<UserModel<Vec<Todo>>> = User::query()
        .filter(User::id.eq(user.id))
        .with(User::todos.selectin())
        .all(&mut tx)
        .await?;

    assert_eq!(users.len(), 1);
    let mut titles: Vec<String> = users[0]
        .todos
        .iter()
        .map(|todo| todo.title.clone())
        .collect();
    titles.sort();
    assert_eq!(titles, vec!["Ship code", "Write tests"]);

    Ok(())
}

#[tokio::test]
async fn selectin_belongs_to_loads_parent() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Dana", "d@b.com").await?;
    let todo = seed_todo(&mut tx, user.id, "Map relations").await?;

    let todos: Vec<TodoModel<Option<User>>> = Todo::query()
        .filter(Todo::id.eq(todo.id))
        .with(Todo::user.selectin())
        .all(&mut tx)
        .await?;

    assert_eq!(todos.len(), 1);
    let loaded = todos[0].user.as_ref().expect("loaded user");
    assert_eq!(loaded.id, user.id);
    assert_eq!(loaded.email, "d@b.com");

    Ok(())
}

#[tokio::test]
async fn nested_selectin_loads() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Jo", "jo@b.com").await?;
    let _todo = seed_todo(&mut tx, user.id, "Chain loads").await?;

    let users = User::query()  // should be Vec<UserModel<Vec<TodoModel<Option<User>>>>>
        .filter(User::id.eq(user.id))
        .with(User::todos.selectin().with(Todo::user.selectin()))
        .all(&mut tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].todos.len(), 1);
    let nested_user = users[0].todos[0].user.as_ref().expect("nested user");
    assert_eq!(nested_user.id, user.id);

    Ok(())
}

#[tokio::test]
async fn lazy_load_relation() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Ari", "ari@b.com").await?;
    let _todo = seed_todo(&mut tx, user.id, "Lazy load").await?;

    let loaded = user.load(User::todos, &mut tx).await?;
    assert_eq!(loaded.todos.len(), 1);
    assert_eq!(loaded.todos[0].title, "Lazy load");

    Ok(())
}

#[tokio::test]
async fn join_filter_on_child_table() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user_keep = seed_user(&mut tx, "Keep", "keep@db.com").await?;
    let user_drop = seed_user(&mut tx, "Drop", "drop@db.com").await?;
    let _todo_keep = seed_todo(&mut tx, user_keep.id, "Keep me").await?;
    let _todo_other = seed_todo(&mut tx, user_keep.id, "Also me").await?;
    let _todo_drop = seed_todo(&mut tx, user_drop.id, "Ignore me").await?;

    let users = User::query()
        .join(User::todos)
        .filter(Todo::title.eq("Keep me"))
        .distinct()
        .all(&mut tx)
        .await?;

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].id, user_keep.id);

    Ok(())
}

#[tokio::test]
async fn uuid_date_time_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let date = NaiveDate::from_ymd_opt(2024, 1, 2).expect("date");
    let time = NaiveTime::from_hms_opt(3, 4, 5).expect("time");
    let starts_at = NaiveDateTime::new(date, time);
    let id = Uuid::nil();

    let inserted = seed_event(&mut tx, id, "Launch", starts_at, date, time).await?;
    assert_eq!(inserted.id, id);
    assert_eq!(inserted.starts_at, starts_at);
    assert_eq!(inserted.day, date);
    assert_eq!(inserted.starts_at_time, time);

    let found = Event::query()
        .filter(Event::id.eq(id))
        .filter(Event::day.eq(date))
        .filter(Event::starts_at.eq(starts_at))
        .filter(Event::starts_at_time.eq(time))
        .one(&mut tx)
        .await?
        .expect("event");

    assert_eq!(found.id, id);
    assert_eq!(found.name, "Launch");
    assert_eq!(found.starts_at, starts_at);
    assert_eq!(found.day, date);
    assert_eq!(found.starts_at_time, time);

    Ok(())
}

#[tokio::test]
async fn insert_update_and_filter_nulls() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let inserted = seed_nullable_row(&mut tx, None).await?;
    assert!(inserted.note.is_none());

    let some_row = seed_nullable_row(&mut tx, Some("hello".to_string())).await?;
    assert_eq!(some_row.note.as_deref(), Some("hello"));

    let updated = NullableRow::update()
        .set(NullableRow::note, None)
        .filter(NullableRow::id.eq(some_row.id))
        .returning_all()
        .all(&mut tx)
        .await?;
    assert_eq!(updated.len(), 1);
    assert!(updated[0].note.is_none());

    let null_rows = NullableRow::query()
        .filter(NullableRow::note.eq(None))
        .all(&mut tx)
        .await?;
    assert_eq!(null_rows.len(), 2);
    assert!(null_rows.iter().all(|row| row.note.is_none()));

    Ok(())
}

#[tokio::test]
async fn array_column_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let tags = vec!["alpha".to_string(), "beta".to_string()];
    let inserted = Profile::insert(ProfileInsert { tags: tags.clone() })
        .returning_all()
        .one(&mut tx)
        .await?
        .expect("inserted profile");
    assert_eq!(inserted.tags, tags);

    let matched = Profile::query()
        .filter(Profile::tags.eq(tags.clone()))
        .all(&mut tx)
        .await?;
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id, inserted.id);

    let updated_tags = vec!["gamma".to_string(), "delta".to_string()];
    let mut active = inserted.into_active();
    active.tags = updated_tags.clone().into();
    let updated = active.update(&mut tx).await?;
    assert_eq!(updated.tags, updated_tags);

    let fetched = Profile::by_id(updated.id)
        .one(&mut tx)
        .await?
        .expect("updated profile");
    assert_eq!(fetched.tags, updated_tags);

    Ok(())
}

#[tokio::test]
async fn json_column_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let payload = json!({"name": "alpha", "active": true});
    let inserted = JsonRow::insert(JsonRowInsert { data: payload.clone() })
        .returning_all()
        .one(&mut tx)
        .await?
        .expect("inserted json row");
    assert_eq!(inserted.data, payload);

    let matched = JsonRow::query()
        .filter(JsonRow::data.eq(payload.clone()))
        .all(&mut tx)
        .await?;
    assert_eq!(matched.len(), 1);
    assert_eq!(matched[0].id, inserted.id);

    let updated_payload = json!({"name": "beta", "active": false});
    let mut active = inserted.into_active();
    active.data = updated_payload.clone().into();
    let updated = active.update(&mut tx).await?;
    assert_eq!(updated.data, updated_payload);

    let fetched = JsonRow::by_id(updated.id)
        .one(&mut tx)
        .await?
        .expect("updated json row");
    assert_eq!(fetched.data, updated_payload);

    Ok(())
}

#[tokio::test]
async fn function_expressions_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 1, 2).expect("day");
    let day_start = NaiveDateTime::new(day, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));
    let later_day = NaiveDate::from_ymd_opt(2024, 1, 3).expect("day");
    let later_start =
        NaiveDateTime::new(later_day, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));

    let row1 = FuncRow::insert(FuncRowInsert {
        email: Some("alpha@ex.com".to_string()),
        backup_email: None,
        region: Some("us".to_string()),
        starts_at: NaiveDateTime::new(day, NaiveTime::from_hms_opt(10, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&mut tx)
    .await?
    .expect("row1");

    let row2 = FuncRow::insert(FuncRowInsert {
        email: None,
        backup_email: Some("beta@ex.com".to_string()),
        region: Some("eu".to_string()),
        starts_at: NaiveDateTime::new(day, NaiveTime::from_hms_opt(12, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&mut tx)
    .await?
    .expect("row2");

    let row3 = FuncRow::insert(FuncRowInsert {
        email: None,
        backup_email: None,
        region: Some("uk".to_string()),
        starts_at: NaiveDateTime::new(later_day, NaiveTime::from_hms_opt(9, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&mut tx)
    .await?
    .expect("row3");

    let row4 = FuncRow::insert(FuncRowInsert {
        email: Some("gamma@ex.com".to_string()),
        backup_email: Some("backup@ex.com".to_string()),
        region: None,
        starts_at: NaiveDateTime::new(later_day, NaiveTime::from_hms_opt(15, 0, 0).expect("time")),
    })
    .returning_all()
    .one(&mut tx)
    .await?
    .expect("row4");

    let upper_match = FuncRow::query()
        .filter(dbkit::func::upper(dbkit::func::coalesce(
            FuncRow::email,
            FuncRow::backup_email,
        ))
        .eq("BETA@EX.COM"))
        .all(&mut tx)
        .await?;
    assert_eq!(upper_match.len(), 1);
    assert_eq!(upper_match[0].id, row2.id);

    let fallback_match = FuncRow::query()
        .filter(dbkit::func::coalesce(FuncRow::email, "fallback").eq("fallback"))
        .all(&mut tx)
        .await?;
    let mut fallback_ids: Vec<i64> = fallback_match.iter().map(|row| row.id).collect();
    fallback_ids.sort();
    assert_eq!(fallback_ids, vec![row2.id, row3.id]);

    let nested_match = FuncRow::query()
        .filter(
            dbkit::func::coalesce(
                dbkit::func::coalesce(FuncRow::email, FuncRow::backup_email),
                "none",
            )
            .eq("none"),
        )
        .all(&mut tx)
        .await?;
    assert_eq!(nested_match.len(), 1);
    assert_eq!(nested_match[0].id, row3.id);

    let truncated_match = FuncRow::query()
        .filter(dbkit::func::date_trunc("day", FuncRow::starts_at).eq(day_start))
        .all(&mut tx)
        .await?;
    let mut day_ids: Vec<i64> = truncated_match.iter().map(|row| row.id).collect();
    day_ids.sort();
    assert_eq!(day_ids, vec![row1.id, row2.id]);

    let region_match = FuncRow::query()
        .filter(
            dbkit::func::upper(dbkit::func::coalesce(FuncRow::region, "unknown"))
                .eq("UNKNOWN"),
        )
        .all(&mut tx)
        .await?;
    assert_eq!(region_match.len(), 1);
    assert_eq!(region_match[0].id, row4.id);

    let combined_match = FuncRow::query()
        .filter(
            dbkit::func::upper(dbkit::func::coalesce(
                FuncRow::email,
                FuncRow::backup_email,
            ))
            .eq("ALPHA@EX.COM"),
        )
        .filter(dbkit::func::date_trunc("day", FuncRow::starts_at).eq(day_start))
        .all(&mut tx)
        .await?;
    assert_eq!(combined_match.len(), 1);
    assert_eq!(combined_match[0].id, row1.id);

    let _ = later_start;

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct RegionAgg {
    region: String,
    total: dbkit::sqlx::types::BigDecimal,
    count: i64,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct BucketAgg {
    bucket: NaiveDateTime,
    total: dbkit::sqlx::types::BigDecimal,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct UserTodoAgg {
    name: String,
    todo_count: i64,
}

#[tokio::test]
async fn aggregation_and_group_by_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let day1 = NaiveDate::from_ymd_opt(2024, 2, 1).expect("day");
    let day2 = NaiveDate::from_ymd_opt(2024, 2, 2).expect("day");
    let day1_start = NaiveDateTime::new(day1, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));
    let day2_start = NaiveDateTime::new(day2, NaiveTime::from_hms_opt(0, 0, 0).expect("time"));

    let inserted = Sale::insert_many(vec![
        SaleInsert {
            region: "us".to_string(),
            amount: 40,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(10, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "us".to_string(),
            amount: 70,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(12, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "eu".to_string(),
            amount: 30,
            created_at: NaiveDateTime::new(day1, NaiveTime::from_hms_opt(14, 0, 0).expect("time")),
        },
        SaleInsert {
            region: "apac".to_string(),
            amount: 200,
            created_at: NaiveDateTime::new(day2, NaiveTime::from_hms_opt(9, 0, 0).expect("time")),
        },
    ])
    .execute(&mut tx)
    .await?;
    assert_eq!(inserted, 4);

    let day1_end = NaiveDateTime::new(day1, NaiveTime::from_hms_opt(23, 59, 59).expect("time"));

    let mut amount_between = Sale::query()
        .filter(Sale::amount.between(40_i64, 70_i64))
        .all(&mut tx)
        .await?;
    amount_between.sort_by(|a, b| a.amount.cmp(&b.amount));
    assert_eq!(amount_between.len(), 2);
    assert_eq!(amount_between[0].amount, 40);
    assert_eq!(amount_between[1].amount, 70);

    let day1_sales = Sale::query()
        .filter(Sale::created_at.between(day1_start, day1_end))
        .all(&mut tx)
        .await?;
    assert_eq!(day1_sales.len(), 3);

    let mut region_rows: Vec<RegionAgg> = Sale::query()
        .select_only()
        .column(Sale::region)
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .column_as(dbkit::func::count(Sale::id), "count")
        .group_by(Sale::region)
        .order_by(dbkit::Order::asc(Sale::region.as_ref()))
        .having(dbkit::func::sum(Sale::amount).gt(100_i64))
        .into_model()
        .all(&mut tx)
        .await?;
    region_rows.sort_by(|a, b| a.region.cmp(&b.region));
    assert_eq!(region_rows.len(), 2);
    assert_eq!(region_rows[0].region, "apac");
    assert_eq!(region_rows[0].total.to_string(), "200");
    assert_eq!(region_rows[0].count, 1);
    assert_eq!(region_rows[1].region, "us");
    assert_eq!(region_rows[1].total.to_string(), "110");
    assert_eq!(region_rows[1].count, 2);

    let mut bucket_rows: Vec<BucketAgg> = Sale::query()
        .select_only()
        .column_as(dbkit::func::date_trunc("day", Sale::created_at), "bucket")
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .group_by(dbkit::func::date_trunc("day", Sale::created_at))
        .into_model()
        .all(&mut tx)
        .await?;
    bucket_rows.sort_by(|a, b| a.bucket.cmp(&b.bucket));
    assert_eq!(bucket_rows.len(), 2);
    assert_eq!(bucket_rows[0].bucket, day1_start);
    assert_eq!(bucket_rows[0].total.to_string(), "140");
    assert_eq!(bucket_rows[1].bucket, day2_start);
    assert_eq!(bucket_rows[1].total.to_string(), "200");

    let ordered_buckets: Vec<BucketAgg> = Sale::query()
        .select_only()
        .column_as(dbkit::func::date_trunc("day", Sale::created_at), "bucket")
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .group_by(dbkit::func::date_trunc("day", Sale::created_at))
        .order_by(dbkit::Order::desc(dbkit::func::date_trunc(
            "day",
            Sale::created_at,
        )))
        .into_model()
        .all(&mut tx)
        .await?;
    assert_eq!(ordered_buckets.len(), 2);
    assert_eq!(ordered_buckets[0].bucket, day2_start);
    assert_eq!(ordered_buckets[0].total.to_string(), "200");
    assert_eq!(ordered_buckets[1].bucket, day1_start);
    assert_eq!(ordered_buckets[1].total.to_string(), "140");

    let ordered_regions: Vec<RegionAgg> = Sale::query()
        .select_only()
        .column(Sale::region)
        .column_as(dbkit::func::sum(Sale::amount), "total")
        .column_as(dbkit::func::count(Sale::id), "count")
        .group_by(Sale::region)
        .order_by(dbkit::Order::desc_alias("total"))
        .into_model()
        .all(&mut tx)
        .await?;
    assert_eq!(ordered_regions.len(), 3);
    assert_eq!(ordered_regions[0].region, "apac");
    assert_eq!(ordered_regions[0].total.to_string(), "200");
    assert_eq!(ordered_regions[1].region, "us");
    assert_eq!(ordered_regions[1].total.to_string(), "110");
    assert_eq!(ordered_regions[2].region, "eu");
    assert_eq!(ordered_regions[2].total.to_string(), "30");

    let user = seed_user(&mut tx, "AggUser", "agg@db.com").await?;
    let _todo1 = seed_todo(&mut tx, user.id, "Alpha").await?;
    let _todo2 = seed_todo(&mut tx, user.id, "Beta").await?;

    let joined_rows: Vec<UserTodoAgg> = User::query()
        .select_only()
        .column_as(User::name, "name")
        .column_as(dbkit::func::count(Todo::id), "todo_count")
        .join(User::todos)
        .group_by(User::name)
        .order_by(dbkit::Order::desc(User::name.as_ref()))
        .into_model()
        .all(&mut tx)
        .await?;
    assert_eq!(joined_rows.len(), 1);
    assert_eq!(joined_rows[0].name, "AggUser");
    assert_eq!(joined_rows[0].todo_count, 2);

    Ok(())
}

#[tokio::test]
async fn query_helpers_count_exists_first_paginate() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user1 = seed_user(&mut tx, "PageOne", "page1@db.com").await?;
    let user2 = seed_user(&mut tx, "PageTwo", "page2@db.com").await?;
    let user3 = seed_user(&mut tx, "PageThree", "page3@db.com").await?;

    let total = User::query().count(&mut tx).await?;
    assert_eq!(total, 3);

    let filtered_total = User::query()
        .filter(User::email.eq("page2@db.com"))
        .count(&mut tx)
        .await?;
    assert_eq!(filtered_total, 1);

    let exists = User::query()
        .filter(User::email.eq("page2@db.com"))
        .exists(&mut tx)
        .await?;
    assert!(exists);

    let missing = User::query()
        .filter(User::email.eq("missing@db.com"))
        .exists(&mut tx)
        .await?;
    assert!(!missing);

    let first = User::query()
        .order_by(dbkit::Order::asc(User::id.as_ref()))
        .one(&mut tx)
        .await?;
    assert_eq!(first.expect("first").id, user1.id);

    let page1 = User::query()
        .order_by(dbkit::Order::asc(User::id.as_ref()))
        .paginate(1, 2, &mut tx)
        .await?;
    assert_eq!(page1.items.len(), 2);
    assert_eq!(page1.items[0].id, user1.id);
    assert_eq!(page1.items[1].id, user2.id);
    assert_eq!(page1.page, 1);
    assert_eq!(page1.per_page, 2);
    assert_eq!(page1.total, 3);
    assert_eq!(page1.total_pages(), 2);

    let page2 = User::query()
        .order_by(dbkit::Order::asc(User::id.as_ref()))
        .paginate(2, 2, &mut tx)
        .await?;
    assert_eq!(page2.items.len(), 1);
    assert_eq!(page2.items[0].id, user3.id);
    assert_eq!(page2.page, 2);
    assert_eq!(page2.per_page, 2);
    assert_eq!(page2.total, 3);
    assert_eq!(page2.total_pages(), 2);

    Ok(())
}

#[tokio::test]
async fn many_to_many_selectin_loads_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Tagger", "tagger@db.com").await?;
    let todo1 = seed_todo(&mut tx, user.id, "First").await?;
    let todo2 = seed_todo(&mut tx, user.id, "Second").await?;

    let tag_a = seed_tag(&mut tx, "A").await?;
    let tag_b = seed_tag(&mut tx, "B").await?;

    let _t1a = seed_todo_tag(&mut tx, todo1.id, tag_a.id).await?;
    let _t1b = seed_todo_tag(&mut tx, todo1.id, tag_b.id).await?;
    let _t2b = seed_todo_tag(&mut tx, todo2.id, tag_b.id).await?;

    let todos: Vec<TodoModel<dbkit::NotLoaded, Vec<Tag>>> = Todo::query()
        .filter(Todo::user_id.eq(user.id))
        .with(Todo::tags.selectin())
        .all(&mut tx)
        .await?;

    assert_eq!(todos.len(), 2);
    let mut tags_t1: Vec<String> = todos
        .iter()
        .find(|todo| todo.id == todo1.id)
        .expect("todo1")
        .tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect();
    tags_t1.sort();
    assert_eq!(tags_t1, vec!["A", "B"]);

    let mut tags_t2: Vec<String> = todos
        .iter()
        .find(|todo| todo.id == todo2.id)
        .expect("todo2")
        .tags
        .iter()
        .map(|tag| tag.name.clone())
        .collect();
    tags_t2.sort();
    assert_eq!(tags_t2, vec!["B"]);

    Ok(())
}

#[tokio::test]
async fn many_to_many_selectin_reverse_loads_parents() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Tagger", "tagger2@db.com").await?;
    let todo1 = seed_todo(&mut tx, user.id, "First").await?;
    let todo2 = seed_todo(&mut tx, user.id, "Second").await?;

    let tag_a = seed_tag(&mut tx, "A").await?;
    let tag_b = seed_tag(&mut tx, "B").await?;

    let _t1a = seed_todo_tag(&mut tx, todo1.id, tag_a.id).await?;
    let _t1b = seed_todo_tag(&mut tx, todo1.id, tag_b.id).await?;
    let _t2b = seed_todo_tag(&mut tx, todo2.id, tag_b.id).await?;

    let tags: Vec<TagModel<Vec<Todo>>> = Tag::query()
        .with(Tag::todos.selectin())
        .all(&mut tx)
        .await?;

    let tag_a_loaded = tags.iter().find(|tag| tag.id == tag_a.id).expect("tag a");
    let mut todos_a: Vec<String> = tag_a_loaded
        .todos
        .iter()
        .map(|todo| todo.title.clone())
        .collect();
    todos_a.sort();
    assert_eq!(todos_a, vec!["First"]);

    let tag_b_loaded = tags.iter().find(|tag| tag.id == tag_b.id).expect("tag b");
    let mut todos_b: Vec<String> = tag_b_loaded
        .todos
        .iter()
        .map(|todo| todo.title.clone())
        .collect();
    todos_b.sort();
    assert_eq!(todos_b, vec!["First", "Second"]);

    Ok(())
}

#[tokio::test]
async fn many_to_many_join_filter() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Joiner", "joiner@db.com").await?;
    let todo1 = seed_todo(&mut tx, user.id, "First").await?;
    let todo2 = seed_todo(&mut tx, user.id, "Second").await?;

    let tag_a = seed_tag(&mut tx, "A").await?;
    let tag_b = seed_tag(&mut tx, "B").await?;

    let _t1a = seed_todo_tag(&mut tx, todo1.id, tag_a.id).await?;
    let _t2b = seed_todo_tag(&mut tx, todo2.id, tag_b.id).await?;

    let todos = Todo::query()
        .join(Todo::tags)
        .filter(Tag::name.eq("B"))
        .distinct()
        .all(&mut tx)
        .await?;

    assert_eq!(todos.len(), 1);
    assert_eq!(todos[0].id, todo2.id);

    Ok(())
}

#[tokio::test]
async fn many_to_many_lazy_load() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Lazy", "lazy@db.com").await?;
    let todo = seed_todo(&mut tx, user.id, "First").await?;
    let tag = seed_tag(&mut tx, "A").await?;
    let _link = seed_todo_tag(&mut tx, todo.id, tag.id).await?;

    let loaded = todo.load(Todo::tags, &mut tx).await?;
    assert_eq!(loaded.tags.len(), 1);
    assert_eq!(loaded.tags[0].name, "A");

    Ok(())
}
#[tokio::test]
async fn active_insert_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let mut active = User::new_active();
    active.name = "Active".into();
    active.email = "active@db.com".into();

    let inserted = active.insert(&mut tx).await?;
    let _: User = inserted.clone();
    assert!(inserted.id > 0);
    assert_eq!(inserted.name, "Active");
    assert_eq!(inserted.email, "active@db.com");

    Ok(())
}

#[tokio::test]
async fn active_insert_missing_required_field_errors() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let mut active = UserActive::new();
    active.name = "Missing email".into();

    let result = active.insert(&mut tx).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn active_update_from_loaded() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Before", "before@db.com").await?;
    let user_id = user.id;
    let user_email = user.email.clone();
    let mut active = user.into_active();
    active.name = "After".into();

    let updated = active.update(&mut tx).await?;
    let _: User = updated.clone();
    assert_eq!(updated.id, user_id);
    assert_eq!(updated.name, "After");
    assert_eq!(updated.email, user_email);

    Ok(())
}

#[tokio::test]
async fn active_update_does_not_overwrite_other_fields() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    // This test simulates a concurrent update to a different column and ensures
    // ActiveModel updates only changed fields (no stale-field overwrite).
    let user = seed_user(&mut tx, "Before", "before@db.com").await?;
    let mut active = user.clone().into_active();

    User::update()
        .set(User::email, "updated@db.com")
        .filter(User::id.eq(user.id))
        .execute(&mut tx)
        .await?;

    active.name = "After".into();
    let _ = active.update(&mut tx).await?;

    let fetched = User::by_id(user.id).one(&mut tx).await?.expect("user");
    assert_eq!(fetched.name, "After");
    assert_eq!(fetched.email, "updated@db.com");

    Ok(())
}

#[tokio::test]
async fn active_update_set_null() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let row = seed_nullable_row(&mut tx, Some("note".to_string())).await?;
    let row_id = row.id;
    let mut active = row.into_active();
    active.note = None.into();

    let updated = active.update(&mut tx).await?;
    let _: NullableRow = updated.clone();
    assert_eq!(updated.id, row_id);
    assert!(updated.note.is_none());

    Ok(())
}

#[tokio::test]
async fn active_update_requires_primary_key() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let mut active = UserActive::new();
    active.name = "No PK".into();
    active.email = "no-pk@db.com".into();

    let result = active.update(&mut tx).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn active_update_uses_only_primary_key_filter() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Before", "before@db.com").await?;
    let untouched = seed_user(&mut tx, "Other", "other@db.com").await?;
    let user_id = user.id;
    let mut active = user.into_active();
    active.name = "After".into();
    active.email = "after@db.com".into();

    let updated = active.update(&mut tx).await?;
    assert_eq!(updated.id, user_id);
    assert_eq!(updated.name, "After");
    assert_eq!(updated.email, "after@db.com");

    let fetched = User::by_id(user_id).one(&mut tx).await?.expect("user");
    assert_eq!(fetched.name, "After");
    assert_eq!(fetched.email, "after@db.com");

    let other = User::by_id(untouched.id).one(&mut tx).await?.expect("other");
    assert_eq!(other.name, "Other");
    assert_eq!(other.email, "other@db.com");

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_update_uses_both_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let first = seed_order_line(&mut tx, 1, 1, "A").await?;
    let _same_order = seed_order_line(&mut tx, 1, 2, "B").await?;
    let _same_line = seed_order_line(&mut tx, 2, 1, "C").await?;

    let mut active = first.into_active();
    active.note = "A1".into();
    let updated = active.update(&mut tx).await?;
    assert_eq!(updated.order_id, 1);
    assert_eq!(updated.line_id, 1);
    assert_eq!(updated.note, "A1");

    let fetched = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(1))
        .one(&mut tx)
        .await?
        .expect("updated");
    assert_eq!(fetched.note, "A1");

    let same_order = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(2))
        .one(&mut tx)
        .await?
        .expect("same order");
    assert_eq!(same_order.note, "B");

    let same_line = OrderLine::query()
        .filter(OrderLine::order_id.eq(2))
        .filter(OrderLine::line_id.eq(1))
        .one(&mut tx)
        .await?
        .expect("same line");
    assert_eq!(same_line.note, "C");

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_update_requires_all_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let mut missing_line = OrderLine::new_active();
    missing_line.order_id = 1.into();
    missing_line.note = "Missing line".into();
    assert!(missing_line.update(&mut tx).await.is_err());

    let mut missing_order = OrderLine::new_active();
    missing_order.line_id = 1.into();
    missing_order.note = "Missing order".into();
    assert!(missing_order.update(&mut tx).await.is_err());

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_insert_requires_all_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let mut missing_line = OrderLine::new_active();
    missing_line.order_id = 1.into();
    missing_line.note = "Missing line".into();
    assert!(missing_line.insert(&mut tx).await.is_err());

    let mut missing_order = OrderLine::new_active();
    missing_order.line_id = 1.into();
    missing_order.note = "Missing order".into();
    assert!(missing_order.insert(&mut tx).await.is_err());

    Ok(())
}

#[tokio::test]
async fn active_delete_removes_only_target() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Delete", "delete@db.com").await?;
    let user_id = user.id;
    let other = seed_user(&mut tx, "Keep", "keep@db.com").await?;

    let active = user.into_active();
    let deleted = active.delete(&mut tx).await?;
    assert_eq!(deleted, 1);

    let removed = User::by_id(user_id).one(&mut tx).await?;
    assert!(removed.is_none());

    let remaining = User::by_id(other.id).one(&mut tx).await?.expect("other");
    assert_eq!(remaining.email, "keep@db.com");

    Ok(())
}

#[tokio::test]
async fn active_delete_requires_primary_key() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let mut active = User::new_active();
    active.name = "No PK".into();
    active.email = "no-pk@db.com".into();

    let result = active.delete(&mut tx).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn model_delete_removes_row() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let user = seed_user(&mut tx, "Delete", "delete2@db.com").await?;
    let user_id = user.id;
    let deleted = user.delete(&mut tx).await?;
    assert_eq!(deleted, 1);

    let removed = User::by_id(user_id).one(&mut tx).await?;
    assert!(removed.is_none());

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_delete_uses_both_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let target = seed_order_line(&mut tx, 1, 1, "A").await?;
    let _same_order = seed_order_line(&mut tx, 1, 2, "B").await?;
    let _same_line = seed_order_line(&mut tx, 2, 1, "C").await?;

    let deleted = target.into_active().delete(&mut tx).await?;
    assert_eq!(deleted, 1);

    let removed = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(1))
        .one(&mut tx)
        .await?;
    assert!(removed.is_none());

    let same_order = OrderLine::query()
        .filter(OrderLine::order_id.eq(1))
        .filter(OrderLine::line_id.eq(2))
        .one(&mut tx)
        .await?
        .expect("same order");
    assert_eq!(same_order.note, "B");

    let same_line = OrderLine::query()
        .filter(OrderLine::order_id.eq(2))
        .filter(OrderLine::line_id.eq(1))
        .one(&mut tx)
        .await?
        .expect("same line");
    assert_eq!(same_line.note, "C");

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_active_delete_requires_all_keys() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let mut missing_line = OrderLine::new_active();
    missing_line.order_id = 1.into();
    missing_line.note = "Missing line".into();
    assert!(missing_line.delete(&mut tx).await.is_err());

    let mut missing_order = OrderLine::new_active();
    missing_order.line_id = 1.into();
    missing_order.note = "Missing order".into();
    assert!(missing_order.delete(&mut tx).await.is_err());

    Ok(())
}

#[tokio::test]
async fn composite_primary_key_model_delete_removes_row() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.begin().await?;
    setup_schema(&mut tx).await?;

    let target = seed_order_line(&mut tx, 9, 9, "Z").await?;
    let order_id = target.order_id;
    let line_id = target.line_id;
    let deleted = target.delete(&mut tx).await?;
    assert_eq!(deleted, 1);

    let removed = OrderLine::query()
        .filter(OrderLine::order_id.eq(order_id))
        .filter(OrderLine::line_id.eq(line_id))
        .one(&mut tx)
        .await?;
    assert!(removed.is_none());

    Ok(())
}
