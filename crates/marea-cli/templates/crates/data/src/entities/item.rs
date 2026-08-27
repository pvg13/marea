//! DELETE ME — sample slice showing the marea layering.
//!
//! A synced table. Notice how it differs from `domain::Item`:
//!
//! - `created_at` is an RFC 3339 `String`, not a `chrono` type. Synced columns
//!   travel as JSON and must have the same representation on SQLite and in the
//!   browser, where there is no SeaORM to interpret a column type.
//! - Every field is public and unvalidated. Rows arrive from other devices;
//!   the invariants live in `domain`, and `repo` is where the two meet.
//!
//! # Rules for synced entities
//!
//! - **String UUID primary key, minted by the app.** WaveSyncDB merges per
//!   column, per row; an auto-increment key would collide across devices.
//! - **Additive changes only.** Adding a nullable column is safe (see
//!   `marea_sync::schema::ensure_columns`); renaming or dropping one orphans
//!   the data already on every install.

#[cfg(not(target_arch = "wasm32"))]
use sea_orm::entity::prelude::*;
use wavesyncdb::SyncEntity;

#[derive(Clone, Debug, PartialEq, SyncEntity, serde::Serialize, serde::Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(sea_orm::DeriveEntityModel))]
#[sea_orm(table_name = "item")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub title: String,
    pub done: bool,
    /// RFC 3339, UTC.
    pub created_at: String,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[cfg(not(target_arch = "wasm32"))]
impl ActiveModelBehavior for ActiveModel {}
