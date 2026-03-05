use chrono::NaiveDateTime;
use dbkit::model;

#[model(table = "records")]
pub struct Record {
    #[key]
    pub id: i64,
    pub occurred_at: NaiveDateTime,
}

fn main() {
    let cutoff = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("cutoff")
        .naive_utc();
    let _query = Record::query().filter((Record::occurred_at + 1_i64).le(cutoff)); //~ E0271
}
