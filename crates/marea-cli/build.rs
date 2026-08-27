//! Two things cargo cannot work out on its own.
//!
//! 1. `include_dir!` embeds the template tree at compile time, but its inputs
//!    are not tracked as source files — so a newly *added* template stays
//!    invisible until an unrelated edit to `src/` forces a rebuild. That is a
//!    genuinely confusing failure: the file is on disk and the generator
//!    swears it does not exist.
//!
//! 2. What marea itself is built against. A freshly generated project has no
//!    lockfile, so it re-resolves everything from scratch — and drifts:
//!
//!    - `wavesyncdb` tracks `branch = "dev"`, so it lands on whatever the tip
//!      is that morning, which marea has never compiled against.
//!    - `dioxus = "0.7.9"` is a caret requirement, so a project generated
//!      after 0.7.10 ships picks *that* up — and `dx 0.7.9` then refuses to
//!      build it ("dx and dioxus versions are incompatible").
//!
//!    Neither can be fixed by changing the spec: an exact `=` requirement or a
//!    `rev =` would differ from marea's own spec and cause the double
//!    resolution the specs exist to prevent. The lockfile is where a *commit*
//!    or an exact version belongs, so we read marea's and bake it in.

use std::path::Path;

/// Crates whose types cross the app/framework boundary, plus the one the
/// build tool version-checks. These get pinned to marea's exact resolution.
const SHARED: [&str; 4] = ["dioxus", "sea-orm", "dioxus-sdk-storage", "wavesyncdb"];

fn main() {
    println!("cargo:rerun-if-changed=templates");

    let lock = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("Cargo.toml");
    let lock = lock.with_file_name("Cargo.lock");
    println!("cargo:rerun-if-changed={}", lock.display());

    let text = std::fs::read_to_string(&lock).unwrap_or_default();
    let packages = parse(&text);

    let revs: Vec<String> = packages
        .iter()
        .filter_map(|p| p.rev.as_ref().map(|r| format!("{}={}", p.name, r)))
        .collect();
    let versions: Vec<String> = packages
        .iter()
        .filter(|p| SHARED.contains(&p.name.as_str()))
        .map(|p| format!("{}={}", p.name, p.version))
        .collect();

    println!("cargo:rustc-env=MAREA_GIT_REVS={}", revs.join(","));
    println!("cargo:rustc-env=MAREA_PKG_VERSIONS={}", versions.join(","));
}

struct Package {
    name: String,
    version: String,
    /// `Some` for git dependencies.
    rev: Option<String>,
}

fn parse(lock: &str) -> Vec<Package> {
    let mut out = Vec::new();
    let mut name: Option<String> = None;

    for line in lock.lines() {
        let line = line.trim();
        if line == "[[package]]" {
            name = None;
        } else if let Some(v) = line.strip_prefix("name = ") {
            name = Some(v.trim_matches('"').to_string());
        } else if let Some(v) = line.strip_prefix("version = ") {
            // Emit as soon as name and version are both known; a git `source`
            // line, if there is one, follows and fills in the revision.
            if let Some(n) = &name {
                out.push(Package {
                    name: n.clone(),
                    version: v.trim_matches('"').to_string(),
                    rev: None,
                });
            }
        } else if let Some(v) = line.strip_prefix("source = ") {
            let v = v.trim_matches('"');
            if let Some(sha) = v.strip_prefix("git+").and_then(|s| s.split('#').nth(1))
                && let Some(last) = out.last_mut()
                && Some(&last.name) == name.as_ref()
            {
                last.rev = Some(sha.to_string());
            }
        }
    }
    out
}
