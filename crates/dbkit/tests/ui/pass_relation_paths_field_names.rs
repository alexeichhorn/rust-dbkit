//@check-pass
use dbkit::{model, Expr};

mod targets {
    use dbkit::model;

    #[model(table = "named_targets")]
    pub struct TargetModel {
        #[key]
        pub id: i64,
        pub joined: bool,
        pub selectin: bool,
        pub r#type: String,
        #[dbkit(column = "external_ref")]
        pub code: String,
    }
}

#[model(table = "named_sources")]
pub struct SourceModel {
    #[key]
    pub id: i64,
    pub target_id: i64,
    #[belongs_to(key = target_id, references = id)]
    pub target: dbkit::BelongsTo<targets::TargetModel>,
}

fn main() {
    let _: Expr<Option<bool>> = SourceModel::target.joined.eq(true);
    let _: Expr<Option<bool>> = SourceModel::target.selectin.eq(false);
    let _: Expr<Option<bool>> = SourceModel::target.r#type.eq("primary");
    let _ = SourceModel::query()
        .with(SourceModel::target.joined())
        .filter(SourceModel::target.joined.eq(true));
    let _ = SourceModel::query()
        .with(SourceModel::target.selectin())
        .filter(SourceModel::target.selectin.eq(true));
    let _ = SourceModel::query().filter(SourceModel::target.code.eq("a"));
}
