//@check-pass
use dbkit::model;

#[model(table = "organizations")]
pub struct OrganizationModel {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
}

#[model(table = "users")]
pub struct UserModel {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub organization_id: i64,
    #[belongs_to(key = organization_id, references = id)]
    pub organization: dbkit::BelongsTo<OrganizationModel>,
    pub email: String,
}

fn main() {
    let _query = UserModel::query().filter(UserModel::email.eq("a@b.com"));
    let _insert = UserModel::insert(UserModelInsert {
        organization_id: 1,
        email: "a@b.com".to_string(),
    });

    let _active = UserModelActive {
        id: dbkit::ActiveValue::unchanged(1),
        organization_id: dbkit::ActiveValue::unchanged(1),
        email: dbkit::ActiveValue::Set("a@b.com".to_string()),
    };

    let loaded: UserModel<Option<OrganizationModel>> = UserModel {
        id: 1,
        organization_id: 1,
        organization: Some(OrganizationModel {
            id: 1,
            name: "Acme".to_string(),
        }),
        email: "a@b.com".to_string(),
    };
    let _organization = loaded.organization_loaded();
}
