#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "run_state", rename_all = "snake_case")]
pub enum RunState {
    Scheduled,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "run_outcome", rename_all = "snake_case")]
pub enum RunOutcome {
    Ok,
    Error,
    Timeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "integration_mode", rename_all = "snake_case")]
pub enum IntegrationMode {
    HTTPWebhook,
    OAuthToken,
    XMLHttpRequest,
    WebhookHTTP,
}

#[model(table = "workflow_runs")]
pub struct WorkflowRun {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub external_ref: String,
    pub payload: String,
    pub state: RunState,
    pub outcome: Option<RunOutcome>,
    pub attempts: i64,
}

#[model(table = "integration_events")]
pub struct IntegrationEvent {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub ext_id: String,
    pub mode: IntegrationMode,
    pub fallback_mode: Option<IntegrationMode>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TYPE pg_temp.run_state AS ENUM ('scheduled', 'running', 'succeeded', 'failed')",
        PgArguments::default(),
    )
    .await?;
    ex.execute(
        "CREATE TYPE pg_temp.run_outcome AS ENUM ('ok', 'error', 'timeout')",
        PgArguments::default(),
    )
    .await?;
    ex.execute(
        "CREATE TEMP TABLE workflow_runs (\
            id BIGSERIAL PRIMARY KEY,\
            external_ref TEXT NOT NULL UNIQUE,\
            payload TEXT NOT NULL,\
            state run_state NOT NULL,\
            outcome run_outcome NULL,\
            attempts BIGINT NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    ex.execute(
        "CREATE TYPE pg_temp.integration_mode AS ENUM ('http_webhook', 'oauth_token', 'xml_http_request', 'webhook_http')",
        PgArguments::default(),
    )
    .await?;
    ex.execute(
        "CREATE TEMP TABLE integration_events (\
            id BIGSERIAL PRIMARY KEY,\
            ext_id TEXT NOT NULL UNIQUE,\
            mode integration_mode NOT NULL,\
            fallback_mode integration_mode NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    Ok(())
}

async fn seed_run<E: Executor + Send + Sync>(
    ex: &E,
    external_ref: &str,
    payload: &str,
    state: RunState,
    outcome: Option<RunOutcome>,
    attempts: i64,
) -> Result<WorkflowRun, dbkit::Error> {
    let row = WorkflowRun::insert(WorkflowRunInsert {
        external_ref: external_ref.to_string(),
        payload: payload.to_string(),
        state,
        outcome,
        attempts,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted workflow run");
    Ok(row)
}

#[tokio::test]
async fn enum_roundtrip_filter_update_and_optional_nulling() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let inserted = seed_run(&tx, "run-1", "payload-v1", RunState::Scheduled, None, 0).await?;
    assert_eq!(inserted.state, RunState::Scheduled);
    assert_eq!(inserted.outcome, None);

    let by_state = WorkflowRun::query()
        .filter(WorkflowRun::state.eq(RunState::Scheduled))
        .one(&tx)
        .await?
        .expect("row by state");
    assert_eq!(by_state.id, inserted.id);
    assert_eq!(by_state.external_ref, "run-1");

    let mut updated_rows = WorkflowRun::update()
        .set(WorkflowRun::state, RunState::Running)
        .set(WorkflowRun::outcome, Some(RunOutcome::Ok))
        .set(WorkflowRun::attempts, 1_i64)
        .filter(WorkflowRun::id.eq(inserted.id))
        .returning_all()
        .all(&tx)
        .await?;
    assert_eq!(updated_rows.len(), 1);
    let updated = updated_rows.pop().expect("updated row");
    assert_eq!(updated.state, RunState::Running);
    assert_eq!(updated.outcome, Some(RunOutcome::Ok));
    assert_eq!(updated.attempts, 1);

    let mut active = updated.into_active();
    active.state = RunState::Succeeded.into();
    active.outcome = None::<RunOutcome>.into();
    let finished = active.update(&tx).await?;
    assert_eq!(finished.state, RunState::Succeeded);
    assert_eq!(finished.outcome, None);

    let done = WorkflowRun::query()
        .filter(WorkflowRun::state.in_([RunState::Running, RunState::Succeeded]))
        .all(&tx)
        .await?;
    assert_eq!(done.len(), 1);
    assert_eq!(done[0].external_ref, "run-1");
    assert_eq!(done[0].state, RunState::Succeeded);

    let without_outcome = WorkflowRun::query()
        .filter(WorkflowRun::outcome.eq(None::<RunOutcome>))
        .all(&tx)
        .await?;
    assert_eq!(without_outcome.len(), 1);
    assert_eq!(without_outcome[0].id, inserted.id);

    Ok(())
}

#[tokio::test]
async fn enum_upsert_updates_selected_enum_columns_and_preserves_unselected_fields() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let original = seed_run(&tx, "same-ref", "payload-original", RunState::Scheduled, None, 0).await?;

    let upserted = WorkflowRun::insert(WorkflowRunInsert {
        external_ref: "same-ref".to_string(),
        payload: "payload-new-but-should-not-overwrite".to_string(),
        state: RunState::Failed,
        outcome: Some(RunOutcome::Timeout),
        attempts: 5,
    })
    .on_conflict_do_update(
        WorkflowRun::external_ref,
        (WorkflowRun::state, WorkflowRun::outcome, WorkflowRun::attempts),
    )
    .returning_all()
    .one(&tx)
    .await?
    .expect("upserted row");

    assert_eq!(upserted.id, original.id);
    assert_eq!(upserted.external_ref, "same-ref");
    assert_eq!(upserted.payload, "payload-original");
    assert_eq!(upserted.state, RunState::Failed);
    assert_eq!(upserted.outcome, Some(RunOutcome::Timeout));
    assert_eq!(upserted.attempts, 5);

    let fetched = WorkflowRun::query()
        .filter(WorkflowRun::external_ref.eq("same-ref"))
        .one(&tx)
        .await?
        .expect("fetched row");

    assert_eq!(fetched.id, original.id);
    assert_eq!(fetched.payload, "payload-original");
    assert_eq!(fetched.state, RunState::Failed);
    assert_eq!(fetched.outcome, Some(RunOutcome::Timeout));
    assert_eq!(fetched.attempts, 5);

    Ok(())
}

#[tokio::test]
async fn enum_acronym_wire_names_roundtrip_for_crud_filters_and_upsert() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let inserted = IntegrationEvent::insert(IntegrationEventInsert {
        ext_id: "evt-1".to_string(),
        mode: IntegrationMode::HTTPWebhook,
        fallback_mode: Some(IntegrationMode::OAuthToken),
    })
    .returning_all()
    .one(&tx)
    .await?
    .expect("inserted integration event");
    assert_eq!(inserted.mode, IntegrationMode::HTTPWebhook);
    assert_eq!(inserted.fallback_mode, Some(IntegrationMode::OAuthToken));

    let filtered = IntegrationEvent::query()
        .filter(IntegrationEvent::mode.eq(IntegrationMode::HTTPWebhook))
        .filter(IntegrationEvent::fallback_mode.in_([Some(IntegrationMode::OAuthToken)]))
        .one(&tx)
        .await?
        .expect("filtered integration event");
    assert_eq!(filtered.id, inserted.id);

    let mut updated_rows = IntegrationEvent::update()
        .set(IntegrationEvent::mode, IntegrationMode::WebhookHTTP)
        .set(IntegrationEvent::fallback_mode, None::<IntegrationMode>)
        .filter(IntegrationEvent::id.eq(inserted.id))
        .returning_all()
        .all(&tx)
        .await?;
    assert_eq!(updated_rows.len(), 1);
    let updated = updated_rows.pop().expect("updated integration event");
    assert_eq!(updated.mode, IntegrationMode::WebhookHTTP);
    assert_eq!(updated.fallback_mode, None);

    let upserted = IntegrationEvent::insert(IntegrationEventInsert {
        ext_id: "evt-1".to_string(),
        mode: IntegrationMode::XMLHttpRequest,
        fallback_mode: Some(IntegrationMode::HTTPWebhook),
    })
    .on_conflict_do_update(
        IntegrationEvent::ext_id,
        (IntegrationEvent::mode, IntegrationEvent::fallback_mode),
    )
    .returning_all()
    .one(&tx)
    .await?
    .expect("upserted integration event");

    assert_eq!(upserted.id, inserted.id);
    assert_eq!(upserted.mode, IntegrationMode::XMLHttpRequest);
    assert_eq!(upserted.fallback_mode, Some(IntegrationMode::HTTPWebhook));

    Ok(())
}
