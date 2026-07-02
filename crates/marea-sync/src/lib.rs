//! WaveSyncDB bootstrap conventions shared by marea apps.
//!
//! - [`DbLocator`]: per-app, per-account SQLite locations
//!   (`{data_dir}/{App}/u/{user_id}/{app}.db?mode=rwc`).
//! - [`registry_name!`]: schema-registry name derived **in the entity crate**,
//!   making the classic silently-creates-zero-tables mistake impossible.
//! - [`schema`]: idempotent, PRAGMA-guarded column additions for forward-only
//!   schema evolution (never destructive).
//! - [`SyncDbExt`]: safe CRDT write facade on [`wavesyncdb::WaveSyncDb`]
//!   (native) — full-row `INSERT … ON CONFLICT(pk) DO UPDATE` so every column
//!   gets a per-column clock.
//! - [`SyncHandleExt`]: cross-target write facade on
//!   `wavesyncdb::dioxus::SyncHandle` (native SeaORM / wasm `WebSyncClient`),
//!   for apps that also ship a web build.
//!
//! Apps must consume `wavesyncdb` **through this crate's re-export**
//! (`marea_sync::wavesyncdb`) rather than declaring their own git dependency —
//! two resolutions of the same git branch produce distinct types and context
//! lookups fail at runtime.

pub use wavesyncdb;

// The hooks apps actually wire into their shell, re-exported so app code
// doesn't need to name the wavesyncdb::dioxus path (and so grep finds one
// bootstrap vocabulary across apps). Native-only: they manage the native
// `WaveSyncDb`; the web build talks to the relay via `SyncHandle` instead.
#[cfg(not(target_arch = "wasm32"))]
pub use wavesyncdb::dioxus::{
    use_wavesync, use_wavesync_generation, use_wavesync_init, use_wavesync_opt,
    use_wavesync_provider, use_wavesync_provider_lazy,
};

mod handle_ext;
pub use handle_ext::SyncHandleExt;

/// Test-only access to the native upsert primitive behind
/// [`SyncHandleExt::submit_upsert`], so integration tests can exercise it
/// against a plain `DatabaseConnection` without a `SyncHandle`.
#[cfg(not(target_arch = "wasm32"))]
pub mod handle_ext_test_support {
    pub use crate::handle_ext::upsert_by_pk;
}

#[cfg(not(target_arch = "wasm32"))]
mod locator;
#[cfg(not(target_arch = "wasm32"))]
pub use locator::{DbLocator, scoped_topic};

#[cfg(not(target_arch = "wasm32"))]
pub mod schema;

#[cfg(not(target_arch = "wasm32"))]
mod db_ext;
#[cfg(not(target_arch = "wasm32"))]
pub use db_ext::{SyncDbExt, delete_by_pk, upsert_all_columns};

/// The schema-registry name for the crate this macro is expanded in.
///
/// WaveSyncDB's `get_schema_registry(scope)` keys registered entities by the
/// name of the crate that **defines** them. Calling
/// `env!("CARGO_PKG_NAME")` from a launcher crate silently creates zero
/// tables — a classic footgun in every app so far. Instead, the entity crate
/// exports its own name once:
///
/// ```rust,ignore
/// // in the crate that defines the #[derive(SyncEntity)] entities:
/// pub const REGISTRY: &str = marea_sync::registry_name!();
///
/// // in the launcher / shell:
/// db.get_schema_registry(entities::REGISTRY).sync().await?;
/// ```
#[macro_export]
macro_rules! registry_name {
    () => {
        env!("CARGO_PKG_NAME")
    };
}
