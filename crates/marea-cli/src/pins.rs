//! The one table of external dependency specs generated workspaces inherit.
//!
//! # Why this is a table and not a template
//!
//! marea's own `CLAUDE.md` calls this invariant sacred: a dependency whose
//! *source coordinates* differ between marea and an app resolves **twice**.
//! Two resolutions of the same git branch are two distinct sets of types, so
//! `use_context::<SyncHandle>()` in the app finds nothing the framework put
//! there and the failure surfaces at runtime, far from its cause.
//!
//! Feature lists may differ freely — cargo unions them. Only the coordinates
//! (version req, git URL, branch/rev/tag) have to line up, so that is exactly
//! what [`Source`] models and what the parity test at the bottom of this file
//! checks against marea's own `[workspace.dependencies]`.
//!
//! Pins whose name also appears in marea's workspace manifest are parity
//! checked. The rest ([`APP_ONLY`]) are dependencies no marea crate carries —
//! browser glue, Android logging, UUIDs — and are listed explicitly so a
//! typo in a pin name can never quietly opt out of the check.

/// Where a dependency comes from. Feature lists are deliberately not part of
/// this: cargo unions features, so they cannot cause a double resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// A crates.io version requirement, e.g. `"0.7.9"` or `"2.0.0-rc"`.
    Version(&'static str),
    Git {
        url: &'static str,
        git_ref: GitRef,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitRef {
    Branch(&'static str),
    Rev(&'static str),
    Tag(&'static str),
}

/// An owned, comparable rendering of a dependency's source coordinates —
/// what the parity test compares across two manifests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Coords {
    Version(String),
    Git {
        url: String,
        /// `"branch"`, `"rev"` or `"tag"`.
        kind: &'static str,
        value: String,
    },
}

impl Source {
    pub fn coords(&self) -> Coords {
        match self {
            Source::Version(v) => Coords::Version((*v).to_string()),
            Source::Git { url, git_ref } => {
                let (kind, value) = match git_ref {
                    GitRef::Branch(b) => ("branch", *b),
                    GitRef::Rev(r) => ("rev", *r),
                    GitRef::Tag(t) => ("tag", *t),
                };
                Coords::Git {
                    url: (*url).to_string(),
                    kind,
                    value: value.to_string(),
                }
            }
        }
    }
}

/// One entry in the generated `[workspace.dependencies]` table.
pub struct Pin {
    pub name: &'static str,
    pub source: Source,
    pub features: &'static [&'static str],
    /// `Some(false)` emits `default-features = false`.
    pub default_features: Option<bool>,
    /// Rationale emitted above the line in the generated manifest. These
    /// comments are why the pins exist; a generated app that loses them
    /// loses the reason not to "just bump" a version.
    pub comment: Option<&'static str>,
}

/// Dependencies with no marea counterpart, exempt from the parity check.
/// Listed by name so a misspelled [`PINS`] entry fails the test instead of
/// silently skipping it.
pub const APP_ONLY: &[&str] = &[
    "uuid",
    "chrono",
    "js-sys",
    "web-sys",
    "wasm-logger",
    "wasm-bindgen-futures",
    "console_error_panic_hook",
    "android_logger",
    "ndk-context",
    "tracing-log",
];

pub const PINS: &[Pin] = &[
    Pin {
        name: "dioxus",
        source: Source::Version("0.7.9"),
        features: &["router"],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "dioxus-sdk-storage",
        source: Source::Git {
            url: "https://github.com/DioxusLabs/sdk.git",
            git_ref: GitRef::Rev("15525043cadc3c467afb15888ce804bdc9b6a46e"),
        },
        features: &[],
        default_features: None,
        comment: Some(
            "Same revision marea pins. crates.io 0.7.0 predates `data_directory()`\n\
             and the Android JNI bridge for `getFilesDir()`. The spec must stay\n\
             identical to marea's or `set_dir!` runs against a different copy of\n\
             the crate than the one the session is read from.",
        ),
    },
    Pin {
        name: "wavesyncdb",
        source: Source::Git {
            url: "https://github.com/pvg13/WaveSyncDB.git",
            git_ref: GitRef::Branch("dev"),
        },
        features: &["derive", "dioxus"],
        default_features: None,
        comment: Some(
            "Pinned here so the whole workspace resolves ONE copy of the branch.\n\
             Prefer the `marea_sync::wavesyncdb` re-export in app code; the `data`\n\
             crate depends on it directly because the derive macro expands to\n\
             absolute `::wavesyncdb::…` paths.",
        ),
    },
    Pin {
        name: "sea-orm",
        source: Source::Version("2.0.0-rc"),
        features: &["sqlx-sqlite", "runtime-tokio", "macros"],
        default_features: None,
        comment: Some(
            "Deliberately NON-strict so a stricter pin elsewhere in the mesh wins\n\
             unification. rc.41 pulls sqlx 0.9, which needs rustc 1.94; this\n\
             workspace builds on 1.92 (see rust-toolchain.toml).",
        ),
    },
    Pin {
        name: "serde",
        source: Source::Version("1.0"),
        features: &["derive"],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "serde_json",
        source: Source::Version("1.0"),
        features: &[],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "tokio",
        source: Source::Version("1.47"),
        features: &["full"],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "getrandom",
        source: Source::Version("0.2"),
        features: &["js"],
        default_features: None,
        comment: Some(
            "The `js` feature routes entropy to `crypto.getRandomValues`. Its\n\
             companion rustflag in .cargo/config.toml selects the same backend\n\
             for the `getrandom 0.3` pulled in transitively.",
        ),
    },
    Pin {
        name: "log",
        source: Source::Version("0.4"),
        features: &[],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "thiserror",
        source: Source::Version("2.0"),
        features: &[],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "jni",
        source: Source::Version("0.21"),
        features: &[],
        default_features: None,
        comment: Some("Android JNI bridge — same spec marea-ui's back gesture uses."),
    },
    // ── app-only, no marea counterpart ───────────────────────────────────
    Pin {
        name: "uuid",
        source: Source::Version("1"),
        features: &["v4"],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "chrono",
        source: Source::Version("0.4"),
        // `serde` is not optional here: domain types carry timestamps and are
        // serialized. Native builds can get it by accident through another
        // crate's feature unification, so leaving it out fails only on wasm —
        // late, and far from the cause.
        features: &["clock", "std", "serde"],
        default_features: Some(false),
        comment: None,
    },
    Pin {
        name: "js-sys",
        source: Source::Version("0.3"),
        features: &[],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "web-sys",
        source: Source::Version("0.3"),
        features: &["Window", "Location"],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "wasm-logger",
        source: Source::Version("0.2"),
        features: &[],
        default_features: None,
        comment: Some("Bridges `log::*` (wavesyncdb's engine) to the devtools console."),
    },
    Pin {
        name: "wasm-bindgen-futures",
        source: Source::Version("0.4"),
        features: &[],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "console_error_panic_hook",
        source: Source::Version("0.1"),
        features: &[],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "android_logger",
        source: Source::Version("0.14"),
        features: &[],
        default_features: None,
        comment: Some(
            "wavesyncdb installs a logger only from its JNI entry points, so the\n\
             in-process engine is invisible to logcat unless we install one here.",
        ),
    },
    Pin {
        name: "ndk-context",
        source: Source::Version("0.1"),
        features: &[],
        default_features: None,
        comment: None,
    },
    Pin {
        name: "tracing-log",
        source: Source::Version("0.2"),
        features: &[],
        default_features: None,
        comment: Some(
            "Bridges `log` records (wavesyncdb) into the `tracing` subscriber dx\n\
             installs — without it the sync engine is invisible on desktop.",
        ),
    },
];

/// Look up a pin by name.
///
/// Panics on an unknown name: call sites are all internal, so a miss is a
/// bug in the generator rather than bad user input.
pub fn pin(name: &str) -> &'static Pin {
    PINS.iter()
        .find(|p| p.name == name)
        .unwrap_or_else(|| panic!("no pin named `{name}` — add it to pins::PINS"))
}

/// Read the source coordinates out of a parsed manifest dependency entry.
/// Returns `None` for path deps and anything shaped unexpectedly.
pub fn coords_of(item: &toml_edit::Item) -> Option<Coords> {
    if let Some(v) = item.as_str() {
        return Some(Coords::Version(v.to_string()));
    }
    let t = item.as_table_like()?;
    if let Some(url) = t.get("git").and_then(|g| g.as_str()) {
        for kind in ["branch", "rev", "tag"] {
            if let Some(value) = t.get(kind).and_then(|v| v.as_str()) {
                return Some(Coords::Git {
                    url: url.to_string(),
                    kind: match kind {
                        "branch" => "branch",
                        "rev" => "rev",
                        _ => "tag",
                    },
                    value: value.to_string(),
                });
            }
        }
        return None;
    }
    let v = t.get("version")?.as_str()?;
    Some(Coords::Version(v.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// marea's own workspace manifest — the source of truth these pins copy.
    fn marea_workspace_deps() -> toml_edit::DocumentMut {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Cargo.toml");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        text.parse().expect("marea Cargo.toml parses")
    }

    /// The check that keeps `cargo tree -d` clean in every generated app.
    #[test]
    fn every_pin_matches_mareas_source_coordinates() {
        let doc = marea_workspace_deps();
        let deps = doc["workspace"]["dependencies"]
            .as_table()
            .expect("[workspace.dependencies]");

        let mut checked = 0;
        for p in PINS {
            let Some(item) = deps.get(p.name) else {
                continue;
            };
            let theirs = coords_of(item)
                .unwrap_or_else(|| panic!("could not read source coords for `{}`", p.name));
            assert_eq!(
                p.source.coords(),
                theirs,
                "pin `{}` has drifted from marea's [workspace.dependencies]. \
                 Different source coordinates for the same crate resolve twice \
                 and break shared types at runtime.",
                p.name
            );
            checked += 1;
        }
        assert!(
            checked >= 8,
            "expected most pins to be parity-checked, only {checked} were"
        );
    }

    /// A misspelled pin name would otherwise skip the parity check silently.
    #[test]
    fn pins_without_a_marea_counterpart_are_declared_app_only() {
        let doc = marea_workspace_deps();
        let deps = doc["workspace"]["dependencies"]
            .as_table()
            .expect("[workspace.dependencies]");

        let unmatched: Vec<&str> = PINS
            .iter()
            .filter(|p| deps.get(p.name).is_none())
            .map(|p| p.name)
            .collect();

        for name in &unmatched {
            assert!(
                APP_ONLY.contains(name),
                "pin `{name}` is absent from marea's workspace manifest and is not \
                 declared in APP_ONLY — is it a typo?"
            );
        }
    }

    #[test]
    fn pin_names_are_unique() {
        let mut names: Vec<&str> = PINS.iter().map(|p| p.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "duplicate pin name");
    }

    #[test]
    fn coords_of_reads_each_dependency_shape() {
        let doc: toml_edit::DocumentMut = r#"
plain = "1.0"
table = { version = "2.0", features = ["x"] }
branch = { git = "https://example/a.git", branch = "dev" }
rev = { git = "https://example/b.git", rev = "abc123" }
local = { path = "../elsewhere" }
"#
        .parse()
        .unwrap();

        assert_eq!(
            coords_of(&doc["plain"]),
            Some(Coords::Version("1.0".into()))
        );
        assert_eq!(
            coords_of(&doc["table"]),
            Some(Coords::Version("2.0".into()))
        );
        assert_eq!(
            coords_of(&doc["branch"]),
            Some(Coords::Git {
                url: "https://example/a.git".into(),
                kind: "branch",
                value: "dev".into()
            })
        );
        assert_eq!(
            coords_of(&doc["rev"]),
            Some(Coords::Git {
                url: "https://example/b.git".into(),
                kind: "rev",
                value: "abc123".into()
            })
        );
        assert_eq!(coords_of(&doc["local"]), None);
    }

    #[test]
    fn pin_lookup_finds_known_crates() {
        assert_eq!(pin("wavesyncdb").features, &["derive", "dioxus"]);
        assert!(matches!(pin("dioxus").source, Source::Version("0.7.9")));
    }
}
