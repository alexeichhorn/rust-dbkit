#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Order};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

#[model(table = "accounts_exists")]
pub struct AccountExists {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub name: String,
    pub tier: String,
}

#[model(table = "subscriptions_exists")]
pub struct SubscriptionExists {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub account_id: i64,
    pub category: String,
    pub state: String,
}

#[model(table = "account_exists_lock_groups")]
pub struct AccountExistsLockGroup {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub token: Uuid,
    pub name: String,
}

#[model(table = "account_exists_lock_rows")]
pub struct AccountExistsLockRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub token: Uuid,
    pub group_id: i64,
    pub state: String,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

fn unique_lock_token() -> Uuid {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    let pid = std::process::id() as u128;
    Uuid::from_u128((pid << 64) | seq)
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    let statements = [
        "CREATE TEMP TABLE accounts_exists (\
            id BIGSERIAL PRIMARY KEY,\
            name TEXT NOT NULL,\
            tier TEXT NOT NULL\
        )",
        "CREATE TEMP TABLE subscriptions_exists (\
            id BIGSERIAL PRIMARY KEY,\
            account_id BIGINT NOT NULL,\
            category TEXT NOT NULL,\
            state TEXT NOT NULL\
        )",
    ];

    for statement in statements {
        ex.execute(statement, PgArguments::default()).await?;
    }

    Ok(())
}

async fn setup_locking_schema(db: &Database) -> Result<(), dbkit::Error> {
    let tx = db.begin().await?;
    tx.execute("SELECT pg_advisory_xact_lock(816726, 2)", PgArguments::default())
        .await?;
    tx.execute(
        "CREATE TABLE IF NOT EXISTS account_exists_lock_groups (\
            id BIGSERIAL PRIMARY KEY,\
            token UUID NOT NULL,\
            name TEXT NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_account_exists_lock_groups_token \
         ON account_exists_lock_groups(token)",
        PgArguments::default(),
    )
    .await?;
    tx.execute(
        "CREATE TABLE IF NOT EXISTS account_exists_lock_rows (\
            id BIGSERIAL PRIMARY KEY,\
            token UUID NOT NULL,\
            group_id BIGINT NOT NULL,\
            state TEXT NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    tx.execute(
        "CREATE INDEX IF NOT EXISTS idx_account_exists_lock_rows_token \
         ON account_exists_lock_rows(token)",
        PgArguments::default(),
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn seed_account<E: Executor + Send + Sync>(ex: &E, name: &str, tier: &str) -> Result<AccountExists, dbkit::Error> {
    let account = AccountExists::insert(AccountExistsInsert {
        name: name.to_string(),
        tier: tier.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted account");
    Ok(account)
}

async fn seed_lock_group<E: Executor + Send + Sync>(ex: &E, token: Uuid, name: &str) -> Result<AccountExistsLockGroup, dbkit::Error> {
    let row = AccountExistsLockGroup::insert(AccountExistsLockGroupInsert {
        token,
        name: name.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted lock group");
    Ok(row)
}

async fn seed_lock_row<E: Executor + Send + Sync>(
    ex: &E,
    token: Uuid,
    group_id: i64,
    state: &str,
) -> Result<AccountExistsLockRow, dbkit::Error> {
    let row = AccountExistsLockRow::insert(AccountExistsLockRowInsert {
        token,
        group_id,
        state: state.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted lock row");
    Ok(row)
}

async fn cleanup_lock_rows<E: Executor + Send + Sync>(ex: &E, token: Uuid) -> Result<(), dbkit::Error> {
    AccountExistsLockRow::delete()
        .filter(AccountExistsLockRow::token.eq(token))
        .execute(ex)
        .await?;
    AccountExistsLockGroup::delete()
        .filter(AccountExistsLockGroup::token.eq(token))
        .execute(ex)
        .await?;
    Ok(())
}

#[tokio::test]
async fn where_exists_returns_rows_with_matching_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let accounts = vec![
        seed_account(&tx, "Atlas", "pro").await?,
        seed_account(&tx, "Birch", "pro").await?,
        seed_account(&tx, "Cinder", "free").await?,
    ];

    SubscriptionExists::insert_many(vec![
        SubscriptionExistsInsert {
            account_id: accounts[0].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[1].id,
            category: "alerts".to_string(),
            state: "disabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[2].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
    ])
    .execute(&tx)
    .await?;

    let rows: Vec<AccountExists> = AccountExists::query()
        .filter(AccountExists::tier.eq("pro"))
        .where_exists(
            SubscriptionExists::query()
                .select_only()
                .column(SubscriptionExists::id)
                .filter(SubscriptionExists::account_id.eq_col(AccountExists::id))
                .filter(SubscriptionExists::category.eq("alerts"))
                .filter(SubscriptionExists::state.eq("enabled")),
        )
        .order_by(Order::asc(AccountExists::id))
        .all(&tx)
        .await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Atlas");

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn where_not_exists_returns_rows_without_matching_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let accounts = vec![
        seed_account(&tx, "Atlas", "pro").await?,
        seed_account(&tx, "Birch", "pro").await?,
        seed_account(&tx, "Cinder", "pro").await?,
    ];

    SubscriptionExists::insert_many(vec![
        SubscriptionExistsInsert {
            account_id: accounts[0].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[1].id,
            category: "reports".to_string(),
            state: "enabled".to_string(),
        },
    ])
    .execute(&tx)
    .await?;

    let rows: Vec<AccountExists> = AccountExists::query()
        .filter(AccountExists::tier.eq("pro"))
        .where_not_exists(
            SubscriptionExists::query()
                .select_only()
                .column(SubscriptionExists::id)
                .filter(SubscriptionExists::account_id.eq_col(AccountExists::id))
                .filter(SubscriptionExists::category.eq("alerts"))
                .filter(SubscriptionExists::state.eq("enabled")),
        )
        .order_by(Order::asc(AccountExists::id))
        .all(&tx)
        .await?;

    let names: Vec<String> = rows.into_iter().map(|row| row.name).collect();
    assert_eq!(names, vec!["Birch".to_string(), "Cinder".to_string()]);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn delete_where_exists_removes_rows_with_matching_parent() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let accounts = vec![seed_account(&tx, "Atlas", "free").await?, seed_account(&tx, "Birch", "pro").await?];

    SubscriptionExists::insert_many(vec![
        SubscriptionExistsInsert {
            account_id: accounts[0].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[1].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[1].id,
            category: "reports".to_string(),
            state: "disabled".to_string(),
        },
    ])
    .execute(&tx)
    .await?;

    let deleted = SubscriptionExists::delete()
        .where_exists(
            AccountExists::query()
                .select_only()
                .column(AccountExists::id)
                .filter(AccountExists::id.eq_col(SubscriptionExists::account_id))
                .filter(AccountExists::tier.eq("free")),
        )
        .execute(&tx)
        .await?;

    assert_eq!(deleted, 1);

    let remaining: Vec<SubscriptionExists> = SubscriptionExists::query()
        .order_by(Order::asc(SubscriptionExists::id))
        .all(&tx)
        .await?;
    let remaining_accounts: Vec<i64> = remaining.into_iter().map(|row| row.account_id).collect();
    assert_eq!(remaining_accounts, vec![accounts[1].id, accounts[1].id]);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn delete_where_not_exists_removes_rows_without_matching_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let accounts = vec![
        seed_account(&tx, "Atlas", "pro").await?,
        seed_account(&tx, "Birch", "pro").await?,
        seed_account(&tx, "Cinder", "free").await?,
    ];

    SubscriptionExists::insert_many(vec![
        SubscriptionExistsInsert {
            account_id: accounts[0].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[1].id,
            category: "reports".to_string(),
            state: "enabled".to_string(),
        },
    ])
    .execute(&tx)
    .await?;

    let deleted = AccountExists::delete()
        .filter(AccountExists::tier.eq("pro"))
        .where_not_exists(
            SubscriptionExists::query()
                .select_only()
                .column(SubscriptionExists::id)
                .filter(SubscriptionExists::account_id.eq_col(AccountExists::id))
                .filter(SubscriptionExists::category.eq("alerts"))
                .filter(SubscriptionExists::state.eq("enabled")),
        )
        .execute(&tx)
        .await?;

    assert_eq!(deleted, 1);

    let remaining: Vec<AccountExists> = AccountExists::query().order_by(Order::asc(AccountExists::id)).all(&tx).await?;
    let remaining_names: Vec<String> = remaining.into_iter().map(|row| row.name).collect();
    assert_eq!(remaining_names, vec!["Atlas".to_string(), "Cinder".to_string()]);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn update_where_exists_mutates_rows_with_matching_parent() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let accounts = vec![seed_account(&tx, "Atlas", "free").await?, seed_account(&tx, "Birch", "pro").await?];

    SubscriptionExists::insert_many(vec![
        SubscriptionExistsInsert {
            account_id: accounts[0].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[1].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
    ])
    .execute(&tx)
    .await?;

    let updated: Vec<SubscriptionExists> = SubscriptionExists::update()
        .set(SubscriptionExists::state, "retired")
        .where_exists(
            AccountExists::query()
                .select_only()
                .column(AccountExists::id)
                .filter(AccountExists::id.eq_col(SubscriptionExists::account_id))
                .filter(AccountExists::tier.eq("free")),
        )
        .returning_all()
        .all(&tx)
        .await?;

    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].account_id, accounts[0].id);
    assert_eq!(updated[0].state, "retired");

    let rows: Vec<SubscriptionExists> = SubscriptionExists::query()
        .order_by(Order::asc(SubscriptionExists::id))
        .all(&tx)
        .await?;
    let states: Vec<String> = rows.into_iter().map(|row| row.state).collect();
    assert_eq!(states, vec!["retired".to_string(), "enabled".to_string()]);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn update_where_not_exists_mutates_rows_without_matching_children() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let accounts = vec![
        seed_account(&tx, "Atlas", "pro").await?,
        seed_account(&tx, "Birch", "pro").await?,
        seed_account(&tx, "Cinder", "free").await?,
    ];

    SubscriptionExists::insert_many(vec![
        SubscriptionExistsInsert {
            account_id: accounts[0].id,
            category: "alerts".to_string(),
            state: "enabled".to_string(),
        },
        SubscriptionExistsInsert {
            account_id: accounts[1].id,
            category: "reports".to_string(),
            state: "enabled".to_string(),
        },
    ])
    .execute(&tx)
    .await?;

    let updated: Vec<AccountExists> = AccountExists::update()
        .set(AccountExists::tier, "dormant")
        .filter(AccountExists::tier.eq("pro"))
        .where_not_exists(
            SubscriptionExists::query()
                .select_only()
                .column(SubscriptionExists::id)
                .filter(SubscriptionExists::account_id.eq_col(AccountExists::id))
                .filter(SubscriptionExists::category.eq("alerts"))
                .filter(SubscriptionExists::state.eq("enabled")),
        )
        .returning_all()
        .all(&tx)
        .await?;

    assert_eq!(updated.len(), 1);
    assert_eq!(updated[0].name, "Birch");
    assert_eq!(updated[0].tier, "dormant");

    let rows: Vec<AccountExists> = AccountExists::query().order_by(Order::asc(AccountExists::id)).all(&tx).await?;
    let tiers: Vec<String> = rows.into_iter().map(|row| row.tier).collect();
    assert_eq!(tiers, vec!["pro".to_string(), "dormant".to_string(), "free".to_string()]);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn where_exists_preserves_skip_locked_subquery_semantics() -> Result<(), dbkit::Error> {
    let db_a = Database::connect(&db_url()).await?;
    let db_b = Database::connect(&db_url()).await?;
    setup_locking_schema(&db_a).await?;

    let token = unique_lock_token();
    let group = seed_lock_group(&db_a, token, "alpha").await?;
    let row = seed_lock_row(&db_a, token, group.id, "ready").await?;

    let tx1 = db_a.begin().await?;
    let locked = AccountExistsLockRow::query()
        .filter(AccountExistsLockRow::id.eq(row.id))
        .for_update()
        .one(&tx1)
        .await?;
    assert!(locked.is_some(), "row should be locked by tx1");

    let tx2 = db_b.begin().await?;
    let rows = AccountExistsLockGroup::query()
        .filter(AccountExistsLockGroup::token.eq(token))
        .where_exists(
            AccountExistsLockRow::query()
                .select_only()
                .column(AccountExistsLockRow::id)
                .filter(AccountExistsLockRow::group_id.eq_col(AccountExistsLockGroup::id))
                .filter(AccountExistsLockRow::token.eq(token))
                .filter(AccountExistsLockRow::state.eq("ready"))
                .for_update()
                .skip_locked(),
        )
        .all(&tx2)
        .await?;

    assert!(
        rows.is_empty(),
        "skip locked subquery should not match rows locked by another transaction"
    );

    tx2.rollback().await?;
    tx1.rollback().await?;
    cleanup_lock_rows(&db_a, token).await?;
    Ok(())
}
