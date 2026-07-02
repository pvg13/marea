//! Safe CRDT write path on [`WaveSyncDb`] (native).
//!
//! WaveSyncDB records a per-column CRDT clock only for columns that appear in
//! the executed SQL. SeaORM's `ActiveModel::save()` emits only the columns
//! that were explicitly `Set`, so any column left `Unchanged`/`NotSet` (e.g.
//! an update built with `..Default::default()`) is never tracked and never
//! syncs to peers.
//!
//! [`SyncDbExt::submit_upsert`] takes a **full** model, marks every column
//! `Set` (`reset_all`), and runs an upsert (`INSERT … ON CONFLICT(pk) DO
//! UPDATE` over all columns) so the whole row lands in the SQL and gets
//! tracked. Use these for ALL writes; never call raw
//! `ActiveModel::insert/update`.
//!
//! (Byte-compatible port of Ascend's `data::sync_ext` — same SQL shape. Apps
//! that also ship a web build use the cross-target
//! [`SyncHandleExt`](crate::SyncHandleExt) instead.)

use sea_orm::sea_query::OnConflict;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, Iterable,
    PrimaryKeyToColumn, QueryFilter,
};
use wavesyncdb::{SyncedTableEntity, WaveSyncDb};

/// Extension methods for safe, fully-tracked CRDT writes. Implemented on
/// [`WaveSyncDb`] (which is itself a SeaORM `ConnectionTrait`).
#[allow(async_fn_in_trait)]
pub trait SyncDbExt {
    /// Insert or update `model`, forcing every column into the statement so
    /// all columns get per-column CRDT clocks.
    async fn submit_upsert<E: SyncedTableEntity>(&self, model: &E) -> Result<(), DbErr>;

    /// Delete the row of entity `E` whose primary key equals `pk`.
    async fn delete_row<E: SyncedTableEntity>(&self, pk: &str) -> Result<(), DbErr>;
}

impl SyncDbExt for WaveSyncDb {
    async fn submit_upsert<E: SyncedTableEntity>(&self, model: &E) -> Result<(), DbErr> {
        upsert_all_columns(self, model).await
    }

    async fn delete_row<E: SyncedTableEntity>(&self, pk: &str) -> Result<(), DbErr> {
        delete_by_pk::<_, E>(self, pk).await
    }
}

/// The upsert primitive behind [`SyncDbExt::submit_upsert`], generic over the
/// connection so it runs against both `WaveSyncDb` (production — writes flow
/// through the sync intercept) and a bare `DatabaseConnection` (tests).
pub async fn upsert_all_columns<C, E>(conn: &C, model: &E) -> Result<(), DbErr>
where
    C: ConnectionTrait,
    E: SyncedTableEntity,
{
    // reset_all: Unchanged -> Set on every column, so the upsert SQL carries
    // all columns and WaveSyncDB tracks a clock for each.
    let am = model.clone().into_active_model().reset_all();

    let pk_cols: Vec<_> = <<E::Entity as EntityTrait>::PrimaryKey as Iterable>::iter()
        .map(|k| k.into_column())
        .collect();
    // Update every column on conflict (re-setting the pk to itself is a no-op
    // but keeps the update list generic without needing `Column: PartialEq`).
    let all_cols: Vec<_> = <<E::Entity as EntityTrait>::Column as Iterable>::iter().collect();

    let mut on_conflict = OnConflict::columns(pk_cols);
    on_conflict.update_columns(all_cols);

    <E::Entity as EntityTrait>::insert(am)
        .on_conflict(on_conflict)
        .exec(conn)
        .await?;
    Ok(())
}

/// The delete primitive behind [`SyncDbExt::delete_row`]. WaveSyncDb's SeaORM
/// intercept layer parses the DELETE, generates a `__deleted` tombstone and
/// broadcasts it.
pub async fn delete_by_pk<C, E>(conn: &C, pk: &str) -> Result<(), DbErr>
where
    C: ConnectionTrait,
    E: SyncedTableEntity,
{
    let pk_col = <<E::Entity as EntityTrait>::PrimaryKey as Iterable>::iter()
        .next()
        .expect("entity has a primary key")
        .into_column();
    <E::Entity as EntityTrait>::delete_many()
        .filter(pk_col.eq(pk))
        .exec(conn)
        .await?;
    Ok(())
}
