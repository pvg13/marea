//! {{ title }} persistence.
//!
//! This crate is the seam between a screen and a database, and it is the
//! *only* place in the workspace allowed to know which target it is running
//! on. Natively the app owns a SQLite file and gossips with its other devices
//! peer-to-peer; in the browser it holds no database at all and talks to a
//! relay. Both converge on one `SyncHandle`, so the repositories above are
//! written once.
//!
//! ```text
//! entities/   the synced tables (#[derive(SyncEntity)])
//! repo/       one typed repository per aggregate — what screens call
//! bootstrap/  native.rs | web.rs — every cfg(target_arch) in the app
//! ```
//!
//! Screens never import `wavesyncdb`, never see a `SyncHandle`, and never
//! branch on the target. If a screen needs something this crate cannot do,
//! the fix is a method on a repository — not a `#[cfg]` in `ui`.

pub mod bootstrap;
pub mod entities;
pub mod repo;

/// The schema-registry name WaveSyncDB keys this crate's entities under.
///
/// It must be expanded **here**, in the crate that defines the entities:
/// `registry_name!` is `CARGO_PKG_NAME`, and calling it from a launcher would
/// register zero tables and fail silently at runtime.
pub const REGISTRY: &str = marea_sync::registry_name!();
