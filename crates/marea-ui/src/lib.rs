//! The Dioxus UI layer for marea apps.
//!
//! Everything is styled by the bundled [`MAREA_CSS`] stylesheet — plain CSS
//! over the app-supplied `--c-*` design tokens (`docs/theme-contract.md`).
//! Apps reskin components entirely through their token file; marea-ui rsx
//! never uses Tailwind utility classes, so no cross-crate CSS scanning is
//! needed.

pub mod components;
pub mod theme;
pub mod toast;

pub use components::*;
pub use theme::{
    MAREA_CSS, THEME_STORAGE_KEY, ThemeCtl, ThemeMode, provide_theme, use_restore_theme, use_theme,
};
pub use toast::{Toast, ToastHost, ToastKind, Toasts, provide_toasts, use_toasts};
