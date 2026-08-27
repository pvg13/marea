//! Browser sync bootstrap: no local database, a relay instead.
//!
//! The browser cannot open SQLite or dial peers directly, so it connects to
//! the relay over WebSocket and syncs through it. Everything above this file
//! is unaware: `native.rs` and this module both end by providing a
//! `SyncHandle`, and the repositories are written against that.

use dioxus::prelude::*;
use marea_auth::use_auth;
use marea_sync::wavesyncdb::dioxus::SyncHandle;
use marea_sync::wavesyncdb::WebSyncClient;

/// Must match `native.rs`'s topic — this is the same app.
const SYNC_TOPIC: &str = "{{ sync_topic }}";

/// IndexedDB store name. Scopes the persistent libp2p keypair and the shadow
/// tables, so a reload keeps the same peer identity.
const STORE_NAME: &str = "{{ name }}-web";

/// Relay the browser dials.
///
/// `wss://` works from a `dx serve` HTTP origin — browsers only block plain
/// `ws://` from an `https://` page — so this default is usable in development
/// too. Override at build time with `RELAY_WS=…`.
const RELAY: &str = match option_env!("RELAY_WS") {
    Some(v) => v,
    None => "{{ relay }}",
};

/// Escape hatch: `?relay=<multiaddr>` in the page URL points the tab at a
/// different relay, for iterating against one running locally.
fn relay_addr() -> String {
    let query = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .unwrap_or_default();
    for pair in query.trim_start_matches('?').split('&') {
        if let Some(value) = pair.strip_prefix("relay=")
            && !value.is_empty()
        {
            return value.to_string();
        }
    }
    RELAY.to_string()
}

#[component]
pub fn SyncProvider(children: Element) -> Element {
    let auth = use_auth();
    let mut client: Signal<Option<WebSyncClient>> = use_signal(|| None);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let session = (auth.session)();

    use_hook(move || {
        let Some(session) = session else {
            return;
        };
        spawn(async move {
            let addr = relay_addr();
            // The same passphrase the phone derived, so both land on the same
            // key and can actually read each other's changes.
            let psk = session.psk.clone();
            match WebSyncClient::connect_via_relay(&addr, SYNC_TOPIC, Some(&psk), STORE_NAME).await
            {
                Ok(c) => client.set(Some(c)),
                Err(e) => {
                    log::error!("relay connection failed: {e:?}");
                    error.set(Some(format!("{e:?}")));
                }
            }
        });
    });

    rsx! {
        if let Some(msg) = error() {
            div { class: "empty-state",
                div { class: "empty-state__title", "Could not reach the sync relay" }
                div { class: "empty-state__desc", "{msg}" }
            }
        } else if client().is_some() {
            SyncReady { handle: SyncHandle::new(client), {children} }
        } else {
            div { class: "gate-pending" }
        }
    }
}

#[component]
fn SyncReady(handle: SyncHandle, children: Element) -> Element {
    use_context_provider(|| handle);
    rsx! { {children} }
}
