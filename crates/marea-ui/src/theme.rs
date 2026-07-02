//! Theme mode (light / dark) over the CSS-token contract.
//!
//! All actual theming lives in the app's token stylesheet: `[data-theme=
//! "dark"]` re-declares whichever `--c-*` variables change (see
//! `docs/theme-contract.md`). This module only flips the attribute on
//! `<html>` and persists the choice to `localStorage`, mirroring what the
//! shell's pre-paint restore reads.

use dioxus::prelude::*;

/// The bundled component stylesheet. Mount once, before the app's own
/// stylesheet:
///
/// ```rust,ignore
/// document::Stylesheet { href: marea_ui::MAREA_CSS }
/// ```
pub const MAREA_CSS: Asset = asset!("/assets/marea.css");

/// Storage key shared by [`use_restore_theme`] and [`ThemeCtl::set`].
/// Matches what the apps already persist ("theme").
pub const THEME_STORAGE_KEY: &str = "theme";

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
}

impl ThemeMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    pub fn toggled(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
        }
    }
}

/// Re-apply the persisted theme **before first paint** so a dark-mode user
/// never sees a light flash. Call once at the shell root (AppShell does).
pub fn use_restore_theme() {
    use_hook(|| {
        document::eval(concat!(
            "(function(){",
            "var t=localStorage.getItem('theme');",
            "if(t)document.documentElement.setAttribute('data-theme',t);",
            "})()"
        ));
    });
}

/// `Copy` handle on the current theme mode. Obtain via [`use_theme`].
#[derive(Clone, Copy)]
pub struct ThemeCtl {
    mode: Signal<ThemeMode>,
}

impl ThemeCtl {
    pub fn mode(&self) -> ThemeMode {
        (self.mode)()
    }

    /// Set the mode: flips `data-theme` on `<html>` and persists it.
    pub fn set(&mut self, mode: ThemeMode) {
        self.mode.set(mode);
        let tag = mode.as_str();
        document::eval(&format!(
            "document.documentElement.setAttribute('data-theme','{tag}');\
             localStorage.setItem('theme','{tag}');"
        ));
    }

    pub fn toggle(&mut self) {
        self.set(self.mode().toggled());
    }
}

/// Provide the theme context. Called by AppShell; apps normally just call
/// [`use_theme`]. The initial signal value is `Light` — the pre-paint restore
/// handles the visual state, and the first `set()` re-syncs both.
pub fn provide_theme() -> ThemeCtl {
    let mode = use_signal(ThemeMode::default);
    use_context_provider(|| ThemeCtl { mode })
}

pub fn use_theme() -> ThemeCtl {
    use_context::<ThemeCtl>()
}
