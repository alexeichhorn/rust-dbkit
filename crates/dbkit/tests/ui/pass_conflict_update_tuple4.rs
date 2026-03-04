//@check-pass
use dbkit::model;

#[model(table = "jobs")]
pub struct Job {
    #[key]
    pub source_id: String,
    pub title: String,
    pub company: String,
    pub location: String,
    pub content_hash: String,
    pub removed_at: Option<chrono::NaiveDateTime>,
    pub updated_at: chrono::NaiveDateTime,
}

fn main() {
    let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let _query = Job::insert(JobInsert {
        source_id: "job-1".to_string(),
        title: "Senior Rust Engineer".to_string(),
        company: "Acme".to_string(),
        location: "Zurich".to_string(),
        content_hash: "hash-1".to_string(),
        removed_at: None,
        updated_at: now,
    })
    .on_conflict_do_update(
        Job::source_id,
        (Job::title, Job::company, Job::location, Job::content_hash),
    );
}
