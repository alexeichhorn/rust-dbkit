use dbkit::model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "moderation_state", rename_all = "snake_case")]
pub enum ModerationState {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "visibility_state", rename_all = "snake_case")]
pub enum VisibilityState {
    Public,
    Private,
}

#[model(table = "posts")]
pub struct Post {
    #[key]
    pub id: i64,
    pub moderation: Option<ModerationState>,
}

fn main() {
    let _eq = Post::query().filter(Post::moderation.eq(Some(VisibilityState::Private))); //~ E0277
    let _update = Post::update().set(Post::moderation, Some(VisibilityState::Public)); //~ E0277
}
