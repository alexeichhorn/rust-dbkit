//@check-pass
use dbkit::model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "workflow_stage", rename_all = "snake_case")]
pub enum WorkflowStage {
    Queued,
    Running,
    Finished,
}

#[model(table = "tasks")]
pub struct Task {
    #[key]
    pub id: i64,
    pub stage: WorkflowStage,
}

#[model(table = "task_snapshots")]
pub struct TaskSnapshot {
    #[key]
    pub id: i64,
    pub task_id: i64,
    pub stage: WorkflowStage,
}

fn main() {
    let _cross_model_same_enum = Task::query().filter(Task::stage.eq_col(TaskSnapshot::stage));

    let _in_filter = Task::query().filter(Task::stage.in_([WorkflowStage::Queued, WorkflowStage::Running]));

    let _insert_snapshot = TaskSnapshot::insert(TaskSnapshotInsert {
        id: 10,
        task_id: 1,
        stage: WorkflowStage::Queued,
    });

    let _update_snapshot = TaskSnapshot::update()
        .set(TaskSnapshot::stage, WorkflowStage::Finished)
        .filter(TaskSnapshot::task_id.eq(1_i64));
}
