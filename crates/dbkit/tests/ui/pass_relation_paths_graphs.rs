//@check-pass
#[path = "../support/relation_path_graphs.rs"]
mod relation_path_graphs;
#[path = "../support/relation_paths.rs"]
mod relation_paths;

use dbkit::{NotLoaded, SelectExt};
use relation_path_graphs::{Assignment, Node};
use relation_paths::{Member, Organization};

async fn loading(ex: &(impl dbkit::Executor + Send + Sync)) -> Result<(), dbkit::Error> {
    let _: Vec<Assignment<Option<Member>>> = Assignment::query().with(Assignment::first.joined()).all(ex).await?;
    let _: Vec<Assignment<NotLoaded, Option<Member>>> = Assignment::query().with(Assignment::second.joined()).all(ex).await?;
    let _: Vec<Assignment<Option<Member>, Option<Member>>> = Assignment::query()
        .with(Assignment::second.joined())
        .with(Assignment::first.joined())
        .all(ex)
        .await?;
    let _: Vec<Assignment<Option<Member<Option<Organization>>>, Option<Member>>> = Assignment::query()
        .with(Assignment::first.joined().with(Member::organization.joined()))
        .with(Assignment::second.selectin())
        .all(ex)
        .await?;

    let _: Vec<Node<Option<Node<Option<Node>>>>> = Node::query()
        .filter(Node::parent.parent.label.eq("root"))
        .with(Node::parent.joined().with(Node::parent.joined()))
        .all(ex)
        .await?;
    Ok(())
}

fn main() {
    let _ = Assignment::query()
        .filter(Assignment::first.enabled.eq(true))
        .filter(Assignment::second.enabled.eq(false))
        .filter(Assignment::first.score.gt(Assignment::second.score));
    let _ = Assignment::query()
        .filter(Assignment::first.organization.label.eq("north"))
        .filter(Assignment::second.organization.label.eq("south"));
    let _ = Node::query().filter(Node::parent.parent.parent.id.eq(1_i64));
}
