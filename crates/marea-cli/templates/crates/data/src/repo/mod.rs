//! Repositories — the API screens actually call.
//!
//! One module per aggregate, each exposing a `use_<thing>()` hook that
//! returns live reads plus the writes that make sense for it. Screens get
//! `use_items()`, not a `SyncHandle`; they never name a table, a column or a
//! target.
//!
//! This is also the layer to fake in tests and previews: a repository is a
//! small, honest interface, whereas a sync engine is not.

use marea_sync::wavesyncdb::dioxus::SyncSubmitError;
{% if with_example %}
pub mod items;

pub use items::use_items;
{% else %}
// Your repositories go here.
{% endif %}
/// What can go wrong on a write.
///
/// Deliberately small: a failed write is either something the user can retry
/// or a bug, and screens should not be pattern-matching on engine internals.
#[derive(Debug, thiserror::Error)]
pub enum RepoError {
    #[error("could not save: {0}")]
    Write(#[from] SyncSubmitError),
}
