pub use dbkit_core::interval;
pub use dbkit_core::*;
pub use dbkit_derive::{model, DbEnum, Model};
pub use sqlx::postgres::PgPoolOptions;
pub use sqlx;
#[cfg(feature = "migrations")]
pub use sqlx::migrate;

pub mod database;
pub mod error;
pub mod executor;
pub mod func;
mod joined;
pub mod model;
pub mod query_ext;
pub mod runtime;

pub use database::{Database, DbTransaction};
pub use error::Error;
pub use executor::Executor;
pub use model::{GetRelation, JoinedModel, LoadRelation, ModelDelete, ModelValue, SetRelation};
pub use query_ext::{DeleteExt, InsertExt, Page, SelectExt, UpdateExt};

pub mod prelude {
    pub use crate::{DeleteExt, InsertExt, LoadRelation, ModelDelete, SelectExt, UpdateExt};
}
