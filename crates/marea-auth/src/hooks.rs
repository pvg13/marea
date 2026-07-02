//! Dioxus integration: persistent session signal + app-wide auth context.
//!
//! Signal-safety discipline: [`AuthState`] is `Copy`; its async methods do
//! all network work first and only touch the signal with a final `.set()` —
//! no write guard is ever held across an `.await`.

use dioxus::prelude::*;
use dioxus_sdk_storage::{LocalStorage, use_synced_storage};

use crate::config::AuthConfig;
use crate::error::PocketBaseError;
use crate::pocketbase::PocketBase;
use crate::session::AuthSession;

/// Reactive auth-session signal backed by `dioxus-sdk-storage`'s
/// `LocalStorage` — file-backed on native, browser `localStorage` on web.
/// Reads the stored value on first call and writes every subsequent change
/// back automatically.
///
/// We deliberately bypass the SDK's `use_persistent` helper because in 0.7.0
/// it is hardcoded to `SessionStorage` (in-memory on native), which resets on
/// every app restart. `use_synced_storage::<LocalStorage, _>` gives actual
/// cross-restart persistence.
///
/// **Must be called inside a Dioxus component**, after the host binary has
/// invoked `dioxus_sdk_storage::set_dir!()` — otherwise the SDK panics on
/// first access.
pub fn use_persistent_session(cfg: &'static AuthConfig) -> Signal<Option<AuthSession>> {
    use_synced_storage::<LocalStorage, _>(
        cfg.session_storage_key.to_string(),
        || None::<AuthSession>,
    )
}

/// App-wide auth state. The `session` signal is persistent when created via
/// [`use_persistent_session`] — mutating it writes through to storage.
#[derive(Clone, Copy)]
pub struct AuthState {
    pub session: Signal<Option<AuthSession>>,
    cfg: &'static AuthConfig,
}

/// Wire up the auth context from a caller-provided signal (normally
/// [`use_persistent_session`], so the session outlives the component and
/// persists). Returns the state it provided.
pub fn provide_auth_state(
    cfg: &'static AuthConfig,
    session: Signal<Option<AuthSession>>,
) -> AuthState {
    use_context_provider(|| AuthState { session, cfg })
}

pub fn use_auth() -> AuthState {
    use_context::<AuthState>()
}

impl AuthState {
    pub fn config(&self) -> &'static AuthConfig {
        self.cfg
    }

    /// A fresh stateless client for this app's PocketBase instance. Safe to
    /// move into async tasks.
    pub fn client(&self) -> PocketBase {
        PocketBase::new(self.cfg)
    }

    pub fn is_authenticated(&self) -> bool {
        self.session.read().is_some()
    }

    /// Log in and persist the session. Network first, single `.set()` last.
    pub async fn login(mut self, email: &str, password: &str) -> Result<(), PocketBaseError> {
        let session = self.client().login(email, password).await?;
        self.session.set(Some(session));
        Ok(())
    }

    /// Register (optionally with a display name), log in, persist.
    pub async fn register(
        mut self,
        email: &str,
        password: &str,
        name: Option<&str>,
    ) -> Result<(), PocketBaseError> {
        let session = self.client().register_named(email, password, name).await?;
        self.session.set(Some(session));
        Ok(())
    }

    /// Drop the persisted session. Only touches auth state — callers
    /// orchestrate any cross-context teardown (sync engine reset etc.).
    pub fn logout(mut self) {
        self.session.set(None);
    }

    /// Refresh the token if it expires within `margin_secs`, persisting the
    /// new session. No-op when there is no session or it is still fresh.
    pub async fn ensure_fresh(mut self, margin_secs: u64) -> Result<(), PocketBaseError> {
        // Clone out — never hold a read guard across the await below.
        let Some(current) = self.session.peek().clone() else {
            return Ok(());
        };
        if let Some(refreshed) = self.client().ensure_fresh(&current, margin_secs).await? {
            self.session.set(Some(refreshed));
        }
        Ok(())
    }
}
