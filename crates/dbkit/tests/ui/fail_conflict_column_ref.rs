use dbkit::model;

#[model(table = "run_payloads")]
pub struct RunPayload {
    #[key]
    pub target_id: i64,
    #[key]
    pub run_id: i64,
    pub payload: String,
    pub source: String,
    pub version: i64,
}

fn main() {
    let _query = RunPayload::insert(RunPayloadInsert {
        target_id: 1,
        run_id: 2,
        payload: "p".to_string(),
        source: "s".to_string(),
        version: 1,
    })
    .on_conflict_do_nothing((RunPayload::target_id.as_ref(), RunPayload::run_id.as_ref()));
}
