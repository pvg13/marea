//! Per-platform database locations.
//!
//! `dioxus_sdk_storage::data_directory()` already resolves the
//! platform-correct writable directory (Linux `~/.local/share`, Android
//! `getFilesDir()` via JNI, iOS bundle container), so we never re-implement
//! that logic. The host binary must call
//! `set_dir!(dioxus_sdk_storage::data_directory().join("<App>"))` in `main()`
//! before launching so the storage backend and the SQLite paths resolve to
//! the same tree.

use std::path::PathBuf;

/// Names an app's data tree and produces its SQLite URLs.
///
/// ```rust,ignore
/// // Ascend, byte-compatible with its pre-marea layout:
/// pub static DB: DbLocator = DbLocator::new("Ascend", "ascend.db");
/// let url = DB.user_db_url(&session.user_id);
/// // → sqlite:{data_dir}/Ascend/u/{user_id}/ascend.db?mode=rwc
/// ```
///
/// # Compatibility invariant
///
/// `app_dir`/`db_file` are part of an app's on-disk contract — changing them
/// on a migrated app orphans every existing install's database.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DbLocator {
    /// Directory under the platform data dir, e.g. `"Ascend"`,
    /// `"Mediterranea"`.
    pub app_dir: &'static str,
    /// SQLite file name, e.g. `"ascend.db"`.
    pub db_file: &'static str,
}

impl DbLocator {
    pub const fn new(app_dir: &'static str, db_file: &'static str) -> Self {
        Self { app_dir, db_file }
    }

    /// The app's data directory (e.g. `~/.local/share/Ascend`).
    pub fn app_data_dir(&self) -> PathBuf {
        dioxus_sdk_storage::data_directory().join(self.app_dir)
    }

    /// Filesystem path to a user's isolated database.
    pub fn user_db_path(&self, user_id: &str) -> PathBuf {
        self.app_data_dir()
            .join("u")
            .join(sanitize_id(user_id))
            .join(self.db_file)
    }

    /// Per-account SQLite URL. Each user gets an **isolated** local database
    /// under `<data dir>/u/<user_id>/`, so logging in with a different
    /// account on the same device never reads the previous account's rows.
    /// (The WaveSync PSK isolates *network* sync between accounts, but a
    /// single shared file would still physically hold every account's data.)
    /// Creates the directory eagerly so SQLite can open the file on first
    /// run.
    pub fn user_db_url(&self, user_id: &str) -> String {
        let path = self.user_db_path(user_id);
        if let Some(parent) = path.parent() {
            // Failure here means SQLite can't open the file below — log the
            // root cause now so the downstream "unable to open database
            // file" isn't the only trace.
            if let Err(e) = std::fs::create_dir_all(parent) {
                log::error!("create user data dir {}: {e}", parent.display());
            }
        }
        format!("sqlite:{}?mode=rwc", path.display())
    }

    /// The single-file legacy path (`<data dir>/<db_file>`), for apps that
    /// shipped before per-account isolation and run a one-time lift.
    pub fn legacy_db_path(&self) -> PathBuf {
        self.app_data_dir().join(self.db_file)
    }

    /// A scope-isolated database (roommates-style: one DB per group), living
    /// directly under the app data dir as `<stem>_<scope_id>.db`.
    pub fn scoped_db_url(&self, scope_id: &str) -> String {
        let stem = self.db_file.strip_suffix(".db").unwrap_or(self.db_file);
        let dir = self.app_data_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            log::error!("create data dir {}: {e}", dir.display());
        }
        let path = dir.join(format!("{stem}_{}.db", sanitize_id(scope_id)));
        format!("sqlite:{}?mode=rwc", path.display())
    }
}

/// The `wavesync_<app>_<scope>` topic convention (roommates:
/// `wavesync_roommates_{group_id}`).
pub fn scoped_topic(app: &str, scope_id: &str) -> String {
    format!("wavesync_{app}_{scope_id}")
}

/// Keep a user id to a safe single path segment (PocketBase ids are already
/// `[a-z0-9]{15}`, but be defensive against path traversal / separators).
fn sanitize_id(user_id: &str) -> String {
    let s: String = user_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if s.is_empty() { "anon".to_string() } else { s }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DB: DbLocator = DbLocator::new("TestApp", "testapp.db");

    #[test]
    fn user_db_url_shape_matches_the_apps_contract() {
        // The exact layout Ascend/Mediterranea shipped with:
        // sqlite:{data}/{App}/u/{user_id}/{file}?mode=rwc
        let url = DB.user_db_url("abc123def456ghi");
        assert!(url.starts_with("sqlite:"), "{url}");
        assert!(url.ends_with("?mode=rwc"), "{url}");
        let path = format!(
            "TestApp{sep}u{sep}abc123def456ghi{sep}testapp.db",
            sep = std::path::MAIN_SEPARATOR
        );
        assert!(url.contains(&path), "{url}");
    }

    #[test]
    fn user_dbs_are_isolated_per_account() {
        let a = DB.user_db_url("aaaaaaaaaaaaaaa");
        let b = DB.user_db_url("bbbbbbbbbbbbbbb");
        assert_ne!(a, b);
        assert_ne!(
            a,
            format!("sqlite:{}?mode=rwc", DB.legacy_db_path().display())
        );
    }

    #[test]
    fn scoped_db_url_uses_stem_and_scope() {
        let url = DB.scoped_db_url("group42");
        assert!(url.contains("testapp_group42.db"), "{url}");
        assert!(url.ends_with("?mode=rwc"), "{url}");
    }

    #[test]
    fn scoped_topic_convention() {
        assert_eq!(scoped_topic("roommates", "g1"), "wavesync_roommates_g1");
    }

    #[test]
    fn sanitize_id_strips_path_separators() {
        assert_eq!(sanitize_id("../../etc/passwd"), "______etc_passwd");
        assert_eq!(sanitize_id("abc123"), "abc123");
        assert_eq!(sanitize_id(""), "anon");
    }
}
