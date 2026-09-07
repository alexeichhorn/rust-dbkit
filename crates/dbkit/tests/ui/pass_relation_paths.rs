//@check-pass
#[path = "../support/relation_paths.rs"]
mod relation_paths;

use dbkit::{func, Expr, IntoExpr, NotLoaded, Order, SelectExt};
use relation_paths::{Member, Organization, Record};

fn nullable<T>(_: Expr<Option<T>>) {}
fn required<T>(_: Expr<T>) {}

async fn loading(ex: &(impl dbkit::Executor + Send + Sync)) -> Result<(), dbkit::Error> {
    let bare: Vec<Record> = Record::query().filter(Record::owner.enabled.eq(true)).all(ex).await?;
    let _: &NotLoaded = &bare[0].owner;
    let _: Vec<Record<Option<Member>>> = Record::query()
        .with(Record::owner.joined())
        .filter(Record::owner.enabled.eq(true))
        .all(ex)
        .await?;
    let _: Vec<Record<Option<Member>>> = Record::query()
        .filter(Record::owner.enabled.eq(true))
        .with(Record::owner.selectin())
        .all(ex)
        .await?;
    let _: Vec<Record<Option<Member<Option<Organization>>>>> = Record::query()
        .filter(Record::owner.organization.label.eq("north"))
        .with(Record::owner.joined().with(Member::organization.joined()))
        .all(ex)
        .await?;
    Ok(())
}

fn main() {
    // Relation columns include outer-join nullability without changing base columns.
    required::<bool>(Member::enabled.into_expr());
    nullable::<bool>(Record::owner.enabled.into_expr());
    nullable::<i64>(Record::owner.id.into_expr());
    nullable::<String>(Record::owner.label.into_expr());
    nullable::<String>(Record::owner.note.into_expr());
    nullable::<String>(Record::owner.organization.label.into_expr());
    nullable::<bool>(Record::owner.enabled.eq(true));
    nullable::<bool>(Record::owner.score.gt(10_i32));
    nullable::<bool>(Record::owner.score.gt(Member::score));
    nullable::<bool>(Record::owner.note.eq(None));
    nullable::<bool>(Record::owner.label.in_(["atlas", "birch"]));
    nullable::<bool>(Record::owner.label.ilike("a%"));
    nullable::<bool>(Record::owner.enabled.eq(true).and(Record::enabled.eq(true)));
    nullable::<bool>(Record::owner.enabled.eq(true).not());
    required::<bool>(Record::owner.id.is_null());
    required::<bool>(Record::owner.note.is_not_null());
    nullable::<i32>(Record::owner.score + 1_i32);
    nullable::<String>(func::lower(Record::owner.label));
    required::<String>(func::coalesce(Record::owner.note, "missing"));

    let path = Record::owner;
    let column = path.code;
    let predicate = column.eq("a");
    let _: dbkit::Select<Record> = Record::query().filter(predicate.clone());
    let _reused = Record::query().filter(predicate).order_by(Order::asc(path.label));
    let _projection = Record::query()
        .select_only()
        .column_as(path.label, "owner_label")
        .column_as(path.organization.label, "organization_label");
    let _explicit_inner = Record::query().join(path).filter(column.eq("a"));
    let _explicit_outer = Record::query().left_join(path).filter(path.id.is_null());
}
