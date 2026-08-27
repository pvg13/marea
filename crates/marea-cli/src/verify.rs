//! Post-generation checks.
//!
//! Generating a workspace that does not compile is the one failure a
//! scaffolding tool must not have: the person running it has no way to tell
//! their mistake from the generator's. So the generator builds what it just
//! wrote, and says so.
//!
//! The architecture gates run here too, against the freshly generated tree —
//! the same four checks the generated CI runs on every commit afterwards.

use std::path::Path;
use std::process::Command;

use anyhow::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

/// Format the files we just wrote — and *only* those.
///
/// `cargo fmt --all` is the obvious thing to reach for and is wrong here: run
/// inside a workspace whose manifests carry `path = "../marea"` dependencies,
/// it happily reformats the framework checkout next door. Scaffolding a new
/// project must never leave edits in someone else's repository, so this walks
/// the generated tree and runs rustfmt on those paths directly.
fn format_generated_files(root: &Path) -> Check {
    let mut files = Vec::new();
    collect_rs_files(&root.join("crates"), &mut files);
    if files.is_empty() {
        return Check {
            name: "rustfmt",
            passed: true,
            detail: String::new(),
        };
    }
    let mut args: Vec<&str> = vec!["--edition", "2024"];
    args.extend(files.iter().map(String::as_str));
    let (ok, detail) = run(root, "rustfmt", &args);
    Check {
        name: "rustfmt",
        passed: ok,
        detail,
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            collect_rs_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p.display().to_string());
        }
    }
}

fn run(root: &Path, program: &str, args: &[&str]) -> (bool, String) {
    match Command::new(program).args(args).current_dir(root).output() {
        Ok(out) => {
            let mut text = String::from_utf8_lossy(&out.stderr).to_string();
            text.push_str(&String::from_utf8_lossy(&out.stdout));
            (out.status.success(), text)
        }
        Err(e) => (false, format!("could not run {program}: {e}")),
    }
}

/// Format, then compile, then assert the architecture boundaries.
pub fn verify(root: &Path, has_wavesync: bool) -> Result<Vec<Check>> {
    let mut checks = Vec::new();

    checks.push(format_generated_files(root));

    // Before anything is compiled: pin the moving branch dependency to the
    // commit marea itself resolves, so the new project is built against the
    // framework's own engine rather than whatever the branch tip is today.
    let (ok, detail) = crate::lockfile::pin(root);
    checks.push(Check {
        name: "lockfile pinned to marea's revisions",
        passed: ok,
        detail,
    });

    let (ok, detail) = run(root, "cargo", &["check", "--workspace", "--all-targets"]);
    checks.push(Check {
        name: "cargo check",
        passed: ok,
        detail,
    });

    checks.push(shared_types_are_single_copy(root));
    checks.push(domain_is_pure(root));
    if has_wavesync {
        checks.push(ui_has_no_target_cfg(root));
    }

    Ok(checks)
}

/// The invariant marea's CLAUDE.md calls sacred.
///
/// Note what this does *not* assert: that `cargo tree -d` is empty. It never
/// is — the dioxus desktop stack alone carries dozens of duplicate leaves,
/// and none of them matter. What matters is that the crates whose *types*
/// cross the app/framework boundary resolve exactly once; two copies of those
/// means `use_context` silently finds nothing at runtime.
pub fn shared_types_are_single_copy(root: &Path) -> Check {
    const SHARED: [&str; 4] = ["wavesyncdb", "dioxus", "sea-orm", "dioxus-sdk-storage"];
    let (_, out) = run(root, "cargo", &["tree", "--duplicates"]);
    let offenders: Vec<&str> = SHARED
        .iter()
        .copied()
        .filter(|c| out.lines().any(|l| l.starts_with(&format!("{c} v"))))
        .collect();
    Check {
        name: "one copy of each shared crate",
        passed: offenders.is_empty(),
        detail: if offenders.is_empty() {
            String::new()
        } else {
            format!(
                "resolved twice: {}. Different source coordinates for the same crate \
                 produce distinct types, so contexts provided by one copy are invisible \
                 to the other.",
                offenders.join(", ")
            )
        },
    }
}

/// `domain` must not reach the UI or the database.
pub fn domain_is_pure(root: &Path) -> Check {
    let (ok, out) = run(root, "cargo", &["tree", "-p", "domain", "--prefix", "none"]);
    if !ok {
        return Check {
            name: "domain stays pure",
            passed: false,
            detail: out,
        };
    }
    let forbidden: Vec<&str> = ["dioxus", "sea-orm", "wavesyncdb"]
        .into_iter()
        .filter(|c| out.lines().any(|l| l.trim().starts_with(&format!("{c} v"))))
        .collect();
    Check {
        name: "domain stays pure",
        passed: forbidden.is_empty(),
        detail: if forbidden.is_empty() {
            String::new()
        } else {
            format!("`domain` depends on {}", forbidden.join(", "))
        },
    }
}

/// Screens must not know which target they are on.
pub fn ui_has_no_target_cfg(root: &Path) -> Check {
    let src = root.join("crates/ui/src");
    let mut offenders = Vec::new();
    collect_cfg_hits(&src, &mut offenders);
    Check {
        name: "ui has no target cfg",
        passed: offenders.is_empty(),
        detail: if offenders.is_empty() {
            String::new()
        } else {
            format!(
                "target-conditional code in {}. The capability belongs in `data`, \
                 behind a repository that is the same API on both sides.",
                offenders.join(", ")
            )
        },
    }
}

fn collect_cfg_hits(dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        if p.is_dir() {
            collect_cfg_hits(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs")
            && std::fs::read_to_string(&p)
                .map(|s| has_target_cfg(&s))
                .unwrap_or(false)
        {
            out.push(p.display().to_string());
        }
    }
}

/// Real target-conditional code, as opposed to prose about it.
///
/// The naive `contains("cfg(target_arch")` flags the doc comments that
/// explain the rule — including the ones this generator writes — so the check
/// would fail on a correct project. Comment lines are skipped, and only the
/// attribute and macro forms count.
fn has_target_cfg(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with('*') {
            return false;
        }
        let code = trimmed.split("//").next().unwrap_or(trimmed);
        code.contains("#[cfg(target_arch")
            || code.contains("#[cfg(not(target_arch")
            || code.contains("cfg!(target_arch")
            || code.contains("cfg_attr(target_arch")
            || code.contains("cfg_attr(not(target_arch")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("marea-cli-verify-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn prose_about_the_rule_is_not_a_violation_of_it() {
        // The generated `ui/src/lib.rs` documents this very rule, and an
        // earlier version of the check failed every correct project because
        // of it.
        assert!(!has_target_cfg(
            "//! No `#[cfg(target_arch)]` anywhere under `src/`.\n\nfn a() {}\n"
        ));
        assert!(!has_target_cfg(
            "    // cfg(target_arch = \"wasm32\") would go here\n"
        ));
        assert!(has_target_cfg(
            "#[cfg(target_arch = \"wasm32\")]\nfn b() {}\n"
        ));
        assert!(has_target_cfg(
            "    #[cfg(not(target_arch = \"wasm32\"))]\n"
        ));
        assert!(has_target_cfg("    if cfg!(target_arch = \"wasm32\") {}\n"));
    }

    #[test]
    fn the_ui_cfg_gate_catches_a_target_conditional_screen() {
        let root = scratch("uicfg");
        let src = root.join("crates/ui/src/features/items");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("clean.rs"), "fn a() {}\n").unwrap();
        assert!(ui_has_no_target_cfg(&root).passed);

        std::fs::write(
            src.join("dirty.rs"),
            "#[cfg(target_arch = \"wasm32\")]\nfn b() {}\n",
        )
        .unwrap();
        let check = ui_has_no_target_cfg(&root);
        assert!(!check.passed);
        assert!(check.detail.contains("dirty.rs"), "got: {}", check.detail);

        std::fs::remove_dir_all(&root).ok();
    }
}
