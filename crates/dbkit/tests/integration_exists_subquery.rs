#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Order};

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

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
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
