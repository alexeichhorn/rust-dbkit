//! Proposed contract for BelongsTo column paths, intentionally red until implemented.
//!
//! `Record::owner.enabled` is a typed, read-only SQL column path. Using a path
//! adds LEFT JOINs without loading model fields. `.with(...)` still controls loading.
//! Repeated paths share joins; different paths to the same table do not.
//! Explicit `.join(path)` retains INNER JOIN semantics and shares its join with loading.
//! Paths compose through BelongsTo, including self-relations. Collection predicates
//! and mutation syntax are outside this specification.
//! Related expressions are nullable, even for required target columns, because the
//! related row can be absent. Existing Option columns do not become nested Options.
//! Alias spelling is internal: tests inspect its use, not a particular naming scheme.

#[path = "support/relation_path_graphs.rs"]
mod relation_path_graphs;
#[path = "support/relation_paths.rs"]
mod relation_paths;

use dbkit::executor::BoxFuture;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{func, Error, Executor, Order, SelectExt, Value};
use relation_path_graphs::{Assignment, Node};
use relation_paths::{Member, Organization, Record};
use std::sync::Mutex;

// Only inspect dbkit's generated JOIN clauses; execution semantics are tested against PostgreSQL.
fn aliases<'a>(sql: &'a str, table: &str) -> Vec<&'a str> {
    sql.split(&format!(" JOIN {table} "))
        .skip(1)
        .map(|rest| {
            let alias = rest.split_once(" ON ").expect("JOIN needs ON").0;
            let alias = alias.strip_prefix("AS ").unwrap_or(alias);
            assert!(!alias.is_empty() && !alias.contains(' '), "expected a table alias: {sql}");
            alias
        })
        .collect()
}

fn only_alias<'a>(sql: &'a str, table: &str) -> &'a str {
    let found = aliases(sql, table);
    assert_eq!(found.len(), 1, "expected one join of {table}: {sql}");
    found[0]
}

#[test]
fn filter_adds_a_left_join_without_selecting_or_loading_the_relation() {
    let query: dbkit::Select<Record> = Record::query().filter(Record::owner.enabled.eq(true));
    let compiled = query.compile();
    let owner = only_alias(&compiled.sql, "path_members");

    assert!(compiled
        .sql
        .starts_with("SELECT path_records.* FROM path_records LEFT JOIN path_members "));
    assert!(compiled.sql.contains(&format!("({owner}.id = path_records.owner_id)")));
    assert!(compiled.sql.ends_with(&format!("WHERE ({owner}.enabled = $1)")));
    assert_eq!(compiled.binds, vec![Value::Bool(true)]);
}

#[test]
fn repeated_columns_and_computed_expressions_reuse_the_path() {
    let compiled = Record::query()
        .filter(Record::owner.enabled.eq(true))
        .filter(func::lower(Record::owner.label).eq("atlas"))
        .filter((Record::owner.score + 1_i32).gt(10_i32))
        .order_by(Order::desc(Record::owner.score))
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");

    assert!(compiled.sql.contains(&format!("LOWER({owner}.label)")));
    assert!(compiled.sql.contains(&format!("({owner}.score + $3)")));
    assert!(compiled.sql.ends_with(&format!("ORDER BY {owner}.score DESC")));
    assert_eq!(
        compiled.binds,
        vec![Value::Bool(true), Value::String("atlas".into()), Value::I32(1), Value::I32(10)]
    );
}

#[test]
fn or_and_null_checks_keep_the_automatic_join_outer() {
    let compiled = Record::query()
        .filter(Record::enabled.eq(true).or(Record::owner.id.is_null()))
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");

    assert_eq!(compiled.sql.matches(" LEFT JOIN ").count(), 1);
    assert!(compiled
        .sql
        .contains(&format!("((path_records.enabled = $1) OR ({owner}.id IS NULL))")));
}

#[test]
fn renamed_columns_use_database_names_and_values_stay_bound() {
    let value = "x' OR true --";
    let compiled = Record::query().filter(Record::owner.code.eq(value)).compile();
    let owner = only_alias(&compiled.sql, "path_members");

    assert!(compiled.sql.ends_with(&format!("WHERE ({owner}.external_ref = $1)")));
    assert!(!compiled.sql.contains(value));
    assert_eq!(compiled.binds, vec![Value::String(value.into())]);
}

#[test]
fn order_only_paths_add_joins_before_pagination() {
    let compiled = Record::query()
        .order_by(Order::asc(Record::owner.label))
        .order_by(Order::asc(Record::id))
        .limit(2)
        .offset(1)
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");

    assert!(compiled
        .sql
        .ends_with(&format!("ORDER BY {owner}.label ASC, path_records.id ASC LIMIT 2 OFFSET 1")));
    assert!(compiled.binds.is_empty());
}

#[test]
fn projection_only_paths_add_joins_and_keep_output_aliases() {
    let compiled = Record::query()
        .select_only()
        .column(Record::id)
        .column_as(Record::owner.label, "owner_label")
        .column_as(func::coalesce(Record::owner.note, "missing"), "note")
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");

    assert!(compiled.sql.starts_with(&format!(
        "SELECT path_records.id, {owner}.label AS owner_label, COALESCE({owner}.note, $1) AS note FROM "
    )));
    assert_eq!(compiled.binds, vec![Value::String("missing".into())]);
}

#[test]
fn group_by_and_having_also_discover_relation_paths() {
    let grouped = Record::query()
        .select_only()
        .column(func::count(Record::id))
        .group_by(Record::owner.enabled)
        .compile();
    let owner = only_alias(&grouped.sql, "path_members");
    assert!(grouped.sql.ends_with(&format!("GROUP BY {owner}.enabled")));

    let having = Record::query()
        .select_only()
        .column(func::count(Record::id))
        .having(func::count(Record::owner.id).gt(0_i64))
        .compile();
    let owner = only_alias(&having.sql, "path_members");
    assert!(having.sql.contains(&format!("HAVING (COUNT({owner}.id) > $1)")));
    assert_eq!(having.binds, vec![Value::I64(0)]);
}

#[test]
fn explicit_join_kind_is_preserved_without_an_extra_implicit_join() {
    let inner = Record::query().join(Record::owner).filter(Record::owner.enabled.eq(true)).compile();
    only_alias(&inner.sql, "path_members");
    assert!(!inner.sql.contains("LEFT JOIN"), "explicit INNER JOIN changed: {}", inner.sql);

    let outer = Record::query()
        .filter(Record::owner.enabled.eq(true))
        .left_join(Record::owner)
        .compile();
    only_alias(&outer.sql, "path_members");
    assert_eq!(outer.sql.matches("LEFT JOIN").count(), 1);
}

#[test]
fn custom_table_join_keeps_its_columns_when_a_relation_path_uses_the_same_table() {
    // A custom ON condition is not interchangeable with the declared relation.
    let compiled = Record::query()
        .left_join_on(Member::TABLE, Member::id.eq_col(Record::owner_id).and(Member::score.gt(25_i32)))
        .filter(Member::note.is_null())
        .filter(Record::owner.code.eq("c"))
        .compile();
    assert!(
        compiled
            .sql
            .contains("LEFT JOIN path_members ON ((path_members.id = path_records.owner_id)"),
        "{}",
        compiled.sql
    );
    assert!(compiled.sql.contains("WHERE (path_members.note IS NULL)"), "{}", compiled.sql);
    assert_eq!(compiled.sql.matches("JOIN path_members ").count(), 2);
}

#[test]
fn nested_paths_share_their_prefix_and_join_in_dependency_order() {
    let compiled = Record::query()
        .filter(Record::owner.organization.label.eq("north"))
        .filter(Record::owner.enabled.eq(true))
        .order_by(Order::asc(Record::owner.organization.id))
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");
    let organization = only_alias(&compiled.sql, "path_organizations");

    assert_ne!(owner, organization);
    assert!(compiled.sql.find("JOIN path_members").unwrap() < compiled.sql.find("JOIN path_organizations").unwrap());
    assert!(compiled.sql.contains(&format!("({organization}.id = {owner}.organization_id)")));
    assert!(compiled.sql.contains(&format!("({organization}.label = $1)")));
    assert_eq!(compiled.binds, vec![Value::String("north".into()), Value::Bool(true)]);
}

#[test]
fn sibling_paths_keep_distinct_aliases_and_their_own_reference_keys() {
    let compiled = Assignment::query()
        .filter(Assignment::first.enabled.eq(true))
        .filter(Assignment::second.enabled.eq(false))
        .filter(Assignment::first.score.gt(Assignment::second.score))
        .compile();
    let members = aliases(&compiled.sql, "path_members");
    assert_eq!(members.len(), 2, "{}", compiled.sql);
    assert_ne!(members[0], members[1]);
    let first = members
        .iter()
        .find(|alias| compiled.sql.contains(&format!("({alias}.id = path_assignments.first_id)")))
        .unwrap();
    let second = members
        .iter()
        .find(|alias| {
            compiled
                .sql
                .contains(&format!("({alias}.external_ref = path_assignments.second_code)"))
        })
        .unwrap();
    assert_ne!(first, second);
    assert!(compiled.sql.contains(&format!("({first}.enabled = $1)")));
    assert!(compiled.sql.contains(&format!("({second}.enabled = $2)")));
    assert!(compiled.sql.contains(&format!("({first}.score > {second}.score)")));
    assert_eq!(compiled.binds, vec![Value::Bool(true), Value::Bool(false)]);
}

#[test]
fn nested_sibling_paths_do_not_merge_at_the_same_target_table() {
    let compiled = Assignment::query()
        .filter(Assignment::first.organization.label.eq("north"))
        .filter(Assignment::second.organization.label.eq("south"))
        .compile();
    let members = aliases(&compiled.sql, "path_members");
    let organizations = aliases(&compiled.sql, "path_organizations");
    assert_eq!(members.len(), 2);
    assert_eq!(organizations.len(), 2);
    assert_ne!(organizations[0], organizations[1]);
    for member in members {
        assert_eq!(
            organizations
                .iter()
                .filter(|organization| { compiled.sql.contains(&format!("({organization}.id = {member}.organization_id)")) })
                .count(),
            1,
            "{}",
            compiled.sql
        );
    }
}

#[test]
fn self_paths_keep_the_base_parent_and_grandparent_distinct() {
    let compiled = Node::query()
        .filter(Node::label.eq("leaf"))
        .filter(Node::parent.label.eq("branch"))
        .filter(Node::parent.parent.label.eq("root"))
        .compile();
    let nodes = aliases(&compiled.sql, "path_nodes");
    assert_eq!(nodes.len(), 2);
    assert_ne!(nodes[0], nodes[1]);
    assert!(nodes.iter().all(|alias| *alias != "path_nodes"));
    let parent = nodes
        .iter()
        .find(|alias| compiled.sql.contains(&format!("({alias}.id = path_nodes.parent_id)")))
        .unwrap();
    let grandparent = nodes
        .iter()
        .find(|alias| compiled.sql.contains(&format!("({alias}.id = {parent}.parent_id)")))
        .unwrap();
    assert!(compiled.sql.contains(&format!("({parent}.label = $2)")));
    assert!(compiled.sql.contains(&format!("({grandparent}.label = $3)")));
    assert_eq!(
        compiled.binds,
        vec![
            Value::String("leaf".into()),
            Value::String("branch".into()),
            Value::String("root".into())
        ]
    );
}

#[test]
fn subquery_paths_stay_in_the_subquery_and_preserve_correlated_base_columns() {
    let compiled = Organization::query()
        .filter(Organization::label.eq("north"))
        .where_exists(
            Record::query()
                .select_only()
                .column(Record::id)
                .filter(Record::owner.organization_id.eq(Organization::id))
                .filter(Record::owner.enabled.eq(true)),
        )
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");
    let (outer, inner) = compiled.sql.split_once("EXISTS (").expect("EXISTS subquery");
    assert!(!outer.contains("JOIN"), "subquery join leaked into outer query: {}", compiled.sql);
    assert!(inner.contains(&format!("({owner}.organization_id = path_organizations.id)")));
    assert!(inner.contains(&format!("({owner}.enabled = $2)")));
    assert_eq!(compiled.binds, vec![Value::String("north".into()), Value::Bool(true)]);
}

#[test]
fn correlated_exists_preserves_outer_columns_with_automatic_and_declared_relation_joins() {
    for inner in [
        Record::query(),
        Record::query().join(Record::owner),
        Record::query().left_join(Record::owner),
    ] {
        let compiled = Member::query()
            .where_exists(inner.filter(Record::owner.id.eq(Member::id)))
            .compile();
        let owner = only_alias(&compiled.sql, "path_members");
        assert!(
            compiled.sql.contains(&format!("({owner}.id = path_members.id)")),
            "{}",
            compiled.sql
        );
        assert!(!compiled.sql.contains(&format!("({owner}.id = {owner}.id)")), "{}", compiled.sql);
        assert!(compiled.binds.is_empty());
    }
}

#[test]
fn correlated_not_exists_keeps_outer_filters_and_bind_order() {
    let compiled = Member::query()
        .filter(Member::label.eq("Atlas"))
        .where_not_exists(
            Record::query()
                .filter(Record::owner.id.eq(Member::id))
                .filter(Record::owner.score.gt(25_i32)),
        )
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");
    assert!(compiled.sql.contains("NOT (EXISTS ("));
    assert!(
        compiled.sql.contains(&format!("({owner}.id = path_members.id)")),
        "{}",
        compiled.sql
    );
    assert!(compiled.sql.contains(&format!("({owner}.score > $2)")), "{}", compiled.sql);
    assert_eq!(compiled.binds, vec![Value::String("Atlas".into()), Value::I32(25)]);
}

#[test]
fn correlated_renamed_computed_and_nullable_columns_keep_the_outer_qualifier() {
    let compiled = Member::query()
        .where_exists(
            Record::query()
                .filter(Record::owner.code.eq(Member::code))
                .filter(func::lower(Record::owner.label).eq_col(Member::label))
                .filter(Member::score.gt(Record::owner.score))
                .filter(Record::owner.note.is_distinct_from_col(Member::note)),
        )
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");
    for expected in [
        format!("({owner}.external_ref = path_members.external_ref)"),
        format!("(LOWER({owner}.label) = path_members.label)"),
        format!("(path_members.score > {owner}.score)"),
        format!("({owner}.note IS DISTINCT FROM path_members.note)"),
    ] {
        assert!(compiled.sql.contains(&expected), "missing {expected}: {}", compiled.sql);
    }
}

#[test]
fn correlated_nested_relation_keeps_the_outer_target_table() {
    let compiled = Organization::query()
        .where_exists(Record::query().filter(Record::owner.organization.id.eq(Organization::id)))
        .compile();
    only_alias(&compiled.sql, "path_members");
    let organization = only_alias(&compiled.sql, "path_organizations");
    assert!(
        compiled.sql.contains(&format!("({organization}.id = path_organizations.id)")),
        "{}",
        compiled.sql
    );
}

#[test]
fn correlated_columns_can_reference_both_enclosing_query_levels() {
    let compiled = Member::query()
        .where_exists(
            Organization::query().where_exists(
                Record::query()
                    .filter(Record::owner.id.eq(Member::id))
                    .filter(Record::owner.organization_id.eq(Organization::id)),
            ),
        )
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");
    assert_eq!(compiled.sql.matches("EXISTS (").count(), 2);
    assert!(
        compiled.sql.contains(&format!("({owner}.id = path_members.id)")),
        "{}",
        compiled.sql
    );
    assert!(
        compiled.sql.contains(&format!("({owner}.organization_id = path_organizations.id)")),
        "{}",
        compiled.sql
    );
}

#[test]
fn correlated_exists_projection_preserves_outer_column_identity() {
    let compiled = Member::query()
        .select_only()
        .column(Member::id)
        .column_as(func::exists(Record::query().filter(Record::owner.id.eq(Member::id))), "has_records")
        .compile();
    let owner = only_alias(&compiled.sql, "path_members");
    assert!(compiled.sql.starts_with("SELECT path_members.id, EXISTS ("));
    assert!(compiled.sql.contains(" AS has_records FROM path_members"));
    assert!(
        compiled.sql.contains(&format!("({owner}.id = path_members.id)")),
        "{}",
        compiled.sql
    );
}

#[test]
fn correlated_outer_alias_cannot_be_shadowed_by_an_automatic_join_alias() {
    // Include a name currently used by the automatic allocator. Its spelling is
    // not a contract; respecting an explicitly named enclosing table is.
    for outer_alias in ["outer_member", "__dbkit_r0"] {
        let table = Member::TABLE.with_alias(outer_alias);
        let outer_id = dbkit::Column::<Member, i64>::new(table, "id");
        let compiled = dbkit::Select::<Member>::new(table)
            .where_exists(Record::query().filter(Record::owner.id.eq(outer_id)))
            .compile();
        let owner = only_alias(&compiled.sql, "path_members");
        assert_ne!(owner, outer_alias, "inner alias shadows outer table: {}", compiled.sql);
        assert!(
            compiled.sql.contains(&format!("({owner}.id = {outer_alias}.id)")),
            "{}",
            compiled.sql
        );
    }
}

#[test]
fn correlated_reused_exists_expression_is_resolved_in_each_enclosing_scope() {
    let inner = Record::query().filter(Record::owner.id.eq(Member::id));
    let standalone = inner.compile();
    let predicate = func::exists(inner.clone());
    let correlated = Member::query().filter(predicate.clone()).compile();
    let local = Organization::query().filter(predicate).compile();

    let owner = only_alias(&correlated.sql, "path_members");
    assert!(
        correlated.sql.contains(&format!("({owner}.id = path_members.id)")),
        "{}",
        correlated.sql
    );
    // No enclosing Member binding exists here, so the legacy local table-column
    // spelling still refers to this query's joined owner. Compilation must not mutate the reusable expression.
    let local_owner = only_alias(&local.sql, "path_members");
    assert!(
        local.sql.contains(&format!("({local_owner}.id = {local_owner}.id)")),
        "{}",
        local.sql
    );
    assert_eq!(inner.compile(), standalone);
}

#[test]
fn correlated_sibling_paths_keep_local_comparisons_distinct_from_outer_columns() {
    let compiled = Member::query()
        .where_exists(
            Assignment::query()
                .filter(Assignment::first.id.eq(Member::id))
                .filter(Assignment::first.score.gt(Assignment::second.score)),
        )
        .compile();
    let members = aliases(&compiled.sql, "path_members");
    assert_eq!(members.len(), 2);
    let first = members
        .iter()
        .find(|alias| compiled.sql.contains(&format!("({alias}.id = path_assignments.first_id)")))
        .unwrap();
    let second = members
        .iter()
        .find(|alias| {
            compiled
                .sql
                .contains(&format!("({alias}.external_ref = path_assignments.second_code)"))
        })
        .unwrap();
    assert_ne!(first, second);
    assert!(
        compiled.sql.contains(&format!("({first}.id = path_members.id)")),
        "{}",
        compiled.sql
    );
    assert!(
        compiled.sql.contains(&format!("({first}.score > {second}.score)")),
        "{}",
        compiled.sql
    );
}

#[test]
fn compiling_and_cloning_do_not_accumulate_joins_or_change_aliases() {
    let query = Record::query().filter(Record::owner.enabled.eq(true));
    let first = query.compile();
    assert_eq!(query.compile(), first);
    assert_eq!(query.clone().compile(), first);
    let extended = query.clone().filter(Record::owner.organization.label.eq("north")).compile();
    only_alias(&extended.sql, "path_members");
    only_alias(&extended.sql, "path_organizations");
    assert_eq!(query.compile(), first);
}

#[derive(Default)]
struct CaptureExecutor(Mutex<Vec<String>>);

impl Executor for CaptureExecutor {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, _: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.0.lock().unwrap().push(sql.to_owned());
        Box::pin(async { Ok(Vec::new()) })
    }

    fn fetch_optional<'e, T>(&'e self, sql: &'e str, _: PgArguments) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.0.lock().unwrap().push(sql.to_owned());
        Box::pin(async { Ok(None) })
    }

    fn fetch_rows<'e>(&'e self, sql: &'e str, _: PgArguments) -> BoxFuture<'e, Result<Vec<dbkit::sqlx::postgres::PgRow>, Error>> {
        self.0.lock().unwrap().push(sql.to_owned());
        Box::pin(async { Ok(Vec::new()) })
    }

    fn execute<'e>(&'e self, _: &'e str, _: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        panic!("read-only query must not call execute")
    }
}

#[tokio::test]
async fn filtering_and_joined_loading_share_one_join_regardless_of_builder_order() -> Result<(), Error> {
    let ex = CaptureExecutor::default();
    let _: Vec<Record<Option<Member>>> = Record::query()
        .filter(Record::owner.enabled.eq(true))
        .with(Record::owner.joined())
        .all(&ex)
        .await?;
    let _: Vec<Record<Option<Member>>> = Record::query()
        .with(Record::owner.joined())
        .filter(Record::owner.enabled.eq(true))
        .all(&ex)
        .await?;

    let sqls = ex.0.lock().unwrap();
    assert_eq!(sqls.len(), 2);
    assert_eq!(sqls[0], sqls[1]);
    let owner = only_alias(&sqls[0], "path_members");
    assert!(sqls[0].contains(&format!("{owner}.label AS ")));
    assert!(sqls[0].contains(&format!("({owner}.enabled = $1)")));
    Ok(())
}

#[tokio::test]
async fn explicit_join_and_joined_loading_share_the_relation_join() -> Result<(), Error> {
    let ex = CaptureExecutor::default();
    let _: Vec<Record<Option<Member>>> = Record::query()
        .join(Record::owner)
        .filter(Record::owner.enabled.eq(true))
        .with(Record::owner.joined())
        .all(&ex)
        .await?;
    let sqls = ex.0.lock().unwrap();
    assert_eq!(sqls.len(), 1);
    only_alias(&sqls[0], "path_members");
    assert!(!sqls[0].contains("LEFT JOIN"));
    Ok(())
}

#[tokio::test]
async fn nested_filter_and_nested_loader_reuse_the_entire_path() -> Result<(), Error> {
    let ex = CaptureExecutor::default();
    let _: Vec<Record<Option<Member<Option<Organization>>>>> = Record::query()
        .filter(Record::owner.organization.label.eq("north"))
        .with(Record::owner.joined().with(Member::organization.joined()))
        .all(&ex)
        .await?;
    let sqls = ex.0.lock().unwrap();
    assert_eq!(sqls.len(), 1);
    let owner = only_alias(&sqls[0], "path_members");
    let organization = only_alias(&sqls[0], "path_organizations");
    assert!(sqls[0].contains(&format!("{owner}.label AS ")));
    assert!(sqls[0].contains(&format!("{organization}.label AS ")));
    Ok(())
}

#[tokio::test]
async fn count_and_exists_include_filter_joins_without_loading_columns() -> Result<(), Error> {
    let ex = CaptureExecutor::default();
    let query = Record::query()
        .filter(Record::owner.enabled.eq(true))
        .with(Record::owner.joined())
        .limit(1)
        .offset(2);
    query.count(&ex).await?;
    query.exists(&ex).await?;
    let sqls = ex.0.lock().unwrap();
    assert_eq!(sqls.len(), 2);
    for sql in sqls.iter() {
        let owner = only_alias(sql, "path_members");
        assert!(sql.contains(&format!("({owner}.enabled = $1)")));
        assert!(!sql.contains("LIMIT") && !sql.contains("OFFSET"));
        assert!(!sql.contains(&format!("{owner}.label AS ")));
    }
    Ok(())
}
