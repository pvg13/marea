//! Pinning a generated project to the commits marea was built against.
//!
//! `wavesyncdb` is a `branch = "dev"` dependency. The spec cannot be changed
//! to a `rev =` — it has to stay byte-identical to marea's, or cargo resolves
//! the crate twice and the shared types stop matching. So the pin has to
//! happen where cargo intends it to: in the lockfile.
//!
//! Without this, `marea new` on a Tuesday and `marea new` on a Thursday
//! produce projects built against different sync engines, and a generated app
//! can fail to compile because of a framework change nobody in the new
//! project has heard of.

use std::path::Path;
use std::process::Command;

/// `name=sha` pairs baked in at build time from marea's own `Cargo.lock`.
const BAKED: &str = env!("MAREA_GIT_REVS");

/// `name=version` for the crates whose types cross the app/framework
/// boundary, as marea resolves them.
const BAKED_VERSIONS: &str = env!("MAREA_PKG_VERSIONS");

/// The revisions marea resolves for its git dependencies.
pub fn marea_revisions() -> Vec<(&'static str, &'static str)> {
    BAKED
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| pair.split_once('='))
        .collect()
}

/// The crates.io packages pinned to marea's exact resolution.
///
/// A caret requirement is not a pin. `dioxus = "0.7.9"` happily resolves to
/// 0.7.10 in a fresh project, and `dx 0.7.9` then refuses to build it — so the
/// generated lockfile has to say which version marea actually uses.
pub fn marea_versions() -> Vec<(&'static str, &'static str)> {
    BAKED_VERSIONS
        .split(',')
        .filter(|s| !s.is_empty())
        .filter_map(|pair| pair.split_once('='))
        .collect()
}

/// Which git packages are worth pinning: the ones tracking a moving branch.
///
/// `dioxus-sdk-storage` is already pinned by `rev =` in the manifest, so
/// `wavesyncdb` is the only git dependency that can drift.
const BRANCH_TRACKED: [&str; 1] = ["wavesyncdb"];

fn run(root: &Path, args: &[&str]) -> (bool, String) {
    match Command::new("cargo").args(args).current_dir(root).output() {
        Ok(out) => {
            let mut text = String::from_utf8_lossy(&out.stderr).to_string();
            text.push_str(&String::from_utf8_lossy(&out.stdout));
            (out.status.success(), text)
        }
        Err(e) => (false, format!("could not run cargo: {e}")),
    }
}

/// Create the project's lockfile and pin the branch-tracked crates to the
/// same commits marea uses. Returns `(ok, detail)`.
pub fn pin(root: &Path) -> (bool, String) {
    // Git revisions for branch-tracked crates, plus exact versions for the
    // crates.io ones whose requirement is only a caret.
    let mut wanted: Vec<(&str, &str)> = marea_revisions()
        .into_iter()
        .filter(|(name, _)| BRANCH_TRACKED.contains(name))
        .collect();
    // A git dependency is pinned by its revision — either the one added above
    // or a `rev =` already in the manifest. `cargo update --precise <version>`
    // does not apply to those and fails outright, so only crates.io packages
    // get a version pin.
    let git_sourced: Vec<&str> = marea_revisions().into_iter().map(|(n, _)| n).collect();
    for (name, version) in marea_versions() {
        if !git_sourced.contains(&name) && !wanted.iter().any(|(n, _)| *n == name) {
            wanted.push((name, version));
        }
    }

    if wanted.is_empty() {
        return (true, "nothing to pin".into());
    }

    let (ok, detail) = run(root, &["generate-lockfile"]);
    if !ok {
        return (false, detail);
    }

    // A project without an engine has no wavesyncdb at all, and
    // `cargo update` on an absent package is a hard error rather than a
    // no-op. Pin only what this project actually resolved.
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
    let wanted: Vec<(&str, &str)> = wanted
        .into_iter()
        .filter(|(name, _)| lock.contains(&format!("name = \"{name}\"")))
        .collect();

    if wanted.is_empty() {
        return (
            true,
            "no branch-tracked dependencies in this project".into(),
        );
    }

    for (name, sha) in &wanted {
        let (ok, detail) = run(root, &["update", name, "--precise", sha]);
        if !ok {
            return (false, format!("pinning {name} to {sha} failed:\n{detail}"));
        }
    }

    let summary = wanted
        .iter()
        .map(|(n, s)| format!("{n}@{}", &s[..s.len().min(9)]))
        .collect::<Vec<_>>()
        .join(", ");
    (true, summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// If this is empty the build script failed to find marea's lockfile, and
    /// every generated project would silently float to the branch tip.
    #[test]
    fn mareas_resolved_revisions_were_baked_in() {
        let revs = marea_revisions();
        assert!(
            !revs.is_empty(),
            "no git revisions baked in — did build.rs fail to read marea's Cargo.lock?"
        );
        let (_, sha) = revs
            .iter()
            .find(|(n, _)| *n == "wavesyncdb")
            .expect("wavesyncdb should be in marea's lockfile");
        assert_eq!(sha.len(), 40, "expected a full commit sha, got {sha:?}");
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "expected a hex sha, got {sha:?}"
        );
    }

    /// Without this, `dx` refuses to build a generated project whenever a new
    /// dioxus patch has shipped since marea last updated.
    #[test]
    fn the_shared_crates_versions_were_baked_in() {
        let versions = marea_versions();
        for name in ["dioxus", "sea-orm"] {
            let (_, v) = versions
                .iter()
                .find(|(n, _)| *n == name)
                .unwrap_or_else(|| panic!("{name} should be in marea's lockfile"));
            assert!(
                v.chars().next().is_some_and(|c| c.is_ascii_digit()),
                "{name} version looks wrong: {v:?}"
            );
        }
    }

    /// `dioxus-sdk-storage` is a git dependency pinned by `rev =`; a version
    /// pin on it makes `cargo update --precise` fail outright.
    #[test]
    fn git_sourced_crates_are_never_given_a_version_pin() {
        let git_sourced: Vec<&str> = marea_revisions().into_iter().map(|(n, _)| n).collect();
        assert!(
            git_sourced.contains(&"dioxus-sdk-storage"),
            "expected dioxus-sdk-storage to be git-sourced in marea's lockfile"
        );
        assert!(
            git_sourced.contains(&"wavesyncdb"),
            "expected wavesyncdb to be git-sourced in marea's lockfile"
        );
    }

    #[test]
    fn every_branch_tracked_crate_is_actually_in_the_lockfile() {
        let revs = marea_revisions();
        for name in BRANCH_TRACKED {
            assert!(
                revs.iter().any(|(n, _)| *n == name),
                "{name} is listed as branch-tracked but absent from marea's lockfile"
            );
        }
    }
}
