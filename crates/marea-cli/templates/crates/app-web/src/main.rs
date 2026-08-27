//! {{ title }} — browser launcher.
//!
//!     dx serve --package app-web --platform web
//!
//! The binary also compiles natively so `cargo check --workspace` stays green
//! on a machine with no wasm target installed; natively it does nothing.

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        // Panics reach the devtools console with a readable stack instead of
        // "unreachable executed".
        console_error_panic_hook::set_once();

        // Dioxus's own logger captures `tracing` events only. The sync engine
        // and several libraries log through `log`, so without this the
        // browser console stays silent about exactly the parts most likely to
        // be misbehaving.
        wasm_logger::init(wasm_logger::Config::default());

{%- if pairing %}
        dioxus::launch(WebApp);
{%- else %}
        dioxus::launch(ui::App);
{%- endif %}
    }

    #[cfg(not(target_arch = "wasm32"))]
    eprintln!(
        "app-web is a wasm binary; run it with `dx serve --package app-web --platform web`"
    );
}
{% if pairing %}
/// The browser's entry screen is the QR pairing flow rather than an
/// email/password form: the phone already holds the credentials.
///
/// This wrapper exists so `ui` never has to name `PairScreen`, which is a
/// wasm-only component — the target knowledge stays in the launcher, which is
/// the one crate allowed to have any.
#[cfg(target_arch = "wasm32")]
#[dioxus::prelude::component]
fn WebApp() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    rsx! {
        ui::AppWithLogin { login: rsx! { PairLogin {} } }
    }
}

/// Wraps `PairScreen` so it can read the active language.
///
/// The locale context is provided inside `AppWithLogin`, so a `use_lang()` in
/// *this* function would run above it and see nothing. As a component,
/// `PairLogin` mounts inside that subtree and reads it correctly.
#[cfg(target_arch = "wasm32")]
#[dioxus::prelude::component]
fn PairLogin() -> dioxus::prelude::Element {
    use dioxus::prelude::*;
    let lang = ui::i18n::use_lang();
    rsx! {
        marea_ui::pairing::PairScreen {
            config: &ui::app::PAIRING,
            brand: "{{ title }}".to_string(),
            strings: ui::i18n::pairing_strings(lang),
        }
    }
}
{% endif %}