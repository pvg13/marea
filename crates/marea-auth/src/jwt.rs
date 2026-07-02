//! JWT freshness helpers — enough to decide when to refresh, no verification.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// Extract the `exp` (expiry, unix seconds) claim from a JWT payload.
/// Returns `None` for anything that doesn't parse — callers treat that as
/// "assume expired, refresh".
pub fn token_expiry_secs(token: &str) -> Option<u64> {
    let payload = token.split('.').nth(1)?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    json["exp"].as_u64()
}

/// True when the token expires within `margin_secs` from now (or doesn't
/// carry a parseable expiry at all).
pub fn is_near_expiry(token: &str, margin_secs: u64) -> bool {
    token_expiry_secs(token).is_none_or(|exp| now_secs() + margin_secs >= exp)
}

#[cfg(not(target_arch = "wasm32"))]
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(target_arch = "wasm32")]
fn now_secs() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) fn make_jwt(exp: u64) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(format!(r#"{{"exp":{exp}}}"#));
        format!("{header}.{payload}.signature")
    }

    #[test]
    fn expiry_from_valid_jwt() {
        assert_eq!(
            token_expiry_secs(&make_jwt(1_700_000_000)),
            Some(1_700_000_000)
        );
    }

    #[test]
    fn expiry_missing_or_malformed_is_none() {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"HS256"}"#);
        let no_exp = format!("{header}.{}.sig", URL_SAFE_NO_PAD.encode(r#"{"sub":"u"}"#));
        assert_eq!(token_expiry_secs(&no_exp), None);
        assert_eq!(token_expiry_secs("header.!!!bad-base64!!!.sig"), None);
        assert_eq!(token_expiry_secs("nodots"), None);
        assert_eq!(token_expiry_secs(""), None);
        let bad_json = format!("{header}.{}.sig", URL_SAFE_NO_PAD.encode("not json"));
        assert_eq!(token_expiry_secs(&bad_json), None);
    }

    #[test]
    fn near_expiry_boundaries() {
        assert!(
            is_near_expiry(&make_jwt(946_684_800), 0),
            "year 2000 = expired"
        );
        assert!(
            !is_near_expiry(&make_jwt(4_102_444_800), 300),
            "year 2100 = fresh"
        );
        assert!(is_near_expiry("garbage", 0), "unparseable = assume expired");
    }
}
