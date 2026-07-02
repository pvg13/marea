//! Thin PocketBase REST client scoped to the stock `users` auth collection,
//! plus a generic [`Collection`] handle for app-defined collections.
//!
//! The client is **stateless** — sessions are passed in, never stored — so it
//! can be freely cloned into async tasks without signal-safety hazards.
//!
//! The WaveSync passphrase (PSK) is **derived client-side** from
//! `(user_id, password)` via Argon2id (see [`crate::derive_psk`]).
//! Trade-off: changing the user's PocketBase password rotates the PSK, which
//! forces every device to re-derive on next login. This is acceptable for our
//! threat model — the alternative would be storing the PSK on the server in
//! some form, which we explicitly want to avoid.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::config::AuthConfig;
use crate::error::{PocketBaseError, auth_error_from_body};
use crate::jwt::is_near_expiry;
use crate::psk::derive_psk;
use crate::session::AuthSession;

#[derive(Clone)]
pub struct PocketBase {
    base_url: String,
    psk_domain: Vec<u8>,
    client: reqwest::Client,
}

impl PocketBase {
    pub fn new(cfg: &AuthConfig) -> Self {
        Self::with_url(cfg.pocketbase_url, cfg.psk_domain)
    }

    /// Construct against an arbitrary URL (tests, local dev instances).
    pub fn with_url(url: impl Into<String>, psk_domain: impl Into<Vec<u8>>) -> Self {
        let url: String = url.into();
        Self {
            base_url: url.trim_end_matches('/').to_string(),
            psk_domain: psk_domain.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Borrow the inner `reqwest::Client`. Exposed so sibling modules
    /// (`pairing`) can issue their own collection-specific requests without
    /// reimplementing connection pooling.
    pub fn http_client(&self) -> &reqwest::Client {
        &self.client
    }

    /// Register a new user against the stock `users` collection, then log in.
    /// Only sends the fields PocketBase ships with by default; the PSK never
    /// touches the wire.
    pub async fn register(
        &self,
        email: &str,
        password: &str,
    ) -> Result<AuthSession, PocketBaseError> {
        self.register_named(email, password, None).await
    }

    /// [`register`](Self::register) with the optional display `name` field
    /// some apps set on the stock users collection.
    pub async fn register_named(
        &self,
        email: &str,
        password: &str,
        name: Option<&str>,
    ) -> Result<AuthSession, PocketBaseError> {
        let create_url = format!("{}/api/collections/users/records", self.base_url);
        let create_body = CreateRequest {
            email: email.to_string(),
            password: password.to_string(),
            password_confirm: password.to_string(),
            name: name.map(str::to_string),
        };
        let resp = self
            .client
            .post(&create_url)
            .json(&create_body)
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(auth_error_from_body(&body));
        }
        self.login(email, password).await
    }

    /// Authenticate with email + password. PocketBase returns identity and a
    /// session token; the PSK is derived locally from `(user_id, password)`.
    pub async fn login(&self, email: &str, password: &str) -> Result<AuthSession, PocketBaseError> {
        let url = format!("{}/api/collections/users/auth-with-password", self.base_url);
        let body = AuthRequest {
            identity: email.to_string(),
            password: password.to_string(),
        };
        let resp = self.client.post(&url).json(&body).send().await?;
        let (token, record) = parse_auth_response(resp).await?;
        let psk = derive_psk(&self.psk_domain, &record.id, password)?;
        Ok(AuthSession {
            user_id: record.id,
            email: record.email.unwrap_or_default(),
            token,
            psk,
        })
    }

    /// Refresh the session token. Cannot re-derive the PSK because we no
    /// longer have the password — preserves the PSK from the supplied session
    /// instead.
    pub async fn refresh(&self, session: &AuthSession) -> Result<AuthSession, PocketBaseError> {
        let url = format!("{}/api/collections/users/auth-refresh", self.base_url);
        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, &session.token)
            .send()
            .await?;
        let (token, record) = parse_auth_response(resp).await?;
        Ok(AuthSession {
            user_id: record.id,
            email: record.email.unwrap_or_else(|| session.email.clone()),
            token,
            psk: session.psk.clone(),
        })
    }

    /// Refresh only when the token expires within `margin_secs` (or doesn't
    /// parse). Returns `Ok(None)` when the session is still fresh, so callers
    /// can persist the new session only when one was actually issued.
    pub async fn ensure_fresh(
        &self,
        session: &AuthSession,
        margin_secs: u64,
    ) -> Result<Option<AuthSession>, PocketBaseError> {
        if !is_near_expiry(&session.token, margin_secs) {
            return Ok(None);
        }
        self.refresh(session).await.map(Some)
    }

    /// Change the account password. PocketBase requires the current password
    /// and invalidates all existing tokens on success — so the caller MUST
    /// re-login afterwards, which also re-derives the rotated PSK (see the
    /// module-level note). Field errors (wrong current password, too-short
    /// new one) come back as [`PocketBaseError::Auth`].
    pub async fn change_password(
        &self,
        session: &AuthSession,
        old_password: &str,
        new_password: &str,
    ) -> Result<(), PocketBaseError> {
        let url = format!(
            "{}/api/collections/users/records/{}",
            self.base_url, session.user_id
        );
        let body = ChangePasswordRequest {
            old_password: old_password.to_string(),
            password: new_password.to_string(),
            password_confirm: new_password.to_string(),
        };
        let resp = self
            .client
            .patch(&url)
            .header(reqwest::header::AUTHORIZATION, &session.token)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(auth_error_from_body(&text));
        }
        Ok(())
    }

    /// Request an email change. PocketBase emails a confirmation link to the
    /// **new** address; the change only takes effect once the user clicks it
    /// (so the local session keeps the old email until then).
    pub async fn request_email_change(
        &self,
        session: &AuthSession,
        new_email: &str,
    ) -> Result<(), PocketBaseError> {
        let url = format!(
            "{}/api/collections/users/request-email-change",
            self.base_url
        );
        let body = EmailChangeRequest {
            new_email: new_email.to_string(),
        };
        let resp = self
            .client
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, &session.token)
            .json(&body)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(auth_error_from_body(&text));
        }
        Ok(())
    }

    /// A handle on an app-defined collection (`group`, `pairing_mailboxes`,
    /// …). Pass the session token for authenticated collections, `None` for
    /// public ones.
    pub fn collection(&self, name: &str, token: Option<String>) -> Collection {
        Collection {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            token,
            name: name.to_string(),
        }
    }
}

/// Reads the response body as text first so any decode error can carry the
/// raw payload back to the caller. Returns the token + the (possibly partial)
/// user record — the PSK is *not* read from this response.
async fn parse_auth_response(
    resp: reqwest::Response,
) -> Result<(String, UserRecord), PocketBaseError> {
    let status = resp.status();
    let body = resp.text().await?;
    if !status.is_success() {
        return Err(auth_error_from_body(&body));
    }
    let data: AuthResponse = serde_json::from_str(&body).map_err(|e| PocketBaseError::Decode {
        body: body.clone(),
        message: e.to_string(),
    })?;
    Ok((data.token, data.record))
}

// ── generic collection CRUD ──────────────────────────────────────────────────

/// Paged list response for [`Collection::list`].
#[derive(Clone, Debug, Deserialize)]
pub struct ListResult<T> {
    pub page: u32,
    #[serde(rename = "totalItems")]
    pub total_items: u32,
    pub items: Vec<T>,
}

/// CRUD against one app-defined PocketBase collection. Obtained via
/// [`PocketBase::collection`].
pub struct Collection {
    client: reqwest::Client,
    base_url: String,
    token: Option<String>,
    name: String,
}

impl Collection {
    fn url(&self) -> String {
        format!("{}/api/collections/{}/records", self.base_url, self.name)
    }

    fn with_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => req.header(reqwest::header::AUTHORIZATION, t),
            None => req,
        }
    }

    async fn expect_json<T: DeserializeOwned>(
        resp: reqwest::Response,
    ) -> Result<T, PocketBaseError> {
        let status = resp.status();
        let body = resp.text().await?;
        if !status.is_success() {
            return Err(auth_error_from_body(&body));
        }
        serde_json::from_str(&body).map_err(|e| PocketBaseError::Decode {
            body,
            message: e.to_string(),
        })
    }

    pub async fn create<T: Serialize, R: DeserializeOwned>(
        &self,
        record: &T,
    ) -> Result<R, PocketBaseError> {
        let resp = self
            .with_auth(self.client.post(self.url()).json(record))
            .send()
            .await?;
        Self::expect_json(resp).await
    }

    pub async fn list<T: DeserializeOwned>(
        &self,
        filter: Option<&str>,
        page: Option<u32>,
        per_page: Option<u32>,
        expand: Option<&str>,
    ) -> Result<ListResult<T>, PocketBaseError> {
        let mut req = self.client.get(self.url());
        if let Some(f) = filter {
            req = req.query(&[("filter", f)]);
        }
        if let Some(p) = page {
            req = req.query(&[("page", p.to_string().as_str())]);
        }
        if let Some(pp) = per_page {
            req = req.query(&[("perPage", pp.to_string().as_str())]);
        }
        if let Some(e) = expand {
            req = req.query(&[("expand", e)]);
        }
        let resp = self.with_auth(req).send().await?;
        Self::expect_json(resp).await
    }

    pub async fn get_one<T: DeserializeOwned>(
        &self,
        id: &str,
        expand: Option<&str>,
    ) -> Result<T, PocketBaseError> {
        let mut req = self.client.get(format!("{}/{}", self.url(), id));
        if let Some(e) = expand {
            req = req.query(&[("expand", e)]);
        }
        let resp = self.with_auth(req).send().await?;
        Self::expect_json(resp).await
    }

    pub async fn update<T: Serialize, R: DeserializeOwned>(
        &self,
        id: &str,
        record: &T,
    ) -> Result<R, PocketBaseError> {
        let url = format!("{}/{}", self.url(), id);
        let resp = self
            .with_auth(self.client.patch(&url).json(record))
            .send()
            .await?;
        Self::expect_json(resp).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), PocketBaseError> {
        let url = format!("{}/{}", self.url(), id);
        let resp = self.with_auth(self.client.delete(&url)).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(auth_error_from_body(&body));
        }
        Ok(())
    }
}

// ── wire types ───────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct CreateRequest {
    email: String,
    password: String,
    #[serde(rename = "passwordConfirm")]
    password_confirm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

#[derive(Serialize)]
struct AuthRequest {
    identity: String,
    password: String,
}

#[derive(Serialize)]
struct ChangePasswordRequest {
    #[serde(rename = "oldPassword")]
    old_password: String,
    password: String,
    #[serde(rename = "passwordConfirm")]
    password_confirm: String,
}

#[derive(Serialize)]
struct EmailChangeRequest {
    #[serde(rename = "newEmail")]
    new_email: String,
}

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
    record: UserRecord,
}

#[derive(Deserialize)]
struct UserRecord {
    id: String,
    #[serde(default)]
    email: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_url_strips_trailing_slash() {
        let pb = PocketBase::with_url("http://localhost:8090/", b"t.v1|".to_vec());
        assert_eq!(pb.base_url(), "http://localhost:8090");
        let col = pb.collection("test", None);
        assert_eq!(
            col.url(),
            "http://localhost:8090/api/collections/test/records"
        );
    }

    #[test]
    fn new_reads_config() {
        static CFG: AuthConfig = AuthConfig {
            pocketbase_url: "https://admin.almares.es",
            psk_domain: b"test.psk.v1|",
            session_storage_key: "test_session",
        };
        let pb = PocketBase::new(&CFG);
        assert_eq!(pb.base_url(), "https://admin.almares.es");
    }
}
