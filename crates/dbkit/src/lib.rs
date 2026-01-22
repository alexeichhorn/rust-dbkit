pub use dbkit_core::*;
pub use dbkit_derive::{model, Model};
pub use sqlx;

pub mod database;
pub mod error;
pub mod executor;
pub mod func;
pub mod model;
pub mod query_ext;
pub mod runtime;
mod joined;

pub use database::{Database, DbTransaction};
pub use error::Error;
pub use executor::Executor;
pub use model::{GetRelation, JoinedModel, LoadRelation, ModelDelete, ModelValue, SetRelation};
pub use query_ext::{DeleteExt, InsertExt, Page, SelectExt, UpdateExt};

pub mod prelude {
    pub use crate::{DeleteExt, InsertExt, LoadRelation, ModelDelete, SelectExt, UpdateExt};
}
