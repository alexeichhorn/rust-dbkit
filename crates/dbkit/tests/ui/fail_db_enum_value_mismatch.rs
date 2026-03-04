use dbkit::model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "task_state", rename_all = "snake_case")]
pub enum TaskState {
    Queued,
    Running,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "delivery_state", rename_all = "snake_case")]
pub enum DeliveryState {
    Queued,
    Running,
}

#[model(table = "tasks")]
pub struct Task {
    #[key]
    pub id: i64,
    pub state: TaskState,
}

fn main() {
    let _eq = Task::query().filter(Task::state.eq(DeliveryState::Queued)); //~ E0277
}
