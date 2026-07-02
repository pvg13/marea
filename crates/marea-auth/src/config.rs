//! Per-app static configuration.

/// Everything app-specific about auth, in one static the app defines once:
///
/// ```rust
/// use marea_auth::AuthConfig;
///
/// pub static AUTH: AuthConfig = AuthConfig {
///     pocketbase_url: "https://admin.almares.es",
///     psk_domain: b"ascend.psk.v1|",
///     session_storage_key: "ascend_auth_session",
/// };
/// ```
///
/// # Compatibility invariants
///
/// For an app migrating onto marea, these values MUST match what the app
/// shipped with, byte for byte:
///
/// - `psk_domain` feeds the Argon2id salt — changing it silently rotates every
///   existing user's WaveSyncDB passphrase.
/// - `session_storage_key` is where `dioxus-sdk-storage` persisted the session
///   — changing it logs every existing user out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthConfig {
    /// Base URL of the PocketBase instance, no trailing slash
    /// (e.g. `https://admin.almares.es`).
    pub pocketbase_url: &'static str,
    /// Domain-separation tag mixed into the Argon2 salt as **raw bytes**,
    /// conventionally `b"<app>.psk.v1|"`. Keeps PSKs distinct across apps
    /// sharing the same PocketBase accounts.
    pub psk_domain: &'static [u8],
    /// `dioxus-sdk-storage` key for the persisted [`crate::AuthSession`].
    pub session_storage_key: &'static str,
}
