//! Push-token context (FCM / APNs), persisted across launches.
//!
//! Port of roommates' `PushTokenContext`: the platform injects the token
//! (JNI on Android via WaveSyncDB's push-sync, env/FFI on iOS) and app code
//! reads it wherever registrations happen. Persisted shape is unchanged
//! (`platform` as the string "Fcm"/"Apns" under the "push_token" key) so
//! migrated installs keep their token.

use dioxus::prelude::*;
use dioxus_sdk_storage::{LocalStorage, use_synced_storage};
use serde::{Deserialize, Serialize};

/// Storage key roommates already persists under — do not change.
const PUSH_TOKEN_KEY: &str = "push_token";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PushToken {
    /// "Fcm" (Android) or "Apns" (iOS).
    pub platform: String,
    pub token: String,
}

/// Provide the persisted push-token signal. Called by
/// [`AppShell`](crate::shell::AppShell).
pub fn provide_push_token() -> Signal<Option<PushToken>> {
    let sig =
        use_synced_storage::<LocalStorage, Option<PushToken>>(PUSH_TOKEN_KEY.to_string(), || None);
    use_context_provider(|| sig)
}

pub fn use_push_token() -> Signal<Option<PushToken>> {
    use_context::<Signal<Option<PushToken>>>()
}

/// Store a fresh token (e.g. from the platform injector at startup).
pub fn set_push_token(mut sig: Signal<Option<PushToken>>, platform: &str, token: &str) {
    sig.set(Some(PushToken {
        platform: platform.to_string(),
        token: token.to_string(),
    }));
}
