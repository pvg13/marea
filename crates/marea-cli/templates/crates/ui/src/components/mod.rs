//! Components shared by **two or more** features.
//!
//! The rule that keeps this file from becoming a 1000-line grab bag: a
//! component starts life inside the feature that needs it
//! (`features/<name>/components.rs`) and only moves here when a second
//! feature needs it too. Moving it is a two-line change; splitting an
//! overgrown shared module later is not.
//!
//! Anything generic enough to be themable — buttons, cards, rows, pills,
//! empty states — is already in `marea_ui`. Reach for that first.

use dioxus::prelude::*;
use marea_ui::{Button, ButtonVariant, ThemeMode, use_theme};

/// A 24×24 stroke icon from a single SVG path.
///
/// `stroke: currentColor` is the theme contract: icons inherit the text
/// colour of wherever they are placed, so they recolour with the theme
/// instead of needing a palette of their own.
pub fn icon(path: &'static str) -> Element {
    rsx! {
        svg {
            width: "22",
            height: "22",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "{path}" }
        }
    }
}

#[component]
pub fn ThemeToggle() -> Element {
    let mut theme = use_theme();
    let label = crate::i18n::theme_toggle(
        crate::i18n::use_lang(),
        matches!(theme.mode(), ThemeMode::Dark),
    );
    rsx! {
        Button {
            variant: ButtonVariant::Ghost,
            small: true,
            onclick: move |_| theme.toggle(),
            "{label}"
        }
    }
}
{% if auth %}
/// `block` makes it full-width for a settings card; the sidebar footer wants
/// the compact form.
///
/// The hook is read at the top of the component, never inside the handler —
/// Dioxus requires hooks to run unconditionally on every render, so calling
/// one from a closure is a panic waiting for the first click.
#[component]
pub fn SignOut(#[props(default)] block: bool) -> Element {
    let auth = marea_auth::use_auth();
    let label = crate::i18n::sign_out(crate::i18n::use_lang());
    rsx! {
        Button {
            variant: ButtonVariant::Outline,
            small: !block,
            block,
            onclick: move |_| auth.logout(),
            "{label}"
        }
    }
}
{% endif %}
