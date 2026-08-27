//! Native sync bootstrap: a local SQLite database plus peer-to-peer gossip.
//!
//! Runs on desktop, Android and iOS. The browser half lives in `web.rs`; both
//! end by providing the same `SyncHandle`, which is what lets every
//! repository above be written once.

use dioxus::prelude::*;
use marea_auth::use_auth;
use marea_sync::wavesyncdb::dioxus::SyncHandle;
use marea_sync::{
    DbLocator, use_wavesync_generation, use_wavesync_init, use_wavesync_opt,
    use_wavesync_provider_lazy,
};

/// # Compatibility invariant — permanent once this app ships
///
/// These two strings are where every install's database lives. Change either
/// and existing users' data is orphaned on disk: the app comes up empty, with
/// the old file still sitting there.
pub const LOCATOR: DbLocator = DbLocator::new("{{ app_dir }}", "{{ db_file }}");

/// Gossipsub topic shared by every install of this app.
///
/// Per-user isolation does **not** come from the topic — it comes from the
/// passphrase below. Peers on the same topic with different passphrases
/// derive incompatible keys and silently ignore each other.
const SYNC_TOPIC: &str = "{{ sync_topic }}";
{% if relay %}
/// Relay used for WAN reachability and discovery. Override at build time with
/// `RELAY_QUIC=… cargo build` to point at a local relay.
const RELAY: &str = match option_env!("RELAY_QUIC") {
    Some(v) => v,
    None => "{{ relay }}",
};
{% endif %}
/// Brings up the database for the signed-in account and provides the
/// `SyncHandle` every repository reads.
///
/// Mounted inside marea's `AppShell`, which re-keys its authed subtree by
/// user id — so switching accounts remounts this component and opens the
/// other account's database rather than reusing the first one.
#[component]
pub fn SyncProvider(children: Element) -> Element {
    use_wavesync_provider_lazy();
    use_wavesync_generation();

    let auth = use_auth();
    let init = use_wavesync_init();
    let db = use_wavesync_opt();
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let session = (auth.session)();

    use_hook(move || {
        let Some(session) = session else {
            return;
        };
        spawn(async move {
            // Per-account database file. Two accounts on one device get two
            // files: the passphrase isolates them on the *network*, but a
            // single shared file would still physically hold both.
            let url = LOCATOR.user_db_url(&session.user_id);
            let psk = session.psk.clone();

            let result = init
                .call_with(
                    &url,
                    SYNC_TOPIC,
                    move |b| {
                        let b = b.with_passphrase(&psk);
                        {%- if relay %}
                        // WAN reachability and discovery: peers that cannot
                        // see each other over mDNS reserve a circuit through
                        // the relay, and find each other by registering in the
                        // same rendezvous namespace. One relay serves both
                        // roles, hence the same multiaddr twice.
                        let b = b.with_relay_server(RELAY).with_rendezvous_server(RELAY);
                        {%- endif %}
                        // iOS cannot do multicast without the
                        // `com.apple.developer.networking.multicast`
                        // entitlement, so mDNS there only logs "No route to
                        // host" on every interface and never finds a peer.
                        #[cfg(target_os = "ios")]
                        let b = b.with_mdns_enabled(false);
                        b
                    },
                    |db| async move {
                        // Creates the tables and the sync shadow tables for
                        // everything registered under this crate's name.
                        // Registering under any other name silently creates
                        // nothing — hence `crate::REGISTRY`.
                        db.get_schema_registry(crate::REGISTRY).sync().await?;
                        Ok(())
                    },
                )
                .await;

            if let Err(e) = result {
                log::error!("sync bootstrap failed: {e}");
                error.set(Some(e.to_string()));
            }
        });
    });

    rsx! {
        if let Some(msg) = error() {
            // Plain markup over marea.css classes rather than marea-ui
            // components: `data` is the persistence layer and has no business
            // depending on the component library.
            div { class: "empty-state",
                div { class: "empty-state__title", "Could not open the local database" }
                div { class: "empty-state__desc", "{msg}" }
            }
        } else if let Some(db) = db() {
            SyncReady { handle: SyncHandle::new(db), {children} }
        } else {
            div { class: "gate-pending" }
        }
    }
}

/// Publishes the handle. Separate so the context is provided by a component
/// that only ever renders once the database exists.
#[component]
fn SyncReady(handle: SyncHandle, children: Element) -> Element {
    use_context_provider(|| handle.clone());
    rsx! { {children} }
}
