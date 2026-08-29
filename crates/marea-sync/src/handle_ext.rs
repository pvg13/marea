//! Cross-target extensions to `wavesyncdb::dioxus::SyncHandle`.
//!
//! For apps that ship native **and** web builds (Mediterranea): pages call
//! `handle.submit_upsert(&model).await?` / `handle.delete_row::<E>(&pk)
//! .await?` without any cfg gates of their own; the gates live entirely in
//! this file. Both targets converge on the same wire-level behaviour — peers
//! see the same changesets / tombstones.
//!
//! - [`SyncHandleExt::submit_upsert`] — true upsert by primary key. The
//!   upstream `submit` is broken on native for any row whose PK was
//!   user-minted (all marea apps' `String`-UUID entities):
//!   `Model.into_active_model()` marks every field `Unchanged`, so
//!   `ActiveModelTrait::save` chooses the UPDATE branch, then runs an UPDATE
//!   with no `Set` columns → `RecordNotFound` on new rows and a silent no-op
//!   on existing ones. The native impl does an explicit `find_by_id` then
//!   `insert`/`update` with all values reset to `Set(...)`. On wasm the
//!   upstream `submit` is already a true upsert (`WebSyncClient::submit`
//!   sends the full column bag), so the wasm impl just forwards.
//! - [`SyncHandleExt::delete_row`] — delete by PK. Upstream has no delete
//!   primitive: native issues `delete_by_id` (the intercept layer broadcasts
//!   a `__deleted` tombstone); web calls `submit_local_write` with the
//!   documented tombstone sentinel.
//!
//! (Byte-compatible port of Mediterranea's `ui::sync_ext`.)

#[cfg(target_arch = "wasm32")]
use dioxus::prelude::ReadableExt;
use wavesyncdb::SyncedTableEntity;
use wavesyncdb::dioxus::{SyncHandle, SyncSubmitError};

/// String column-id used by the WaveSyncDB protocol to encode a row
/// deletion. Mirrors the constant the engine writes when it intercepts a
/// `DELETE FROM …` on native.
#[cfg(target_arch = "wasm32")]
const DELETED_SENTINEL: &str = "__deleted";

/// Cross-target write API on top of `SyncHandle`.
///
/// Constraint: marea apps' synced entities all carry `String` UUID primary
/// keys, so `&str` covers every call site. The trait definition is
/// target-split because native needs SeaORM-typed supertrait bounds that are
/// meaningless on wasm — both targets expose the same method signatures, so
/// callers don't see the divergence.
#[cfg(not(target_arch = "wasm32"))]
#[allow(async_fn_in_trait)]
pub trait SyncHandleExt {
    async fn delete_row<E>(&self, pk: &str) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity,
        <<E::Entity as ::sea_orm::EntityTrait>::PrimaryKey as ::sea_orm::PrimaryKeyTrait>::ValueType:
            From<String>;

    async fn submit_upsert<E>(&self, entity: &E) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity,
        <<E::Entity as ::sea_orm::EntityTrait>::PrimaryKey as ::sea_orm::PrimaryKeyTrait>::ValueType:
            ::sea_orm::sea_query::FromValueTuple;
}

#[cfg(target_arch = "wasm32")]
#[allow(async_fn_in_trait)]
pub trait SyncHandleExt {
    async fn delete_row<E>(&self, pk: &str) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity;

    async fn submit_upsert<E>(&self, entity: &E) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity;
}

#[cfg(not(target_arch = "wasm32"))]
impl SyncHandleExt for SyncHandle {
    async fn delete_row<E>(&self, pk: &str) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity,
        <<E::Entity as ::sea_orm::EntityTrait>::PrimaryKey as ::sea_orm::PrimaryKeyTrait>::ValueType:
            From<String>,
    {
        use ::sea_orm::EntityTrait;
        let typed_pk =
            <<E::Entity as EntityTrait>::PrimaryKey as ::sea_orm::PrimaryKeyTrait>::ValueType::from(
                pk.to_string(),
            );
        <E::Entity as EntityTrait>::delete_by_id(typed_pk)
            .exec(self.db())
            .await?;
        Ok(())
    }

    /// True upsert (find-then-insert-or-update) for entities with user-minted
    /// primary keys. The logic lives in the standalone [`upsert_by_pk`] so it
    /// can be unit-tested against a plain in-memory connection.
    async fn submit_upsert<E>(&self, entity: &E) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity,
        <<E::Entity as ::sea_orm::EntityTrait>::PrimaryKey as ::sea_orm::PrimaryKeyTrait>::ValueType:
            ::sea_orm::sea_query::FromValueTuple,
    {
        upsert_by_pk(self.db(), entity).await?;
        Ok(())
    }
}

/// Find-then-insert-or-update by primary key, with every column forced to
/// `Set`. Generic over the connection so it runs against both `WaveSyncDb`
/// (production — writes flow through the sync intercept) and a bare
/// `DatabaseConnection` (tests).
#[cfg(not(target_arch = "wasm32"))]
pub async fn upsert_by_pk<E, C>(conn: &C, entity: &E) -> Result<(), ::sea_orm::DbErr>
where
    C: ::sea_orm::ConnectionTrait,
    E: SyncedTableEntity,
    <<E::Entity as ::sea_orm::EntityTrait>::PrimaryKey as ::sea_orm::PrimaryKeyTrait>::ValueType:
        ::sea_orm::sea_query::FromValueTuple,
{
    use ::sea_orm::{ActiveModelTrait, EntityTrait};
    let am: E::ActiveModel = entity.clone().into_active_model();
    let am = am.reset_all();
    let pk_tuple = am
        .get_primary_key_value()
        .ok_or_else(|| ::sea_orm::DbErr::Custom("submit_upsert: missing primary key".into()))?;
    let pk_typed = <<<E::Entity as EntityTrait>::PrimaryKey as ::sea_orm::PrimaryKeyTrait>::ValueType as ::sea_orm::sea_query::FromValueTuple>::from_value_tuple(pk_tuple);
    let exists = <E::Entity as EntityTrait>::find_by_id(pk_typed)
        .one(conn)
        .await?
        .is_some();
    if exists {
        am.update(conn).await?;
    } else {
        am.insert(conn).await?;
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
impl SyncHandleExt for SyncHandle {
    async fn delete_row<E>(&self, pk: &str) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity,
    {
        // `client()` became `group()` in WaveSyncDB dbc5a36: the web engine
        // grew multiple groups per client, and a handle now names one of them.
        // `submit_local_write` is unchanged and stamps this group's topic.
        let signal = self.group();
        let Some(client) = signal.read().clone() else {
            return Err(SyncSubmitError::NotConnected);
        };
        // `submit_local_write` returns the new db_version which we discard —
        // the engine has already broadcast the tombstone changeset, and
        // `use_synced_table` will fold it via `subscribe_resolved`.
        let _new_db_version = client
            .submit_local_write(
                E::table_name(),
                pk,
                vec![(DELETED_SENTINEL.to_string(), serde_json::Value::Null)],
            )
            .await?;
        Ok(())
    }

    /// On wasm the upstream `submit` already serializes the full column bag
    /// and goes through `submit_local_write`, which is upsert-by-PK at the
    /// IndexedDB shadow-table layer. So we just forward.
    async fn submit_upsert<E>(&self, entity: &E) -> Result<(), SyncSubmitError>
    where
        E: SyncedTableEntity,
    {
        self.submit(entity).await
    }
}
