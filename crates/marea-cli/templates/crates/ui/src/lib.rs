//! {{ title }} screens, routes and shell.
//!
//! One `Route` enum and one screen set for every target. marea's `NavShell`
//! switches between a sidebar (≥768px) and bottom tabs on its own, so a phone
//! and a desktop window differ by CSS and by responsive branches inside a
//! screen — never by a second router or a second view layer.
//!
//! Two rules keep this crate from becoming the place where everything lands:
//!
//! 1. A component lives in its feature until a *second* feature needs it,
//!    then it moves to `components/`.
//! 2. No `#[cfg(target_arch)]` anywhere under `src/`. Screens reach data
//!    through `data`'s repositories, which are the same API natively and in
//!    the browser. CI asserts this with a grep.

pub mod app;
pub mod components;
pub mod features;
pub mod i18n;
pub mod routes;
pub mod shell;

pub use app::App;
{%- if pairing %}
pub use app::{AppWithLogin, PAIRING};
{%- endif %}
pub use routes::Route;
