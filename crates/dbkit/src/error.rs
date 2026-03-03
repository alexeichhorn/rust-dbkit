use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[cfg(feature = "migrations")]
    #[error("migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("constraint violation: {constraint}")]
    ConstraintViolation { constraint: String },
    #[error("not found")]
    NotFound,
    #[error("relation mismatch")]
    RelationMismatch,
}
