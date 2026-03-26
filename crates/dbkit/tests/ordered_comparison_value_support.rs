#![allow(non_upper_case_globals)]

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

#[test]
fn ordered_comparison_preserves_value_convertible_rhs_support() {
    let published_cutoff = ScheduledAt(
        chrono::DateTime::from_timestamp(1_700_000_000, 0)
            .expect("published_cutoff")
            .naive_utc(),
    );

    let compiled = Article::query()
        .filter(Article::attempts.ge(RetryCount(3)))
        .filter(Article::published_at.lt(published_cutoff))
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT articles.* FROM articles WHERE (articles.attempts >= $1) AND (articles.published_at < $2)"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::I64(3),
            Value::DateTime(published_cutoff.0),
        ]
    );
}
