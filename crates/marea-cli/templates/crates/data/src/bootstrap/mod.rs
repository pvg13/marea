//! Bringing sync up, per target.
//!
//! **This is the only `#[cfg(target_arch)]` fork in the app.** Both branches
//! export a `SyncProvider` component with the same props and the same
//! contract: mount it inside the authed subtree, and everything below it can
//! call `use_context::<SyncHandle>()` — or, better, the repositories in
//! `crate::repo`, which is what screens actually use.
//!
//! If you find yourself wanting a target check in `ui`, the capability is
//! missing here instead.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{LOCATOR, SyncProvider};

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::SyncProvider;
