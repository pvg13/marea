//! WhatsApp-style device-pairing handoff via a PocketBase mailbox.
//!
//! The web client wants to log in, but only the phone has the credentials.
//! The phone can't reach the (potentially behind-NAT) web tab directly, so a
//! tiny PocketBase collection acts as a one-shot encrypted message passer:
//!
//! ```text
//!  Web (unauthenticated)                Phone (authenticated)
//!  ─────────────────────                ──────────────────────
//!  1. Generate X25519 keypair
//!  2. Generate random pairing code
//!  3. Create mailbox(code, pubkey)
//!  4. Render QR(code, pubkey)
//!  5. Poll mailbox by code ───────┐
//!                                 │   ─── camera scan ──►
//!                                 │   6. Parse code + pubkey
//!                                 │   7. Confirm with user
//!                                 │   8. Seal AuthSession with pubkey
//!                                 │   9. Patch mailbox.payload
//!                                 │
//!  10. Pull encrypted payload     │
//!  11. Unseal with privkey ◄──────┘
//!  12. Persist session, delete mailbox
//! ```
//!
//! The mailbox only ever stores the recipient's pubkey + opaque ciphertext. A
//! PocketBase admin can see metadata (creation time, that a pairing happened)
//! but **not** the AuthSession itself — the sealed-box ciphertext is
//! decryptable only by the holder of the web tab's ephemeral private key.
//!
//! ## Crypto: NaCl `crypto_box_seal`
//!
//! `crypto_box::SealedBox` implements the libsodium "anonymous sender" shape:
//! encrypt with the recipient's X25519 pubkey + a fresh ephemeral sender
//! keypair, prepend the sender pubkey to the ciphertext, throw the sender
//! privkey away. No authentication of *who* sent the message — fine here
//! since the pairing code itself is the bearer of authority (anyone who
//! scanned the QR within the TTL window).
//!
//! The generic [`seal_value`]/[`unseal_value`] pair is public so apps can run
//! their own one-shot handoffs (e.g. Mediterranea's household invites) over
//! the same mailbox pattern.

use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD as B64;
use crypto_box::aead::OsRng;
use crypto_box::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};

use crate::pocketbase::PocketBase;
use crate::session::AuthSession;

/// Per-app pairing knobs. Apps define one static next to their
/// [`crate::AuthConfig`]:
///
/// ```rust
/// use marea_auth::PairingConfig;
///
/// pub static PAIRING: PairingConfig = PairingConfig {
///     url_scheme: "mediterranea",
///     mailbox_collection: "pairing_mailboxes",
///     ttl_secs: 300,
/// };
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairingConfig {
    /// Deep-link scheme for the QR payload, e.g. `mediterranea` →
    /// `mediterranea://pair?code=…&pubkey=…`.
    pub url_scheme: &'static str,
    /// PocketBase collection holding in-flight mailboxes.
    pub mailbox_collection: &'static str,
    /// How long a mailbox stays valid before the web side gives up.
    pub ttl_secs: u64,
}

/// Length of the human-shareable pairing code embedded in the QR. 8 chars
/// over the 31-symbol alphabet gives ~40 bits of entropy — plenty for a
/// 5-minute TTL window with rate-limiting on the PocketBase side (and the
/// code's secrecy isn't what protects confidentiality — the sealed-box
/// pubkey is the real auth).
const CODE_LEN: usize = 8;

/// Alphabet used by [`generate_code`]. Base32 minus look-alikes (0/O, 1/I/L)
/// so codes are unambiguous when typed by hand as a backup.
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// Web-side ephemeral keypair. The pubkey ships in the QR; the privkey stays
/// on the web client and is dropped after a successful pairing.
#[derive(Clone)]
pub struct WebPairingKeys {
    pub secret: SecretKey,
    pub public: PublicKey,
}

impl WebPairingKeys {
    /// Generate a fresh X25519 keypair using the platform CSPRNG.
    pub fn generate() -> Self {
        let secret = SecretKey::generate(&mut OsRng);
        let public = secret.public_key();
        Self { secret, public }
    }

    /// Base64-encode the public key for embedding in the QR.
    pub fn pubkey_b64(&self) -> String {
        B64.encode(self.public.as_bytes())
    }
}

/// Generate a fresh pairing code. Eight uppercase alphanumeric chars.
pub fn generate_code() -> String {
    let mut bytes = [0u8; CODE_LEN];
    getrandom::getrandom(&mut bytes).expect("CSPRNG unavailable");
    bytes
        .iter()
        .map(|b| CODE_ALPHABET[(*b as usize) % CODE_ALPHABET.len()] as char)
        .collect()
}

/// The deep-link URL encoded into the QR the phone scans.
///
/// The pubkey is percent-encoded: base64 (`A-Za-z0-9+/=`) carries `+` and `/`,
/// both of which are ambiguous in a query value (`+` decodes as a space under
/// form-encoding rules), which would corrupt the key before it ever reaches
/// [`seal_value`]. `=` needs no escaping in a query value, and
/// [`WebPairingKeys::pubkey_b64`] emits unpadded base64 anyway.
/// [`parse_pair_url`] is the exact inverse.
pub fn pair_url(cfg: &PairingConfig, code: &str, pubkey_b64: &str) -> String {
    format!(
        "{}://pair?code={code}&pubkey={}",
        cfg.url_scheme,
        url_encode(pubkey_b64)
    )
}

/// Parse a scanned pairing deep link back into `(code, pubkey_b64)`.
///
/// Returns `None` for anything that isn't this app's `pair` link or that is
/// missing either field — the phone side treats that as "not a pairing QR"
/// and shows a scan error rather than attempting a handoff.
pub fn parse_pair_url(cfg: &PairingConfig, raw: &str) -> Option<(String, String)> {
    let prefix = format!("{}://pair?", cfg.url_scheme);
    let query = raw.trim().strip_prefix(&prefix)?;
    let mut code = None;
    let mut pubkey = None;
    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let k = parts.next()?;
        let v = parts.next().unwrap_or("");
        match k {
            "code" => code = Some(v.to_string()),
            "pubkey" => pubkey = Some(url_decode(v)),
            _ => {}
        }
    }
    let code = code?;
    let pubkey = pubkey?;
    if code.is_empty() || pubkey.is_empty() {
        return None;
    }
    Some((code, pubkey))
}

/// Escape the two base64 characters that a query value can't carry verbatim.
/// Avoids a `percent-encoding` dependency for one call site.
fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '+' => out.push_str("%2B"),
            '/' => out.push_str("%2F"),
            other => out.push(other),
        }
    }
    out
}

/// Inverse of [`url_encode`]. Decodes any `%XX` escape, not just the two we
/// emit, so a QR produced by a hand-rolled or third-party encoder still
/// parses. Byte-oriented: base64 is ASCII, so no UTF-8 reassembly is needed.
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < bytes.len() {
        // A malformed escape (`%` at the end, or non-hex digits) falls through
        // and is kept verbatim rather than dropped — better a wrong-looking key
        // that fails to unseal than a silently truncated one.
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            out.push(byte as char);
            i += 3;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Sealed-box encrypt any serializable value for the holder of
/// `recipient_pubkey_b64`. No sender authentication — the right shape for
/// one-shot handoffs (device pairing, invites) where possession of the
/// pairing code is the only authority.
pub fn seal_value<T: Serialize>(
    recipient_pubkey_b64: &str,
    value: &T,
) -> Result<String, PairingError> {
    let pubkey_bytes = B64
        .decode(recipient_pubkey_b64.trim())
        .map_err(|e| PairingError::Decode(format!("pubkey base64: {e}")))?;
    let pubkey_arr: [u8; 32] = pubkey_bytes
        .as_slice()
        .try_into()
        .map_err(|_| PairingError::Decode("pubkey must be 32 bytes".into()))?;
    let pubkey = PublicKey::from(pubkey_arr);

    let plaintext =
        serde_json::to_vec(value).map_err(|e| PairingError::Decode(format!("encode: {e}")))?;

    let ciphertext = pubkey
        .seal(&mut OsRng, &plaintext)
        .map_err(|e| PairingError::Crypto(format!("seal: {e}")))?;
    Ok(B64.encode(&ciphertext))
}

/// Inverse of [`seal_value`] — decode + sealed-box-unseal with the
/// recipient's stored ephemeral secret key.
pub fn unseal_value<T: serde::de::DeserializeOwned>(
    secret: &SecretKey,
    payload_b64: &str,
) -> Result<T, PairingError> {
    let ciphertext = B64
        .decode(payload_b64.trim())
        .map_err(|e| PairingError::Decode(format!("payload base64: {e}")))?;
    let plaintext = secret
        .unseal(&ciphertext)
        .map_err(|e| PairingError::Crypto(format!("unseal: {e}")))?;
    serde_json::from_slice(&plaintext).map_err(|e| PairingError::Decode(format!("decode: {e}")))
}

/// Seal an [`AuthSession`] for device pairing.
pub fn seal_session(
    recipient_pubkey_b64: &str,
    session: &AuthSession,
) -> Result<String, PairingError> {
    seal_value(recipient_pubkey_b64, session)
}

/// Inverse of [`seal_session`].
pub fn unseal_session(secret: &SecretKey, payload_b64: &str) -> Result<AuthSession, PairingError> {
    unseal_value(secret, payload_b64)
}

// ── PocketBase mailbox CRUD ──────────────────────────────────────────────────

/// A single mailbox row as returned by PocketBase.
#[derive(Clone, Debug, Deserialize)]
pub struct MailboxRecord {
    pub id: String,
    pub code: String,
    pub pubkey: String,
    #[serde(default)]
    pub payload: Option<String>,
}

#[derive(Deserialize)]
struct ListResponse {
    items: Vec<MailboxRecord>,
}

#[derive(Serialize)]
struct CreateBody<'a> {
    code: &'a str,
    pubkey: &'a str,
}

#[derive(Serialize)]
struct PatchBody<'a> {
    payload: &'a str,
}

impl PocketBase {
    /// Create an empty mailbox keyed by the pairing code. Called by the web
    /// client before showing the QR.
    pub async fn create_pairing_mailbox(
        &self,
        cfg: &PairingConfig,
        code: &str,
        pubkey_b64: &str,
    ) -> Result<MailboxRecord, PairingError> {
        let url = format!(
            "{}/api/collections/{}/records",
            self.base_url(),
            cfg.mailbox_collection
        );
        let body = CreateBody {
            code,
            pubkey: pubkey_b64,
        };
        let resp = self.http_client().post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(PairingError::Api(extract_error_message(&text)));
        }
        serde_json::from_str(&text).map_err(|e| PairingError::Decode(e.to_string()))
    }

    /// Look up a mailbox by its pairing code. Returns `None` if no mailbox
    /// matches (e.g. expired or invalid code). The mailbox may still be empty
    /// (`payload = None`) — the caller polls until it fills.
    pub async fn read_pairing_mailbox(
        &self,
        cfg: &PairingConfig,
        code: &str,
    ) -> Result<Option<MailboxRecord>, PairingError> {
        let url = format!(
            "{}/api/collections/{}/records?filter=code='{}'&perPage=1",
            self.base_url(),
            cfg.mailbox_collection,
            code
        );
        let resp = self.http_client().get(&url).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(PairingError::Api(extract_error_message(&text)));
        }
        let list: ListResponse =
            serde_json::from_str(&text).map_err(|e| PairingError::Decode(e.to_string()))?;
        Ok(list.items.into_iter().next())
    }

    /// Phone-side helper: fill the mailbox payload with the encrypted
    /// AuthSession. Takes the record id rather than the code so the phone can
    /// do a single `read → patch` round-trip without re-resolving the code on
    /// the update path.
    pub async fn fill_pairing_mailbox(
        &self,
        cfg: &PairingConfig,
        record_id: &str,
        payload_b64: &str,
    ) -> Result<(), PairingError> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.base_url(),
            cfg.mailbox_collection,
            record_id
        );
        let body = PatchBody {
            payload: payload_b64,
        };
        let resp = self.http_client().patch(&url).json(&body).send().await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PairingError::Api(extract_error_message(&text)));
        }
        Ok(())
    }

    /// Delete a mailbox after a successful read. Best-effort — the optional
    /// PocketBase TTL cron handles abandoned rows.
    pub async fn delete_pairing_mailbox(
        &self,
        cfg: &PairingConfig,
        record_id: &str,
        token: &str,
    ) -> Result<(), PairingError> {
        let url = format!(
            "{}/api/collections/{}/records/{}",
            self.base_url(),
            cfg.mailbox_collection,
            record_id
        );
        let resp = self
            .http_client()
            .delete(&url)
            .header(reqwest::header::AUTHORIZATION, token)
            .send()
            .await?;
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(PairingError::Api(extract_error_message(&text)));
        }
        Ok(())
    }
}

fn extract_error_message(body: &str) -> String {
    #[derive(Deserialize)]
    struct E {
        #[serde(default)]
        message: Option<String>,
    }
    serde_json::from_str::<E>(body)
        .ok()
        .and_then(|e| e.message)
        .filter(|m| !m.is_empty())
        .unwrap_or_else(|| body.to_string())
}

#[derive(thiserror::Error, Debug)]
pub enum PairingError {
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("api: {0}")]
    Api(String),
    #[error("decode: {0}")]
    Decode(String),
    #[error("crypto: {0}")]
    Crypto(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: PairingConfig = PairingConfig {
        url_scheme: "testapp",
        mailbox_collection: "pairing_mailboxes",
        ttl_secs: 300,
    };

    fn session() -> AuthSession {
        AuthSession {
            user_id: "u-123".into(),
            email: "test@example.com".into(),
            token: "tok-abc".into(),
            psk: "deadbeef".repeat(8),
        }
    }

    #[test]
    fn generate_code_is_correct_length_alphabet_and_random() {
        let code = generate_code();
        assert_eq!(code.len(), CODE_LEN);
        for c in code.chars() {
            assert!(
                CODE_ALPHABET.contains(&(c as u8)),
                "char {c} not in alphabet"
            );
        }
        assert_ne!(generate_code(), generate_code());
    }

    #[test]
    fn pair_url_uses_the_app_scheme() {
        let url = pair_url(&CFG, "ABCD2345", "pk==");
        assert_eq!(url, "testapp://pair?code=ABCD2345&pubkey=pk==");
    }

    /// `+` and `/` are legal base64 but ambiguous in a query value. This is
    /// the format Mediterranea has shipped since its pairing launch — the
    /// phone-side parser percent-decodes, so the escaping must stay.
    #[test]
    fn pair_url_escapes_base64_plus_and_slash() {
        let url = pair_url(&CFG, "ABCD2345", "a+b/c=");
        assert_eq!(url, "testapp://pair?code=ABCD2345&pubkey=a%2Bb%2Fc=");
    }

    #[test]
    fn parses_well_formed_pair_url() {
        let (code, pubkey) =
            parse_pair_url(&CFG, "testapp://pair?code=ABCD1234&pubkey=AAAAA%2BBBBB").unwrap();
        assert_eq!(code, "ABCD1234");
        assert_eq!(pubkey, "AAAAA+BBBB");
    }

    #[test]
    fn parse_rejects_wrong_scheme() {
        assert!(parse_pair_url(&CFG, "https://relay.example.es/pair?code=x&pubkey=y").is_none());
        // Another marea app's link must not pair into this one.
        assert!(parse_pair_url(&CFG, "otherapp://pair?code=x&pubkey=y").is_none());
    }

    #[test]
    fn parse_rejects_missing_fields() {
        assert!(parse_pair_url(&CFG, "testapp://pair?code=ABCD").is_none());
        assert!(parse_pair_url(&CFG, "testapp://pair?pubkey=KEY").is_none());
        assert!(parse_pair_url(&CFG, "testapp://pair?code=&pubkey=").is_none());
    }

    /// The two halves are used on different devices, so a mismatch would only
    /// surface as an unseal failure in the field. Lock them together over a
    /// real generated key, which is where `+`//` actually show up.
    #[test]
    fn pair_url_round_trips_through_parse() {
        let keys = WebPairingKeys::generate();
        let pubkey = keys.pubkey_b64();
        let code = generate_code();
        let (got_code, got_pubkey) = parse_pair_url(&CFG, &pair_url(&CFG, &code, &pubkey)).unwrap();
        assert_eq!(got_code, code);
        assert_eq!(got_pubkey, pubkey);

        // And the round-tripped key still decrypts a real sealed session.
        let s = session();
        let payload = seal_session(&got_pubkey, &s).unwrap();
        assert_eq!(unseal_session(&keys.secret, &payload).unwrap(), s);
    }

    #[test]
    fn keypair_pubkey_is_base64_32_bytes() {
        let keys = WebPairingKeys::generate();
        let raw = B64.decode(keys.pubkey_b64()).unwrap();
        assert_eq!(raw.len(), 32);
    }

    #[test]
    fn seal_and_unseal_round_trip() {
        let keys = WebPairingKeys::generate();
        let s = session();
        let payload = seal_session(&keys.pubkey_b64(), &s).unwrap();
        assert_eq!(unseal_session(&keys.secret, &payload).unwrap(), s);
    }

    #[test]
    fn seal_payload_is_not_plaintext_session() {
        let keys = WebPairingKeys::generate();
        let payload = seal_session(&keys.pubkey_b64(), &session()).unwrap();
        // Catches regressions where someone accidentally puts the json in the
        // payload field directly.
        assert!(!payload.contains("u-123"));
        assert!(!payload.contains("test@example.com"));
        assert!(!payload.contains("tok-abc"));
    }

    #[test]
    fn unseal_with_wrong_key_fails() {
        let keys = WebPairingKeys::generate();
        let other = WebPairingKeys::generate();
        let payload = seal_session(&keys.pubkey_b64(), &session()).unwrap();
        assert!(unseal_session(&other.secret, &payload).is_err());
    }

    #[test]
    fn generic_seal_value_round_trips_custom_payloads() {
        #[derive(Debug, PartialEq, Serialize, serde::Deserialize)]
        struct Invite {
            id: String,
            psk: String,
        }
        let keys = WebPairingKeys::generate();
        let invite = Invite {
            id: "hh-1".into(),
            psk: "cafe".repeat(16),
        };
        let payload = seal_value(&keys.pubkey_b64(), &invite).unwrap();
        assert!(!payload.contains(&invite.psk));
        assert_eq!(
            unseal_value::<Invite>(&keys.secret, &payload).unwrap(),
            invite
        );
    }
}
