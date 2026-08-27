//! Finding a marea checkout to point generated manifests at.
//!
//! Path deps keep the loop tight while marea itself is moving: edit a
//! framework crate, rebuild the app, done. They only work when a checkout is
//! actually there, so the default is to look, and fall back to `git + tag`.

use std::path::{Path, PathBuf};

/// Environment override, for a checkout that is not beside the project.
pub const MAREA_PATH_ENV: &str = "MAREA_PATH";

/// Does this directory look like a marea checkout?
fn is_marea_checkout(dir: &Path) -> bool {
    dir.join("crates/marea-ui/Cargo.toml").is_file()
        && dir.join("crates/marea-auth/Cargo.toml").is_file()
}

/// Find a marea checkout for a project that will live at `target`.
///
/// Looks at `$MAREA_PATH` first, then at every sibling of the target
/// directory. Returns the path as given (absolute or relative to the target's
/// parent) so the caller can turn it into a manifest-relative prefix.
pub fn find_marea(target: &Path) -> Option<PathBuf> {
    if let Ok(p) = std::env::var(MAREA_PATH_ENV) {
        let p = PathBuf::from(p);
        return is_marea_checkout(&p).then_some(p);
    }

    let parent = target.parent()?;
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir() && is_marea_checkout(p))
        .collect();
    // Deterministic pick when several checkouts exist side by side.
    candidates.sort();
    candidates.into_iter().next()
}

/// The `path = "…"` prefix to write into the generated workspace manifest,
/// relative to the generated workspace root.
pub fn relative_prefix(target: &Path, marea: &Path) -> String {
    // Both live under the same parent in the common case, so `../<name>` is
    // both correct and readable. Anything else falls back to an absolute
    // path, which is still valid — just less portable.
    match (target.parent(), marea.file_name()) {
        (Some(parent), Some(name)) if marea.parent() == Some(parent) => {
            format!("../{}", name.to_string_lossy())
        }
        _ => marea.display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_checkout(root: &Path) {
        for c in ["marea-ui", "marea-auth"] {
            let dir = root.join("crates").join(c);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("Cargo.toml"), "").unwrap();
        }
    }

    /// A unique scratch directory without pulling in a temp-dir crate.
    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("marea-cli-locate-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_a_checkout_beside_the_target_directory() {
        let root = scratch("sibling");
        fake_checkout(&root.join("marea"));
        let target = root.join("my-app");

        let found = find_marea(&target).expect("sibling checkout should be found");
        assert_eq!(found, root.join("marea"));
        assert_eq!(relative_prefix(&target, &found), "../marea");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn ignores_siblings_that_are_not_marea() {
        let root = scratch("nonmarea");
        std::fs::create_dir_all(root.join("some-other-project/src")).unwrap();
        let target = root.join("my-app");

        assert_eq!(find_marea(&target), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn picks_deterministically_when_several_checkouts_exist() {
        let root = scratch("several");
        fake_checkout(&root.join("zzz-marea"));
        fake_checkout(&root.join("aaa-marea"));
        let target = root.join("my-app");

        assert_eq!(find_marea(&target), Some(root.join("aaa-marea")));
        std::fs::remove_dir_all(&root).ok();
    }
}
