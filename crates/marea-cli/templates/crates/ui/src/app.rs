//! The root component every target launches.
//!
//! The platform binaries (`app-desktop`, `app-mobile`, `app-web`) do nothing
//! but set the storage directory and call `dioxus::launch(ui::App)`. All the
//! composition happens here.

use dioxus::prelude::*;
{%- if auth %}
use marea_auth::AuthConfig;
use marea_ui::{AppShell, MAREA_CSS};
{%- else %}
use marea_ui::MAREA_CSS;
{%- endif %}

use crate::routes::Route;
{% if pairing %}
/// Phone → browser login handoff.
///
/// The phone holds the credentials; the browser shows a QR, and the phone
/// seals the session to the ephemeral public key inside it. `url_scheme` is
/// what the QR encodes, so it must match the deep link the mobile app
/// registers.
pub static PAIRING: marea_auth::PairingConfig = marea_auth::PairingConfig {
    url_scheme: "{{ name }}",
    // Plural — this is the collection name marea's pairing client expects and
    // the one the shared PocketBase actually has. Getting it wrong fails at
    // runtime with "Missing or invalid collection context", which says nothing
    // about collection names.
    mailbox_collection: "pairing_mailboxes",
    ttl_secs: 300,
};
{% endif %}

/// Your skin. Every colour in marea's stylesheet is a `--c-*` token this file
/// defines — see marea's `docs/theme-contract.md`.
const TOKENS_CSS: Asset = asset!("/assets/tokens.css");
/// This app's own layout and chrome, built on the same tokens.
const APP_CSS: Asset = asset!("/assets/app.css");
{%- if tailwind %}
/// Tailwind output for this app's own screens. marea's components never use
/// utility classes, so nothing here changes how they render.
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
{%- endif %}
{% if auth %}
/// # Compatibility invariants — permanent once this app ships
///
/// `psk_domain` salts the Argon2id derivation of the sync passphrase, and
/// `session_storage_key` names where the session is persisted. Change either
/// after the first install and every existing user loses their synced data
/// and their login. They are derived from the project name once, at
/// generation time, and are not knobs.
static AUTH: AuthConfig = AuthConfig {
    pocketbase_url: "{{ pocketbase_url }}",
    psk_domain: b"{{ psk_domain }}",
    session_storage_key: "{{ session_storage_key }}",
};

/// What every launcher calls. `dioxus::launch` needs a component with no
/// required props, so the injectable form lives in [`AppWithLogin`].
#[component]
pub fn App() -> Element {
    rsx! {
        AppWithLogin {
            login: rsx! {
                marea_ui::LoginScreen { title: "{{ title }}" }
            },
        }
    }
}

/// The real root, with the pre-login screen injected.
///
/// A prop rather than a target check: {% if pairing %}the browser signs in by scanning a QR
/// from the phone, and marea's `PairScreen` exists only on wasm. Letting
/// `app-web` pass it in keeps this crate free of `#[cfg]`{% else %}if a target ever needs a
/// different entry screen it passes one in, rather than adding a `#[cfg]`
/// here{% endif %}.
#[component]
pub fn AppWithLogin(login: Element) -> Element {
    marea_ui::provide_locale::<crate::i18n::Lang>(crate::i18n::LANG_STORAGE_KEY);

    rsx! {
        document::Stylesheet { href: MAREA_CSS }
        document::Stylesheet { href: TOKENS_CSS }
        document::Stylesheet { href: APP_CSS }
        {%- if tailwind %}
        document::Stylesheet { href: TAILWIND_CSS }
        {%- endif %}

        // AppShell restores the theme, provides toasts and the push-token
        // context, gates on the session, and re-keys the authed subtree by
        // user id so per-account resources reinitialise on account switch.
        AppShell {
            auth: &AUTH,
            login,
            {%- if wavesync %}
            // The one place in the app where native and web diverge.
            data::bootstrap::SyncProvider {
                Router::<Route> {}
            }
            {%- else %}
            Router::<Route> {}
            {%- endif %}
        }
    }
}
{% else %}
#[component]
pub fn App() -> Element {
    // No auth: do by hand the parts of marea's `AppShell` that are not the
    // session gate. Add `marea-auth` and swap this for `AppShell` if this app
    // ever grows accounts.
    marea_ui::use_restore_theme();
    marea_ui::provide_theme();
    marea_ui::provide_toasts();
    marea_ui::provide_locale::<crate::i18n::Lang>(crate::i18n::LANG_STORAGE_KEY);

    rsx! {
        document::Stylesheet { href: MAREA_CSS }
        document::Stylesheet { href: TOKENS_CSS }
        document::Stylesheet { href: APP_CSS }
        {%- if tailwind %}
        document::Stylesheet { href: TAILWIND_CSS }
        {%- endif %}

        {%- if wavesync %}
        data::bootstrap::SyncProvider {
            Router::<Route> {}
        }
        {%- else %}
        Router::<Route> {}
        {%- endif %}
        marea_ui::ToastHost {}
    }
}
{% endif %}
