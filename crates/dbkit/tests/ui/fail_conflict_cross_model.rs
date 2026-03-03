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

#[model(table = "other_rows")]
pub struct OtherRow {
    #[key]
    pub id: i64,
    pub value: String,
}

fn main() {
    let _query = RunPayload::insert(RunPayloadInsert {
        target_id: 1,
        run_id: 2,
        payload: "p".to_string(),
        source: "s".to_string(),
        version: 1,
    })
    .on_conflict_do_update(
        (RunPayload::target_id, RunPayload::run_id),
        (RunPayload::payload, OtherRow::value),
    );
}
