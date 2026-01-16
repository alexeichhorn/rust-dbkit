use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::Error;

#[derive(Debug, Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    pub async fn connect(url: &str) -> Result<Self, Error> {
        let pool = PgPoolOptions::new().connect(url).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn begin(&self) -> Result<sqlx::Transaction<'_, sqlx::Postgres>, Error> {
        Ok(self.pool.begin().await?)
    }
}
