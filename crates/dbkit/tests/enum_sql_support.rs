#![allow(non_upper_case_globals)]

use dbkit::model;
use dbkit::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "queue_state", rename_all = "snake_case")]
pub enum QueueState {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "delivery_channel", rename_all = "snake_case")]
pub enum DeliveryChannel {
    Email,
    Sms,
    Webhook,
}

#[model(table = "message_jobs")]
pub struct MessageJob {
    #[key]
    pub id: i64,
    pub dedupe_key: String,
    pub state: QueueState,
    pub channel: Option<DeliveryChannel>,
    pub attempts: i64,
}

#[test]
fn enum_filter_sql_uses_typed_casts_and_stable_bind_reuse() {
    let compiled = MessageJob::query()
        .filter(MessageJob::state.eq(QueueState::Queued))
        .filter(MessageJob::state.in_([QueueState::Queued, QueueState::Running]))
        .filter(MessageJob::channel.eq(Some(DeliveryChannel::Email)))
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT message_jobs.* FROM message_jobs WHERE (message_jobs.state = $1::queue_state) AND (message_jobs.state IN ($1::queue_state, $2::queue_state)) AND (message_jobs.channel = $3::delivery_channel)"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::Enum {
                type_name: "queue_state",
                value: "queued".to_string(),
            },
            Value::Enum {
                type_name: "queue_state",
                value: "running".to_string(),
            },
            Value::Enum {
                type_name: "delivery_channel",
                value: "email".to_string(),
            },
        ]
    );
}

#[test]
fn enum_null_paths_compile_without_typed_bind_placeholders() {
    let select_sql = MessageJob::query().filter(MessageJob::channel.eq(None::<DeliveryChannel>)).compile();
    assert_eq!(
        select_sql.sql,
        "SELECT message_jobs.* FROM message_jobs WHERE (message_jobs.channel IS NULL)"
    );
    assert!(select_sql.binds.is_empty());

    let update_sql = MessageJob::update()
        .set(MessageJob::channel, None::<DeliveryChannel>)
        .filter(MessageJob::id.eq(11_i64))
        .compile();
    assert_eq!(
        update_sql.sql,
        "UPDATE message_jobs SET channel = NULL WHERE (message_jobs.id = $1)"
    );
    assert_eq!(update_sql.binds, vec![Value::I64(11)]);
}

#[test]
fn enum_update_and_insert_conflict_sql_keep_explicit_type_casts() {
    let update = MessageJob::update()
        .set(MessageJob::state, QueueState::Completed)
        .set(MessageJob::channel, Some(DeliveryChannel::Webhook))
        .set(MessageJob::attempts, 2_i64)
        .filter(MessageJob::id.eq(42_i64))
        .compile();
    assert_eq!(
        update.sql,
        "UPDATE message_jobs SET state = $1::queue_state, channel = $2::delivery_channel, attempts = $3 WHERE (message_jobs.id = $4)"
    );
    assert_eq!(
        update.binds,
        vec![
            Value::Enum {
                type_name: "queue_state",
                value: "completed".to_string(),
            },
            Value::Enum {
                type_name: "delivery_channel",
                value: "webhook".to_string(),
            },
            Value::I64(2),
            Value::I64(42),
        ]
    );

    let insert = MessageJob::insert(MessageJobInsert {
        id: 42,
        dedupe_key: "job-42".to_string(),
        state: QueueState::Queued,
        channel: Some(DeliveryChannel::Sms),
        attempts: 0,
    })
    .on_conflict_do_update(
        MessageJob::dedupe_key,
        (MessageJob::state, MessageJob::channel, MessageJob::attempts),
    )
    .returning_all()
    .compile();
    assert_eq!(
        insert.sql,
        "INSERT INTO message_jobs (id, dedupe_key, state, channel, attempts) VALUES ($1, $2, $3::queue_state, $4::delivery_channel, $5) ON CONFLICT (dedupe_key) DO UPDATE SET state = EXCLUDED.state, channel = EXCLUDED.channel, attempts = EXCLUDED.attempts RETURNING message_jobs.*"
    );
    assert_eq!(
        insert.binds,
        vec![
            Value::I64(42),
            Value::String("job-42".to_string()),
            Value::Enum {
                type_name: "queue_state",
                value: "queued".to_string(),
            },
            Value::Enum {
                type_name: "delivery_channel",
                value: "sms".to_string(),
            },
            Value::I64(0),
        ]
    );
}

#[test]
fn enum_empty_in_compiles_to_false_without_binds() {
    let compiled = MessageJob::query()
        .filter(MessageJob::state.in_(std::iter::empty::<QueueState>()))
        .compile();

    assert_eq!(compiled.sql, "SELECT message_jobs.* FROM message_jobs WHERE (FALSE)");
    assert!(compiled.binds.is_empty());
}
