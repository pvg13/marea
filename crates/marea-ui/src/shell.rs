//! The app root: providers, auth gate, per-user remount.
//!
//! [`AppShell`] generalizes the root component every marea app used to
//! hand-roll (Mediterranea's `AppShell`, Ascend's `App`, roommates'
//! `AppShell`). In order it:
//!
//! 1. restores the persisted theme **before first paint**,
//! 2. provides theme, persistent auth session/state, push-token and toast
//!    contexts,
//! 3. renders `children` (the app's router) when authenticated — **keyed by
//!    `user_id`**, so switching accounts remounts the whole authed subtree
//!    and per-user resources (WaveSyncDB, storage) re-initialize against the
//!    new user — or the login screen otherwise,
//! 4. mounts the [`ToastHost`](crate::toast::ToastHost).
//!
//! App-specific contexts (Ascend's `CompletionOutcome`, Mediterranea's
//! catalog signals) are provided by the app in its own component *above*
//! `AppShell` — context flows down, so no provider slot is needed. Sync
//! bootstrapping (WaveSyncDB lazy init) lives at the top of the app's
//! `children`, where the app controls registry/reconcile/seed.
//!
//! ```rust,ignore
//! static AUTH: AuthConfig = AuthConfig { /* … */ };
//!
//! #[component]
//! fn App() -> Element {
//!     use_context_provider(|| Signal::new(None::<MyAppContext>));
//!     rsx! {
//!         document::Stylesheet { href: marea_ui::MAREA_CSS }
//!         document::Stylesheet { href: asset!("/assets/tailwind.css") }
//!         AppShell {
//!             auth: &AUTH,
//!             phone_frame: true,          // Ascend-style 430px column
//!             AuthedApp {}                // the app's router + sync boot
//!         }
//!     }
//! }
//! ```

use dioxus::prelude::*;
use marea_auth::{AuthConfig, provide_auth_state, use_persistent_session};

use crate::push::provide_push_token;
use crate::theme::{provide_theme, use_restore_theme};
use crate::toast::{ToastHost, provide_toasts};

#[component]
pub fn AppShell(
    /// The app's static auth configuration (PocketBase URL, PSK domain,
    /// session storage key).
    auth: &'static AuthConfig,
    /// Authed content — the app's `Router` (plus any sync bootstrapping).
    children: Element,
    /// Override the pre-login screen. Defaults to
    /// [`LoginScreen`](crate::auth_screens::LoginScreen) with stock copy —
    /// apps usually pass their branded one.
    #[props(default)]
    login: Option<Element>,
    /// Render everything inside a centered 430px phone column
    /// (`.app-frame--phone`), Ascend's deliberate desktop presentation.
    #[props(default)]
    phone_frame: bool,
) -> Element {
    use_restore_theme();
    provide_theme();

    let session = use_persistent_session(auth);
    provide_auth_state(auth, session);
    provide_push_token();
    provide_toasts();

    let user_id = session().map(|s| s.user_id);

    let content = match user_id {
        // `key` remounts the subtree when the account changes, so per-user
        // resources (WaveSyncDB, per-account DBs) re-initialize.
        Some(uid) => rsx! {
            AuthedSubtree { key: "{uid}", {children} }
        },
        None => match login {
            Some(el) => el,
            None => rsx! {
                crate::auth_screens::LoginScreen {}
            },
        },
    };

    rsx! {
        if phone_frame {
            div { class: "app-frame--phone", {content} }
        } else {
            {content}
        }
        ToastHost {}
    }
}

/// Exists only to carry the per-user `key` — keyed replacement of this node
/// is what forces the authed subtree to remount on account switch.
#[component]
fn AuthedSubtree(children: Element) -> Element {
    rsx! {
        {children}
    }
}
