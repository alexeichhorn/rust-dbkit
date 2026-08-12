use dbkit::{model, NotLoaded};

#[model(table = "users")]
struct User {
    #[key]
    id: i64,
    organization_id: i64,
    name: String,
    #[has_many]
    todos: dbkit::HasMany<Todo>,
    #[belongs_to(key = organization_id, references = id)]
    organization: dbkit::BelongsTo<Organization>,
    #[many_to_many(through = UserGroup, left_key = user_id, right_key = group_id)]
    groups: dbkit::ManyToMany<Group>,
}

#[model(table = "todos")]
struct Todo {
    #[key]
    id: i64,
    user_id: i64,
    title: String,
    #[belongs_to(key = user_id, references = id)]
    user: dbkit::BelongsTo<User>,
}

#[model(table = "organizations")]
struct Organization {
    #[key]
    id: i64,
    name: String,
}

#[model(table = "groups")]
struct Group {
    #[key]
    id: i64,
    name: String,
}

#[model(table = "user_groups")]
struct UserGroup {
    #[key]
    user_id: i64,
    #[key]
    group_id: i64,
}

fn assert_erases_to_unloaded(user: impl Into<User>, expected_id: i64) {
    let user: User = user.into();

    assert_eq!(user.id, expected_id);
    assert_eq!(user.organization_id, expected_id + 100);
    assert_eq!(user.name, format!("User {expected_id}"));
}

fn user_id(user: User) -> i64 {
    user.id
}

fn summarize_todos(user: User<Vec<Todo>>) -> (i64, Vec<i64>) {
    (user.id, user.todos.into_iter().map(|todo| todo.id).collect())
}

fn summarize_organization(user: User<NotLoaded, Option<Organization>>) -> (i64, Option<i64>) {
    (user.id, user.organization.map(|organization| organization.id))
}

fn summarize_groups(user: User<NotLoaded, NotLoaded, Vec<Group>>) -> (i64, Vec<i64>) {
    (user.id, user.groups.into_iter().map(|group| group.id).collect())
}

fn summarize_todos_and_organization(user: User<Vec<Todo>, Option<Organization>>) -> (i64, Vec<i64>, Option<i64>) {
    (
        user.id,
        user.todos.into_iter().map(|todo| todo.id).collect(),
        user.organization.map(|organization| organization.id),
    )
}

fn summarize_todos_and_groups(user: User<Vec<Todo>, NotLoaded, Vec<Group>>) -> (i64, Vec<i64>, Vec<i64>) {
    (
        user.id,
        user.todos.into_iter().map(|todo| todo.id).collect(),
        user.groups.into_iter().map(|group| group.id).collect(),
    )
}

fn summarize_organization_and_groups(user: User<NotLoaded, Option<Organization>, Vec<Group>>) -> (i64, Option<i64>, Vec<i64>) {
    (
        user.id,
        user.organization.map(|organization| organization.id),
        user.groups.into_iter().map(|group| group.id).collect(),
    )
}

fn nested_todo_count(user: User<Vec<Todo<Option<User>>>>) -> usize {
    user.todos.len()
}

fn loaded_todos(user_id: i64) -> Vec<Todo> {
    vec![Todo {
        id: user_id * 10,
        user_id,
        title: format!("Todo {user_id}"),
        user: NotLoaded,
    }]
}

fn loaded_organization(user_id: i64) -> Option<Organization> {
    Some(Organization {
        id: user_id + 100,
        name: format!("Organization {user_id}"),
    })
}

fn loaded_groups(user_id: i64) -> Vec<Group> {
    vec![Group {
        id: user_id + 200,
        name: format!("Group {user_id}"),
    }]
}

fn user<Todos, OrganizationState, Groups>(
    id: i64,
    todos: Todos,
    organization: OrganizationState,
    groups: Groups,
) -> User<Todos, OrganizationState, Groups>
where
    Todos: user_todos_state::State,
    OrganizationState: user_organization_state::State,
    Groups: user_groups_state::State,
{
    User {
        id,
        organization_id: id + 100,
        name: format!("User {id}"),
        todos,
        organization,
        groups,
    }
}

#[test]
fn unloaded_model_keeps_standard_identity_conversion() {
    assert_erases_to_unloaded(user(1, NotLoaded, NotLoaded, NotLoaded), 1);
}

#[test]
fn each_loaded_relation_kind_converts_into_unloaded_model() {
    let user_with_todos: User = user(
        1,
        vec![Todo {
            id: 10,
            user_id: 1,
            title: "Ship it".to_string(),
            user: NotLoaded,
        }],
        NotLoaded,
        NotLoaded,
    )
    .into();
    assert_eq!(user_with_todos.id, 1);

    assert_erases_to_unloaded(
        user(
            2,
            NotLoaded,
            Some(Organization {
                id: 102,
                name: "Acme".to_string(),
            }),
            NotLoaded,
        ),
        2,
    );

    assert_erases_to_unloaded(
        user(
            3,
            NotLoaded,
            NotLoaded,
            vec![Group {
                id: 20,
                name: "Maintainers".to_string(),
            }],
        ),
        3,
    );
}

#[test]
fn every_loaded_and_unloaded_relation_combination_converts() {
    assert_erases_to_unloaded(user(1, NotLoaded, NotLoaded, NotLoaded), 1);
    assert_erases_to_unloaded(user(2, Vec::<Todo>::new(), NotLoaded, NotLoaded), 2);
    assert_erases_to_unloaded(user(3, NotLoaded, None::<Organization>, NotLoaded), 3);
    assert_erases_to_unloaded(user(4, NotLoaded, NotLoaded, Vec::<Group>::new()), 4);
    assert_erases_to_unloaded(user(5, Vec::<Todo>::new(), None::<Organization>, NotLoaded), 5);
    assert_erases_to_unloaded(user(6, Vec::<Todo>::new(), NotLoaded, Vec::<Group>::new()), 6);
    assert_erases_to_unloaded(user(7, NotLoaded, None::<Organization>, Vec::<Group>::new()), 7);
    assert_erases_to_unloaded(user(8, Vec::<Todo>::new(), None::<Organization>, Vec::<Group>::new()), 8);
}

#[test]
fn nested_loaded_relation_models_also_convert() {
    let nested_todos: Vec<Todo<Option<User>>> = Vec::new();

    assert_erases_to_unloaded(user(1, nested_todos, NotLoaded, NotLoaded), 1);
}

#[test]
fn loaded_models_can_be_passed_to_unloaded_parameters_with_into() {
    let todos_loaded = user(1, Vec::<Todo>::new(), NotLoaded, NotLoaded);
    assert_eq!(user_id(todos_loaded.into()), 1);

    let multiple_relations_loaded = user(2, Vec::<Todo>::new(), None::<Organization>, Vec::<Group>::new());
    assert_eq!(user_id(multiple_relations_loaded.into()), 2);

    let nested_todos: Vec<Todo<Option<User>>> = Vec::new();
    let nested_relation_loaded = user(3, nested_todos, NotLoaded, NotLoaded);
    assert_eq!(user_id(nested_relation_loaded.into()), 3);
}

#[test]
fn every_partial_relation_unload_converts_and_preserves_retained_relations() {
    let todos_and_organization = user(1, loaded_todos(1), loaded_organization(1), NotLoaded);
    assert_eq!(summarize_todos(todos_and_organization.clone().into()), (1, vec![10]));
    assert_eq!(summarize_organization(todos_and_organization.into()), (1, Some(101)));

    let todos_and_groups = user(2, loaded_todos(2), NotLoaded, loaded_groups(2));
    assert_eq!(summarize_todos(todos_and_groups.clone().into()), (2, vec![20]));
    assert_eq!(summarize_groups(todos_and_groups.into()), (2, vec![202]));

    let organization_and_groups = user(3, NotLoaded, loaded_organization(3), loaded_groups(3));
    assert_eq!(summarize_organization(organization_and_groups.clone().into()), (3, Some(103)));
    assert_eq!(summarize_groups(organization_and_groups.into()), (3, vec![203]));

    let all_loaded = user(4, loaded_todos(4), loaded_organization(4), loaded_groups(4));
    assert_eq!(summarize_todos(all_loaded.clone().into()), (4, vec![40]));
    assert_eq!(summarize_organization(all_loaded.clone().into()), (4, Some(104)));
    assert_eq!(summarize_groups(all_loaded.clone().into()), (4, vec![204]));
    assert_eq!(
        summarize_todos_and_organization(all_loaded.clone().into()),
        (4, vec![40], Some(104)),
    );
    assert_eq!(summarize_todos_and_groups(all_loaded.clone().into()), (4, vec![40], vec![204]),);
    assert_eq!(summarize_organization_and_groups(all_loaded.into()), (4, Some(104), vec![204]),);
}

#[test]
fn partial_unload_preserves_nested_loaded_relation_types() {
    let nested_todos: Vec<Todo<Option<User>>> = Vec::new();
    let nested_todos_and_organization = user(1, nested_todos, loaded_organization(1), NotLoaded);

    assert_eq!(nested_todo_count(nested_todos_and_organization.into()), 0);
}
