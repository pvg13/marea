//! PocketBase authentication for marea apps.
//!
//! One concrete client against the **stock** PocketBase `users` auth
//! collection — no server-side schema customisation. Each app provides a
//! static [`AuthConfig`] naming its PocketBase instance, its PSK
//! domain-separation tag and its session storage key; everything else is
//! shared:
//!
//! - [`PocketBase`]: login / register / refresh / change-password with
//!   structured, localizable field errors ([`PocketBaseError`]).
//! - [`AuthSession`]: what a device holds after login (`user_id`, `email`,
//!   `token`, `psk`). Serde-compatible with the sessions the apps already
//!   persist — field names must never change.
//! - [`derive_psk`]: client-side Argon2id derivation of the WaveSyncDB group
//!   passphrase from `(user_id, password)`. The server never sees it.
//! - JWT freshness helpers ([`token_expiry_secs`], [`is_near_expiry`]) and
//!   [`PocketBase::ensure_fresh`] for margin-based token refresh.
//! - `pairing` feature: WhatsApp-style phone→web login handoff (X25519
//!   sealed box through a PocketBase mailbox collection).
//! - `dioxus` feature: [`use_persistent_session`], [`AuthState`] context.

pub mod config;
pub mod error;
mod jwt;
pub mod pocketbase;
pub mod psk;
pub mod session;

#[cfg(feature = "pairing")]
pub mod pairing;

#[cfg(feature = "dioxus")]
pub mod hooks;

pub use config::AuthConfig;
pub use error::{FieldError, PocketBaseError};
pub use jwt::{is_near_expiry, token_expiry_secs};
pub use pocketbase::{Collection, ListResult, PocketBase};
pub use psk::derive_psk;
pub use session::AuthSession;

#[cfg(feature = "pairing")]
pub use pairing::{
    MailboxRecord, PairingConfig, PairingError, WebPairingKeys, generate_code, pair_url,
    parse_pair_url, seal_session, seal_value, unseal_session, unseal_value,
};

#[cfg(feature = "dioxus")]
pub use hooks::{AuthState, provide_auth_state, use_auth, use_persistent_session};
