#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "job_state", rename_all = "snake_case")]
pub enum JobState {
    Planned,
    #[dbkit(rename = "planned")]
    Active, //~ ERROR dbkit: duplicate DbEnum wire name
}

fn main() {}
