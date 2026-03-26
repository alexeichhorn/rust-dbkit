//@check-pass
use dbkit::model;

#[model(table = "teams")]
pub struct Team {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub owner_id: i64,
    pub state: String,
}

#[model(table = "team_members")]
pub struct TeamMember {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub team_id: i64,
    pub role: String,
}

fn main() {
    let _delete_sql = TeamMember::delete()
        .where_exists(
            Team::query()
                .select_only()
                .column(Team::id)
                .filter(Team::id.eq_col(TeamMember::team_id))
                .filter(Team::state.eq("active")),
        )
        .compile();

    let _update_sql = Team::update()
        .set(Team::state, "inactive")
        .where_not_exists(
            TeamMember::query()
                .select_only()
                .column(TeamMember::id)
                .filter(TeamMember::team_id.eq_col(Team::id))
                .filter(TeamMember::role.eq("lead")),
        )
        .compile();
}
