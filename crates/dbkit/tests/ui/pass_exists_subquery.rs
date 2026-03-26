//@check-pass
use dbkit::{model, Order};

#[model(table = "organizations")]
pub struct Organization {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub slug: String,
}

#[model(table = "projects")]
pub struct Project {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub organization_id: i64,
    pub state: String,
}

fn main() {
    let _exists_sql = Organization::query()
        .where_exists(
            Project::query()
                .select_only()
                .column(Project::id)
                .filter(Project::organization_id.eq_col(Organization::id))
                .filter(Project::state.eq("active")),
        )
        .order_by(Order::asc(Organization::id))
        .debug_sql();

    let _missing_sql = Organization::query()
        .where_not_exists(
            Project::query()
                .select_only()
                .column(Project::id)
                .filter(Project::organization_id.eq_col(Organization::id))
                .filter(Project::state.eq("archived")),
        )
        .debug_sql();
}
