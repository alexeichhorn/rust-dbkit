//@check-pass
use chrono::{DateTime, TimeZone, Utc};
use dbkit::model;

#[model(table = "events_tz")]
pub struct EventTz {
    #[key]
    pub id: uuid::Uuid,
    pub title: String,
    pub starts_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
}

fn main() {
    let starts_at = Utc
        .with_ymd_and_hms(2024, 1, 2, 3, 4, 5)
        .single()
        .expect("utc datetime");
    let ends_at = starts_at + chrono::Duration::hours(1);

    let _query = EventTz::query()
        .filter(EventTz::starts_at.eq(starts_at))
        .filter(EventTz::starts_at.between(starts_at, ends_at))
        .filter(EventTz::ends_at.eq(None::<DateTime<Utc>>));

    let _insert = EventTz::insert(EventTzInsert {
        id: uuid::Uuid::nil(),
        title: "TZ".to_string(),
        starts_at,
        ends_at: Some(ends_at),
    });

    let _update = EventTz::update()
        .set(EventTz::starts_at, ends_at)
        .set(EventTz::ends_at, Some(ends_at));

    let mut active = EventTz::new_active();
    active.id = uuid::Uuid::nil().into();
    active.title = "Active TZ".to_string().into();
    active.starts_at = starts_at.into();
    active.ends_at = None::<DateTime<Utc>>.into();

    let _ = active;
}
