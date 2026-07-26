//! The Dioxus UI layer for marea apps.
//!
//! Everything is styled by the bundled [`MAREA_CSS`] stylesheet — plain CSS
//! over the app-supplied `--c-*` design tokens (`docs/theme-contract.md`).
//! Apps reskin components entirely through their token file; marea-ui rsx
//! never uses Tailwind utility classes, so no cross-crate CSS scanning is
//! needed.

pub mod auth_screens;
pub mod components;
pub mod i18n;
pub mod nav;
pub mod push;
pub mod shell;
pub mod theme;
pub mod toast;

/// Camera code scanner (`scanner` feature). Off by default — it pulls a
/// vendored decoder on iOS and asks for camera permission, neither of which
/// an app that doesn't scan should carry.
#[cfg(feature = "scanner")]
pub mod scanner;

/// Phone→web QR pairing screens (`pairing` feature), over
/// [`marea_auth::pairing`]'s sealed-box mailbox handoff.
#[cfg(feature = "pairing")]
pub mod pairing;

/// Android back gesture → router (`android-back` feature). Inert on every
/// other target, but opt-in because it exports a fixed JNI symbol.
#[cfg(feature = "android-back")]
pub mod back_gesture;

pub use auth_screens::LoginScreen;
pub use components::*;
pub use i18n::{Locale, LocaleCtl, provide_locale, use_locale};
pub use nav::{NavItem, NavShell};
pub use push::{PushToken, provide_push_token, set_push_token, use_push_token};
pub use shell::AppShell;
pub use theme::{
    MAREA_CSS, THEME_STORAGE_KEY, ThemeCtl, ThemeMode, provide_theme, use_restore_theme, use_theme,
};
pub use toast::{Toast, ToastHost, ToastKind, Toasts, provide_toasts, use_toasts};
