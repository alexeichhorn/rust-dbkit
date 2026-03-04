#![allow(non_upper_case_globals)]

use chrono::{DateTime, Duration, FixedOffset, TimeZone, Utc};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor};
use uuid::Uuid;

#[model(table = "events_tz")]
pub struct EventTz {
    #[key]
    pub id: Uuid,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE events_tz (\
            id UUID PRIMARY KEY,\
            title TEXT NOT NULL,\
            starts_at TIMESTAMPTZ NOT NULL,\
            ends_at TIMESTAMPTZ NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    Ok(())
}

async fn seed_event_tz<E: Executor + Send + Sync>(
    ex: &E,
    id: Uuid,
    title: &str,
    starts_at: DateTime<Utc>,
    ends_at: Option<DateTime<Utc>>,
) -> Result<EventTz, dbkit::Error> {
    let row = EventTz::insert(EventTzInsert {
        id,
        title: title.to_string(),
        starts_at,
        ends_at,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted events_tz row");
    Ok(row)
}

#[tokio::test]
async fn timestamptz_roundtrip_filters_and_between_window() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let offset = FixedOffset::east_opt(5 * 3600).expect("offset");
    let starts_at_local = offset.with_ymd_and_hms(2024, 3, 14, 10, 15, 0).single().expect("offset datetime");
    let starts_at = starts_at_local.with_timezone(&Utc);
    let ends_at = Some(starts_at + Duration::hours(8));
    let id = Uuid::from_u128(1);

    let inserted = seed_event_tz(&tx, id, "TZ event", starts_at, ends_at).await?;
    assert_eq!(inserted.id, id);
    assert_eq!(inserted.starts_at, starts_at);
    assert_eq!(inserted.ends_at, ends_at);

    let found = EventTz::query()
        .filter(EventTz::id.eq(id))
        .filter(EventTz::starts_at.eq(starts_at))
        .one(&tx)
        .await?
        .expect("event");
    assert_eq!(found.id, id);
    assert_eq!(found.title, "TZ event");
    assert_eq!(found.starts_at, starts_at);
    assert_eq!(found.ends_at, ends_at);

    let low = starts_at - Duration::minutes(1);
    let high = starts_at + Duration::minutes(1);
    let between = EventTz::query().filter(EventTz::starts_at.between(low, high)).all(&tx).await?;
    assert_eq!(between.len(), 1);
    assert_eq!(between[0].id, id);

    Ok(())
}

#[tokio::test]
async fn timestamptz_null_filter_and_active_update_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let starts_at = Utc.with_ymd_and_hms(2024, 1, 2, 3, 4, 5).single().expect("utc datetime");
    let id = Uuid::from_u128(2);

    let inserted = seed_event_tz(&tx, id, "Nullable end", starts_at, None).await?;
    assert!(inserted.ends_at.is_none());

    let pending = EventTz::query().filter(EventTz::ends_at.eq(None::<DateTime<Utc>>)).all(&tx).await?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, id);

    let finished_at = starts_at + Duration::hours(6);
    let rescheduled_start = starts_at + Duration::hours(2);
    let mut active = inserted.into_active();
    active.starts_at = rescheduled_start.into();
    active.ends_at = Some(finished_at).into();
    let updated = active.update(&tx).await?;
    assert_eq!(updated.starts_at, rescheduled_start);
    assert_eq!(updated.ends_at, Some(finished_at));

    let mut active = updated.into_active();
    active.ends_at = None::<DateTime<Utc>>.into();
    let cleared = active.update(&tx).await?;
    assert!(cleared.ends_at.is_none());

    let fetched = EventTz::query().filter(EventTz::id.eq(id)).one(&tx).await?.expect("fetched event");
    assert_eq!(fetched.starts_at, rescheduled_start);
    assert!(fetched.ends_at.is_none());

    Ok(())
}
