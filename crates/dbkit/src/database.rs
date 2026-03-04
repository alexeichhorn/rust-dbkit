use std::sync::Arc;

use sqlx::postgres::{PgPool, PgPoolOptions};
use tokio::sync::Mutex;

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

    pub async fn connect_with_max_connections(url: &str, max_connections: u32) -> Result<Self, Error> {
        if max_connections == 0 {
            return Err(Error::Decode("max_connections must be greater than 0".to_string()));
        }

        let pool = PgPoolOptions::new().max_connections(max_connections).connect(url).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn begin(&self) -> Result<DbTransaction<'_>, Error> {
        let tx = self.pool.begin().await?;
        Ok(DbTransaction::new(tx))
    }

    #[cfg(feature = "migrations")]
    pub async fn migrate(&self, migrator: &sqlx::migrate::Migrator) -> Result<(), Error> {
        migrator.run(&self.pool).await?;
        Ok(())
    }
}

pub struct DbTransaction<'t> {
    pub(crate) inner: Arc<Mutex<Option<sqlx::Transaction<'t, sqlx::Postgres>>>>,
}

impl<'t> DbTransaction<'t> {
    pub(crate) fn new(tx: sqlx::Transaction<'t, sqlx::Postgres>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Some(tx))),
        }
    }

    pub async fn commit(self) -> Result<(), Error> {
        let tx = {
            let mut guard = self.inner.lock().await;
            guard
                .take()
                .ok_or_else(|| Error::Decode("transaction already completed".to_string()))?
        };
        tx.commit().await?;
        Ok(())
    }

    pub async fn rollback(self) -> Result<(), Error> {
        let tx = {
            let mut guard = self.inner.lock().await;
            guard
                .take()
                .ok_or_else(|| Error::Decode("transaction already completed".to_string()))?
        };
        tx.rollback().await?;
        Ok(())
    }
}
