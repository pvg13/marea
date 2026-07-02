use serde::{Deserialize, Serialize};

/// Everything a client needs after a successful login. The `psk` field is the
/// long-lived per-user secret that feeds `WaveSyncDbBuilder` as the group
/// passphrase. The `token` is the PocketBase session token (use it as the
/// `Authorization` header for subsequent PocketBase calls).
///
/// Serde field names are part of the on-disk contract: existing apps persist
/// this exact JSON shape via `dioxus-sdk-storage` (`user_id`/`email`/`token`/
/// `psk`, snake_case). Renaming a field logs every existing user out.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSession {
    pub user_id: String,
    pub email: String,
    pub token: String,
    pub psk: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Locks the persisted JSON shape against the sessions apps already have
    /// on disk (Ascend `ascend_auth_session`, Mediterranea `auth_session`).
    #[test]
    fn serde_shape_is_stable() {
        let json =
            r#"{"user_id":"abc123def456ghi","email":"a@b.es","token":"tok","psk":"deadbeef"}"#;
        let s: AuthSession = serde_json::from_str(json).unwrap();
        assert_eq!(s.user_id, "abc123def456ghi");
        assert_eq!(serde_json::to_string(&s).unwrap(), json);
    }
}
