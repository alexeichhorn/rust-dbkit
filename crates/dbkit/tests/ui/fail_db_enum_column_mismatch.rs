use dbkit::model;

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "build_stage", rename_all = "snake_case")]
pub enum BuildStage {
    Planned,
    Built,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::DbEnum)]
#[dbkit(type_name = "deploy_stage", rename_all = "snake_case")]
pub enum DeployStage {
    Planned,
    Deployed,
}

#[model(table = "builds")]
pub struct Build {
    #[key]
    pub id: i64,
    pub stage: BuildStage,
}

#[model(table = "deployments")]
pub struct Deployment {
    #[key]
    pub id: i64,
    pub stage: DeployStage,
}

fn main() {
    let _expr = Build::stage.eq_col(Deployment::stage); //~ E0308
}
