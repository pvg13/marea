//! Feature slices.
//!
//! One directory per feature, holding everything that feature needs:
//!
//! ```text
//! features/<name>/
//!   mod.rs          re-exports the screen under its route name
//!   screen.rs       the routed screen
//!   components.rs   components only this feature uses
//!   state.rs        hooks and signals local to this feature
//!   i18n.rs         this feature's strings
//! ```
//!
//! Not `pages/` + `components/` + `data/` + one central `i18n.rs`. Those
//! split a feature across four directories, so every change touches all four
//! and the shared files grow without bound. Here, deleting a feature is
//! deleting a directory.

pub mod home;
pub mod settings;
{%- if with_example and wavesync %}
pub mod items;
{%- endif %}
