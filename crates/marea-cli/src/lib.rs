//! `marea new` — scaffolding for marea apps.
//!
//! Generates a workspace shaped `crates/{domain,data,ui,app-*}` from a set of
//! selected options. The layering it emits is the point: `domain` is pure
//! Rust, `data` owns every `#[cfg(target_arch)]` in the project, `ui` holds
//! one `Route` enum and one screen set for every target, and the `app-*`
//! crates are launchers. See `docs/scaffolding.md`.
//!
//! The crate is split so the parts that break silently are testable without
//! touching a filesystem:
//!
//! - [`options`] — what the wizard asks, what it derives, what it rejects.

pub mod options;

pub mod pins;

pub mod manifest;

pub mod locate;
pub mod lockfile;
pub mod prompts;
pub mod render;
pub mod verify;
pub mod write;
