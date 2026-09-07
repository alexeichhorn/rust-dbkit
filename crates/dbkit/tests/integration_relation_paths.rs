#[path = "support/relation_path_graphs.rs"]
mod relation_path_graphs;
#[path = "support/relation_paths.rs"]
mod relation_paths;

use dbkit::sqlx::postgres::PgArguments;
use dbkit::{func, Database, Error, Executor, NotLoaded, Order, SelectExt};
use relation_path_graphs::{Assignment, Node};
use relation_paths::{Member, Organization, Record};

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup(ex: &(impl Executor + Send + Sync)) -> Result<(), Error> {
    // Temporary tables and a rolled-back transaction isolate every test. Missing
    // targets are deliberate: model annotations alone do not enforce foreign keys.
    for sql in [
        "CREATE TEMP TABLE path_organizations (id BIGINT PRIMARY KEY, label TEXT NOT NULL) ON COMMIT DROP",
        "CREATE TEMP TABLE path_members (
            id BIGINT PRIMARY KEY, organization_id BIGINT NOT NULL, label TEXT NOT NULL,
            enabled BOOLEAN NOT NULL, score INTEGER NOT NULL, note TEXT,
            external_ref TEXT NOT NULL UNIQUE) ON COMMIT DROP",
        "CREATE TEMP TABLE path_records (
            id BIGINT PRIMARY KEY, owner_id BIGINT, label TEXT NOT NULL, enabled BOOLEAN NOT NULL) ON COMMIT DROP",
        "CREATE TEMP TABLE path_assignments (
            id BIGINT PRIMARY KEY, first_id BIGINT, second_code TEXT) ON COMMIT DROP",
        "CREATE TEMP TABLE path_nodes (
            id BIGINT PRIMARY KEY, parent_id BIGINT, label TEXT NOT NULL) ON COMMIT DROP",
        "INSERT INTO path_organizations VALUES (1, 'north'), (2, 'south')",
        "INSERT INTO path_members VALUES
            (1, 1, 'Atlas', true, 30, NULL, 'a'),
            (2, 2, 'Birch', false, 10, 'memo', 'b'),
            (3, 2, 'Cedar', true, 20, '', 'c'),
            (4, 99, 'Delta', false, 5, NULL, 'd')",
        "INSERT INTO path_records VALUES
            (1, 1, 'first', false), (2, 2, 'second', true), (3, 1, 'third', false),
            (4, 3, 'fourth', false), (5, NULL, 'fifth', true), (6, 99, 'sixth', false),
            (7, 4, 'seventh', false)",
        "INSERT INTO path_assignments VALUES
            (1, 1, 'b'), (2, 2, 'a'), (3, 1, 'a'), (4, NULL, 'a'),
            (5, 1, NULL), (6, 99, 'missing'), (7, 3, 'b'), (8, 2, 'b')",
        "INSERT INTO path_nodes VALUES
            (1, NULL, 'root'), (2, 1, 'branch'), (3, 2, 'leaf'), (4, 99, 'orphan'), (5, 5, 'cycle')",
    ] {
        ex.execute(sql, PgArguments::default()).await?;
    }
    Ok(())
}

async fn setup_correlated(ex: &(impl Executor + Send + Sync)) -> Result<(), Error> {
    setup(ex).await?;
    // The original fixture gives every member a record. Add unmatched outer rows
    // so accidentally uncorrelated EXISTS queries cannot pass by returning everyone.
    ex.execute(
        "INSERT INTO path_members VALUES (5, 1, 'atlas', false, 0, NULL, 'e')",
        PgArguments::default(),
    )
    .await?;
    ex.execute("INSERT INTO path_organizations VALUES (3, 'unused')", PgArguments::default())
        .await?;
    Ok(())
}

#[tokio::test]
async fn correlated_exists_returns_only_members_with_matching_records() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    for inner in [
        Record::query(),
        Record::query().join(Record::owner),
        Record::query().left_join(Record::owner),
    ] {
        let members: Vec<Member> = Member::query()
            .where_exists(inner.filter(Record::owner.id.eq(Member::id)))
            .order_by(Order::asc(Member::id))
            .all(&tx)
            .await?;
        assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4]);
    }
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_not_exists_returns_unmatched_members_despite_missing_record_owners() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let members: Vec<Member> = Member::query()
        .where_not_exists(Record::query().filter(Record::owner.id.eq(Member::id)))
        .order_by(Order::asc(Member::id))
        .all(&tx)
        .await?;
    // NULL and dangling owner IDs in records 5 and 6 must not match any member.
    assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [5]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_exists_projection_is_evaluated_for_each_outer_row() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let rows: Vec<(i64, bool)> = Member::query()
        .select_only()
        .column(Member::id)
        .column_as(func::exists(Record::query().filter(Record::owner.id.eq(Member::id))), "has_records")
        .order_by(Order::asc(Member::id))
        .into_model()
        .all(&tx)
        .await?;
    assert_eq!(rows, [(1, true), (2, true), (3, true), (4, true), (5, false)]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_renamed_column_and_local_predicate_select_the_matching_member() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let members: Vec<Member> = Member::query()
        .where_exists(
            Record::query()
                .filter(Record::owner.code.eq(Member::code))
                .filter(Record::owner.score.gt(25_i32)),
        )
        .order_by(Order::asc(Member::id))
        .all(&tx)
        .await?;
    assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [1]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_computed_comparison_uses_the_outer_text_value() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let members: Vec<Member> = Member::query()
        .where_exists(Record::query().filter(func::lower(Record::owner.label).eq_col(Member::label)))
        .order_by(Order::asc(Member::id))
        .all(&tx)
        .await?;
    // Member 5's label is the lowercase version of member 1's label. Comparing
    // the owner's normalized label to its own original label would find nobody.
    assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [5]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_nullable_comparison_does_not_compare_the_inner_column_to_itself() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let members: Vec<Member> = Member::query()
        .where_exists(
            Record::query()
                .filter(Record::id.eq(1_i64))
                .filter(Record::owner.note.is_distinct_from_col(Member::note)),
        )
        .order_by(Order::asc(Member::id))
        .all(&tx)
        .await?;
    // Record 1's owner has a NULL note; only members 2 and 3 have non-NULL notes.
    assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [2, 3]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_nested_relation_excludes_an_organization_without_records() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let organizations: Vec<Organization> = Organization::query()
        .where_exists(Record::query().filter(Record::owner.organization.id.eq(Organization::id)))
        .order_by(Order::asc(Organization::id))
        .all(&tx)
        .await?;
    assert_eq!(organizations.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_nested_exists_resolves_both_enclosing_models() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let members: Vec<Member> = Member::query()
        .where_exists(
            Organization::query().where_exists(
                Record::query()
                    .filter(Record::owner.id.eq(Member::id))
                    .filter(Record::owner.organization_id.eq(Organization::id)),
            ),
        )
        .order_by(Order::asc(Member::id))
        .all(&tx)
        .await?;
    // Member 4 has a record but no organization; member 5 has no record.
    assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_explicit_outer_alias_is_not_shadowed_by_an_automatic_alias() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    for outer_alias in ["outer_member", "__dbkit_r0"] {
        let table = Member::TABLE.with_alias(outer_alias);
        let outer_id = dbkit::Column::<Member, i64>::new(table, "id");
        let members: Vec<Member> = dbkit::Select::new(table)
            .where_exists(Record::query().filter(Record::owner.id.eq(outer_id)))
            .order_by(Order::asc(outer_id))
            .all(&tx)
            .await?;
        assert_eq!(
            members.iter().map(|row| row.id).collect::<Vec<_>>(),
            [1, 2, 3, 4],
            "outer alias: {outer_alias}"
        );
    }
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_sibling_paths_keep_their_local_comparison_and_outer_identity() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_correlated(&tx).await?;

    let members: Vec<Member> = Member::query()
        .where_exists(
            Assignment::query()
                .filter(Assignment::first.id.eq(Member::id))
                .filter(Assignment::first.score.gt(Assignment::second.score)),
        )
        .order_by(Order::asc(Member::id))
        .all(&tx)
        .await?;
    assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn nested_exists_matches_the_nearer_owner_instead_of_the_outer_member() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let members: Vec<Member> = Member::query()
        .where_exists(
            Record::query()
                .join(Record::owner)
                .filter(Record::id.eq(1_i64))
                .where_exists(Assignment::query().filter(Assignment::first_id.eq_col(Member::id))),
        )
        .order_by(Order::asc(Member::id))
        .all(&tx)
        .await?;
    // Record 1's owner has assignments, independently of the outer member.
    // Resolving the innermost ID to the outer member incorrectly excludes member 4.
    assert_eq!(members.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn nested_not_exists_keeps_a_missing_nearer_relation_instead_of_using_an_outer_relation() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let records: Vec<Record> = Record::query()
        .join(Record::owner)
        .where_exists(
            Assignment::query()
                .left_join(Assignment::first)
                .filter(Assignment::id.eq(4_i64))
                .where_not_exists(Organization::query().filter(Organization::id.eq_col(Member::organization_id))),
        )
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    // Assignment 4 has no first member, so NOT EXISTS succeeds for every joined
    // record. Falling back to the record's owner incorrectly leaves only record 7.
    assert_eq!(records.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4, 7]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_exists_matches_the_outer_relation_join() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let records: Vec<Record> = Record::query()
        .join(Record::owner)
        .where_exists(Assignment::query().filter(Assignment::first_id.eq_col(Member::id)))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    // Owner 4 has no matching assignment; NULL and dangling owners cannot join.
    assert_eq!(records.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_not_exists_preserves_missing_outer_relation_joins() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let records: Vec<Record> = Record::query()
        .left_join(Record::owner)
        .where_not_exists(Assignment::query().filter(Assignment::first_id.eq_col(Member::id)))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    // A NULL owner, a dangling owner, and an owner without assignments all survive.
    assert_eq!(records.iter().map(|row| row.id).collect::<Vec<_>>(), [5, 6, 7]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn correlated_exists_matches_the_outer_joined_loading_relation() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let records: Vec<Record<Option<Member>>> = Record::query()
        .with(Record::owner.joined())
        .where_exists(Assignment::query().filter(Assignment::first_id.eq_col(Member::id)))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(records.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4]);
    assert_eq!(
        records.iter().map(|row| row.owner.as_ref().unwrap().id).collect::<Vec<_>>(),
        [1, 2, 1, 3]
    );
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn filtering_is_independent_of_loading_strategy() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let query = Record::query()
        .filter(Record::owner.enabled.eq(true))
        .order_by(Order::asc(Record::id));
    let bare: Vec<Record> = query.clone().all(&tx).await?;
    assert_eq!(bare.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3, 4]);
    let _: &NotLoaded = &bare[0].owner;

    let joined: Vec<Record<Option<Member>>> = query.clone().with(Record::owner.joined()).all(&tx).await?;
    let selectin: Vec<Record<Option<Member>>> = query.with(Record::owner.selectin()).all(&tx).await?;
    for rows in [joined, selectin] {
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3, 4]);
        assert_eq!(rows.iter().map(|row| row.owner.as_ref().unwrap().id).collect::<Vec<_>>(), [1, 1, 3]);
        assert!(rows.iter().all(|row| row.owner.as_ref().unwrap().enabled));
    }
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn outer_joins_preserve_missing_rows_in_or_null_and_projection_expressions() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let rows: Vec<Record<Option<Member>>> = Record::query()
        .filter(Record::enabled.eq(true).or(Record::owner.enabled.eq(true)))
        .with(Record::owner.joined())
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4, 5]);
    assert!(rows.last().unwrap().owner.is_none());

    let missing: Vec<Record<Option<Member>>> = Record::query()
        .filter(Record::owner.id.is_null())
        .with(Record::owner.joined())
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(missing.iter().map(|row| row.id).collect::<Vec<_>>(), [5, 6]);
    assert!(missing.iter().all(|row| row.owner.is_none()));

    let disabled: Vec<Record> = Record::query()
        .filter(Record::owner.enabled.eq(true).not())
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(disabled.iter().map(|row| row.id).collect::<Vec<_>>(), [2, 7]);

    let rows: Vec<(i64, Option<bool>, Option<String>)> = Record::query()
        .select_only()
        .column(Record::id)
        .column(Record::owner.enabled.eq(true))
        .column(Record::owner.note)
        .order_by(Order::asc(Record::id))
        .into_model()
        .all(&tx)
        .await?;
    assert_eq!(
        rows,
        vec![
            (1, Some(true), None),
            (2, Some(false), Some("memo".into())),
            (3, Some(true), None),
            (4, Some(true), Some("".into())),
            (5, None, None),
            (6, None, None),
            (7, Some(false), None),
        ]
    );
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn expressions_renamed_columns_and_null_checks_use_the_related_row() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let rows: Vec<Record> = Record::query()
        .filter(func::lower(Record::owner.label).eq("atlas"))
        .filter((Record::owner.score + 5_i32).ge(35_i32))
        .filter(Record::owner.code.in_(["a", "b"]))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3]);

    let null_notes: Vec<Record> = Record::query()
        .filter(Record::owner.note.is_null())
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(null_notes.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3, 5, 6, 7]);

    let none: Vec<Record> = Record::query().filter(Record::owner.code.eq("a' OR true --")).all(&tx).await?;
    assert!(none.is_empty());
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn related_ordering_runs_before_limit_and_keeps_missing_rows() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let query = Record::query()
        .order_by(Order::asc(Record::owner.label))
        .order_by(Order::asc(Record::id));
    let all: Vec<Record> = query.clone().all(&tx).await?;
    assert_eq!(all.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3, 2, 4, 7, 5, 6]);
    let page: Vec<Record<Option<Member>>> = query.with(Record::owner.joined()).limit(2).offset(1).all(&tx).await?;
    assert_eq!(page.iter().map(|row| row.id).collect::<Vec<_>>(), [3, 2]);
    assert_eq!(
        page.iter()
            .map(|row| row.owner.as_ref().unwrap().label.as_str())
            .collect::<Vec<_>>(),
        ["Atlas", "Birch"]
    );
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn automatic_left_join_for_update_returns_all_base_rows() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    // No owner filter: PostgreSQL must preserve the outer join and its missing-owner rows.
    let rows: Vec<Record> = Record::query()
        .order_by(Order::asc(Record::owner.id))
        .order_by(Order::asc(Record::id))
        .for_update()
        .all(&tx)
        .await?;

    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3, 2, 4, 7, 5, 6]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn count_exists_one_and_paginate_use_the_same_relation_filter() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let query = Record::query()
        .filter(Record::owner.enabled.eq(true))
        .with(Record::owner.joined())
        .order_by(Order::asc(Record::id));
    assert_eq!(query.clone().limit(1).offset(50).count(&tx).await?, 3);
    assert!(query.clone().limit(1).offset(50).exists(&tx).await?);
    let first = query.clone().one(&tx).await?.unwrap();
    assert_eq!((first.id, first.owner.unwrap().id), (1, 1));
    let page = query.paginate(2, 2, &tx).await?;
    assert_eq!(page.total, 3);
    assert_eq!(page.total_pages(), 2);
    assert_eq!(page.items.iter().map(|row| row.id).collect::<Vec<_>>(), [4]);
    assert_eq!(page.items[0].owner.as_ref().unwrap().id, 3);

    let empty = Record::query().filter(Record::owner.score.gt(100_i32)).with(Record::owner.joined());
    assert_eq!(empty.count(&tx).await?, 0);
    assert!(!empty.exists(&tx).await?);
    assert!(empty.clone().one(&tx).await?.is_none());
    assert!(empty.all(&tx).await?.is_empty());
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn nested_filters_and_both_nested_loading_strategies_agree() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let query = Record::query()
        .filter(Record::owner.organization.label.eq("north"))
        .order_by(Order::asc(Record::id));
    let joined: Vec<Record<Option<Member<Option<Organization>>>>> = query
        .clone()
        .with(Record::owner.joined().with(Member::organization.joined()))
        .all(&tx)
        .await?;
    let selectin: Vec<Record<Option<Member<Option<Organization>>>>> = query
        .with(Record::owner.selectin().with(Member::organization.selectin()))
        .all(&tx)
        .await?;
    for rows in [joined, selectin] {
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3]);
        assert!(rows
            .iter()
            .all(|row| row.owner.as_ref().unwrap().organization.as_ref().unwrap().label == "north"));
    }

    let missing: Vec<Record<Option<Member<Option<Organization>>>>> = Record::query()
        .filter(Record::owner.organization.id.is_null())
        .with(Record::owner.joined().with(Member::organization.joined()))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(missing.iter().map(|row| row.id).collect::<Vec<_>>(), [5, 6, 7]);
    assert!(missing[0].owner.is_none() && missing[1].owner.is_none());
    assert_eq!(missing[2].owner.as_ref().unwrap().id, 4);
    assert!(missing[2].owner.as_ref().unwrap().organization.is_none());
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn sibling_relations_filter_and_decode_independently_in_both_loaders() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let query = Assignment::query()
        .filter(Assignment::first.enabled.eq(true))
        .filter(Assignment::second.enabled.eq(false))
        .filter(Assignment::first.score.gt(Assignment::second.score))
        .order_by(Order::asc(Assignment::id));
    let joined: Vec<Assignment<Option<Member>, Option<Member>>> = query
        .clone()
        .with(Assignment::first.joined())
        .with(Assignment::second.joined())
        .all(&tx)
        .await?;
    let selectin: Vec<Assignment<Option<Member>, Option<Member>>> = query
        .with(Assignment::second.selectin())
        .with(Assignment::first.selectin())
        .all(&tx)
        .await?;
    for rows in [joined, selectin] {
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 7]);
        assert_eq!(rows.iter().map(|row| row.first.as_ref().unwrap().id).collect::<Vec<_>>(), [1, 3]);
        assert!(rows.iter().all(|row| row.second.as_ref().unwrap().id == 2));
    }
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn missing_or_equal_sibling_targets_do_not_overwrite_each_other() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let query = Assignment::query().order_by(Order::asc(Assignment::id));
    let joined: Vec<Assignment<Option<Member>, Option<Member>>> = query
        .clone()
        .with(Assignment::second.joined())
        .with(Assignment::first.joined())
        .all(&tx)
        .await?;
    let selectin: Vec<Assignment<Option<Member>, Option<Member>>> = query
        .with(Assignment::first.selectin())
        .with(Assignment::second.selectin())
        .all(&tx)
        .await?;
    for rows in [joined, selectin] {
        let actual: Vec<_> = rows
            .iter()
            .map(|row| (row.id, row.first.as_ref().map(|m| m.id), row.second.as_ref().map(|m| m.id)))
            .collect();
        assert_eq!(
            actual,
            [
                (1, Some(1), Some(2)),
                (2, Some(2), Some(1)),
                (3, Some(1), Some(1)),
                (4, None, Some(1)),
                (5, Some(1), None),
                (6, None, None),
                (7, Some(3), Some(2)),
                (8, Some(2), Some(2)),
            ]
        );
    }
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn nested_siblings_keep_their_own_organizations() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let rows: Vec<Assignment<Option<Member<Option<Organization>>>, Option<Member<Option<Organization>>>>> = Assignment::query()
        .filter(Assignment::first.organization.label.eq("north"))
        .filter(Assignment::second.organization.label.eq("south"))
        .with(Assignment::first.joined().with(Member::organization.joined()))
        .with(Assignment::second.joined().with(Member::organization.joined()))
        .all(&tx)
        .await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].first.as_ref().unwrap().organization.as_ref().unwrap().id, 1);
    assert_eq!(rows[0].second.as_ref().unwrap().organization.as_ref().unwrap().id, 2);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn self_relations_support_finite_depth_missing_parents_and_cycles() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let leaf: Node<Option<Node<Option<Node>>>> = Node::query()
        .filter(Node::parent.parent.label.eq("root"))
        .with(Node::parent.joined().with(Node::parent.joined()))
        .one(&tx)
        .await?
        .unwrap();
    assert_eq!(leaf.id, 3);
    let parent = leaf.parent.unwrap();
    assert_eq!(parent.id, 2);
    assert_eq!(parent.parent.unwrap().id, 1);

    let query = Node::query().order_by(Order::asc(Node::id));
    let joined: Vec<Node<Option<Node<Option<Node>>>>> = query
        .clone()
        .with(Node::parent.joined().with(Node::parent.joined()))
        .all(&tx)
        .await?;
    let selectin: Vec<Node<Option<Node<Option<Node>>>>> =
        query.with(Node::parent.selectin().with(Node::parent.selectin())).all(&tx).await?;
    for rows in [joined, selectin] {
        assert_eq!(rows.len(), 5);
        assert!(rows[0].parent.is_none());
        assert!(rows[3].parent.is_none());
        let cycle = rows[4].parent.as_ref().unwrap();
        assert_eq!(cycle.id, 5);
        assert_eq!(cycle.parent.as_ref().unwrap().id, 5);
    }
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn aliased_self_relation_filters_by_each_base_rows_parent() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let table = Node::TABLE.with_alias("n");
    let base_id = dbkit::Column::<Node, i64>::new(table, "id");
    let mut results = Vec::new();
    for label in ["branch", "cycle"] {
        let rows: Vec<Node> = dbkit::Select::new(table)
            .filter(Node::parent.label.eq(label))
            .order_by(Order::asc(base_id))
            .all(&tx)
            .await?;
        results.push((label, rows.iter().map(|row| row.id).collect::<Vec<_>>()));
    }

    // A parent-to-itself join loses the leaf and makes the cycle match every base row.
    assert_eq!(results, [("branch", vec![3]), ("cycle", vec![5])]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn explicit_inner_joins_and_existing_table_column_filters_still_work() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let inner: Vec<Record<Option<Member>>> = Record::query()
        .join(Record::owner)
        .with(Record::owner.joined())
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(inner.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4, 7]);

    let legacy: Vec<Record<Option<Member>>> = Record::query()
        .with(Record::owner.joined())
        .filter(Member::enabled.eq(true))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    let mixed: Vec<Record<Option<Member>>> = Record::query()
        .join(Record::owner)
        .filter(Member::enabled.eq(true))
        .filter(Record::owner.code.eq("a"))
        .with(Record::owner.joined())
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    assert_eq!(legacy.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3, 4]);
    assert_eq!(mixed.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3]);

    let custom: Vec<Record> = Record::query()
        .left_join_on(Member::TABLE, Member::id.eq_col(Record::owner_id).and(Member::score.gt(25_i32)))
        .filter(Member::note.is_null())
        .filter(Record::owner.code.eq("c"))
        .all(&tx)
        .await?;
    assert_eq!(custom.iter().map(|row| row.id).collect::<Vec<_>>(), [4]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn custom_join_can_use_a_relation_discovered_in_a_filter_in_either_builder_order() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let on = Organization::id
        .eq_col(Member::organization_id)
        .and(Organization::label.eq("north"));
    for query in [
        Record::query()
            .join_on(Organization::TABLE, on.clone())
            .filter(Record::owner.enabled.eq(true)),
        Record::query()
            .filter(Record::owner.enabled.eq(true))
            .join_on(Organization::TABLE, on),
    ] {
        let records: Vec<Record> = query.order_by(Order::asc(Record::id)).all(&tx).await?;
        // Keep both the custom organization condition and the related owner filter.
        assert_eq!(records.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 3]);
    }
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn custom_left_join_can_use_a_projection_relation_and_preserve_missing_targets() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let rows: Vec<(i64, Option<String>, Option<String>)> = Record::query()
        .left_join_on(Organization::TABLE, Organization::id.eq_col(Member::organization_id))
        .select_only()
        .column(Record::id)
        .column(Record::owner.label)
        .column(Organization::label)
        .order_by(Order::asc(Record::id))
        .into_model()
        .all(&tx)
        .await?;
    // Missing owners and an owner with a dangling organization must survive both LEFT JOINs.
    assert_eq!(
        rows,
        [
            (1, Some("Atlas".into()), Some("north".into())),
            (2, Some("Birch".into()), Some("south".into())),
            (3, Some("Atlas".into()), Some("north".into())),
            (4, Some("Cedar".into()), Some("south".into())),
            (5, None, None),
            (6, None, None),
            (7, Some("Delta".into()), None),
        ]
    );
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn custom_left_join_with_a_dynamic_relation_column_preserves_matches_and_missing_owners() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let enabled: dbkit::Expr<Option<bool>> = dbkit::Expr::new(dbkit::path::column(Member::enabled.as_ref(), &[Record::owner.descriptor()]));
    let records: Vec<Record> = Record::query()
        .left_join_on(Member::TABLE, enabled.eq(true))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    // An enabled owner matches all four rows of the custom join. Disabled,
    // NULL, and dangling owners survive once because this is a LEFT JOIN.
    assert_eq!(
        records.iter().map(|row| row.id).collect::<Vec<_>>(),
        [1, 1, 1, 1, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 6, 7]
    );
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn custom_inner_join_with_a_dynamic_relation_key_uses_the_requested_source_column() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let owner_id: dbkit::Expr<Option<i64>> = dbkit::Expr::new(dbkit::path::column(Member::id.as_ref(), &[Record::owner.descriptor()]));
    let records: Vec<Record> = Record::query()
        .join_on(Member::TABLE, owner_id.eq_col(Record::id))
        .order_by(Order::asc(Record::id))
        .all(&tx)
        .await?;
    // Only records 1 and 2 have owner.id == record.id; each matches all four members.
    assert_eq!(records.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 1, 1, 1, 2, 2, 2, 2]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn grouped_projections_and_correlated_exists_resolve_paths_in_their_query() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let counts: Vec<(Option<bool>, i64)> = Record::query()
        .select_only()
        .column(Record::owner.enabled)
        .column(func::count(Record::id))
        .group_by(Record::owner.enabled)
        .having(func::count(Record::id).gt(1_i64))
        .order_by(Order::asc(Record::owner.enabled))
        .into_model()
        .all(&tx)
        .await?;
    assert_eq!(counts, [(Some(false), 2), (Some(true), 3), (None, 2)]);

    let organizations: Vec<Organization> = Organization::query()
        .where_exists(
            Record::query()
                .select_only()
                .column(Record::id)
                .filter(Record::owner.organization_id.eq(Organization::id))
                .filter(Record::owner.score.gt(25_i32)),
        )
        .all(&tx)
        .await?;
    assert_eq!(organizations.iter().map(|row| row.id).collect::<Vec<_>>(), [1]);
    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn explicit_load_and_partial_unload_preserve_sibling_relation_identity() -> Result<(), Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let bare: Assignment = Assignment::query().filter(Assignment::id.eq(1_i64)).one(&tx).await?.unwrap();
    let second: Assignment<NotLoaded, Option<Member>> = bare.load(Assignment::second, &tx).await?;
    assert_eq!(second.second.as_ref().unwrap().id, 2);
    let both: Assignment<Option<Member>, Option<Member>> = second.load(Assignment::first, &tx).await?;
    assert_eq!(both.first.as_ref().unwrap().id, 1);
    assert_eq!(both.second.as_ref().unwrap().id, 2);
    let first_only: Assignment<Option<Member>> = both.clone().into();
    let second_only: Assignment<NotLoaded, Option<Member>> = both.into();
    assert_eq!(first_only.first.unwrap().id, 1);
    assert_eq!(second_only.second.unwrap().id, 2);
    tx.rollback().await?;
    Ok(())
}
