//@check-pass
use chrono::NaiveDateTime;
use dbkit::{model, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RetryCount(i64);

impl From<RetryCount> for Value {
    fn from(value: RetryCount) -> Self {
        Value::I64(value.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScheduledAt(NaiveDateTime);

impl From<ScheduledAt> for Value {
    fn from(value: ScheduledAt) -> Self {
        Value::DateTime(value.0)
    }
}

#[model(table = "articles")]
pub struct Article {
    #[key]
    pub id: i64,
    pub slug: String,
    pub attempts: i64,
    pub published_at: NaiveDateTime,
}

fn main() {
    let published_cutoff = ScheduledAt(
        chrono::DateTime::from_timestamp(1_700_000_000, 0)
            .expect("published_cutoff")
            .naive_utc(),
    );

    let _query = Article::query()
        .filter(Article::attempts.ge(RetryCount(3)))
        .filter(Article::published_at.lt(published_cutoff));
}
