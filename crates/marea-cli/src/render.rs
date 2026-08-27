//! Turning an [`Options`] into a list of files, without touching the disk.
//!
//! Two sources feed one plan:
//!
//! - **Manifests** come from [`crate::manifest`], built structurally.
//! - **Everything else** — `.rs`, `.css`, `.md`, `.yaml` — is a real file
//!   under `templates/`, rendered with minijinja.
//!
//! Whether a template is emitted at all is decided by `templates/when.toml`,
//! which maps a template path to a minijinja expression over [`Vars`].
//! Anything not listed is always emitted. Feature-specific files get their
//! own entry rather than an `{% if %}` inside a shared file, which keeps the
//! templates readable as the code they will become.
//!
//! [`plan`] is deliberately pure: tests assert on the file list and the
//! rendered contents without a temp directory in sight.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use include_dir::{Dir, include_dir};
use minijinja::Environment;

use crate::manifest::{self, AppCrate, Ctx, MareaSource, app_crates};
use crate::options::{Engine, Options, Target};

static TEMPLATES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/templates");

/// The conditions file, kept alongside the templates it governs.
const WHEN_FILE: &str = "when.toml";

/// One entry of the generated `Lang` enum.
///
/// marea's `Locale` is a *trait* — each app defines its own language enum —
/// so the generator has to mint both the variant identifier and the persisted
/// tag from the locale list the user gave.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocaleVar {
    /// The persisted tag, e.g. `"pt-br"`.
    pub tag: String,
    /// The Rust variant identifier, e.g. `PtBr`.
    pub variant: String,
    /// Whether this is the fallback locale (the first one listed).
    pub default: bool,
    /// True for locales this generator ships real copy for; anything else
    /// gets the English strings and a `TODO` marker.
    pub translated: bool,
}

impl LocaleVar {
    fn new(tag: &str, default: bool) -> Self {
        // `pt-br` / `pt_BR` → `PtBr`
        let variant = tag
            .split(['-', '_'])
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    Some(f) => f.to_ascii_uppercase().to_string() + &c.as_str().to_lowercase(),
                    None => String::new(),
                }
            })
            .collect();
        Self {
            translated: matches!(tag, "en" | "es"),
            tag: tag.to_string(),
            variant,
            default,
        }
    }
}

/// What templates can see. Flat and boolean-heavy on purpose: conditions in
/// `when.toml` read as `wavesync and web`, not as a path through nested
/// structures.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Vars {
    // names
    pub name: String,
    pub snake: String,
    pub pascal: String,
    pub title: String,
    // permanent compatibility constants
    pub app_dir: String,
    pub db_file: String,
    pub psk_domain: String,
    pub session_storage_key: String,
    pub sync_topic: String,
    // choices
    pub auth: bool,
    pub pocketbase_url: String,
    pub wavesync: bool,
    pub relay: String,
    pub tailwind: bool,
    pub bundle_id: String,
    pub with_example: bool,
    // targets
    pub desktop: bool,
    pub android: bool,
    pub ios: bool,
    pub mobile: bool,
    pub web: bool,
    pub native: bool,
    // marea features
    pub pairing: bool,
    pub scanner: bool,
    pub push: bool,
    pub android_back: bool,
    pub locales: Vec<LocaleVar>,
    pub default_locale: LocaleVar,
    // repo furniture
    pub ci: bool,
    pub containers: bool,
    pub agent_docs: bool,
    pub clippy: bool,
    pub docs: bool,
    pub e2e: bool,
}

impl Vars {
    pub fn new(o: &Options) -> Self {
        let d = o.derived();
        let relay = match &o.engine {
            Engine::WaveSync { relay } => relay.clone().unwrap_or_default(),
            Engine::None => String::new(),
        };
        Self {
            name: d.kebab.clone(),
            snake: d.snake.clone(),
            pascal: d.pascal.clone(),
            title: d.title.clone(),
            app_dir: d.app_dir.clone(),
            db_file: d.db_file.clone(),
            psk_domain: d.psk_domain.clone(),
            session_storage_key: d.session_storage_key.clone(),
            sync_topic: d.sync_topic.clone(),
            auth: o.has_auth(),
            pocketbase_url: o
                .auth
                .as_ref()
                .map(|a| a.pocketbase_url.clone())
                .unwrap_or_default(),
            wavesync: o.uses_wavesync(),
            relay,
            tailwind: o.tailwind,
            bundle_id: o.bundle_id.clone().unwrap_or_default(),
            with_example: o.with_example,
            desktop: o.has_target(Target::Desktop),
            android: o.has_target(Target::Android),
            ios: o.has_target(Target::Ios),
            mobile: o.has_mobile(),
            web: o.has_target(Target::Web),
            native: o.has_native(),
            pairing: o.features.pairing,
            scanner: o.features.scanner,
            push: o.features.push,
            android_back: o.features.android_back,
            default_locale: LocaleVar::new(
                o.features
                    .locales
                    .first()
                    .map(String::as_str)
                    .unwrap_or("en"),
                true,
            ),
            locales: o
                .features
                .locales
                .iter()
                .enumerate()
                .map(|(i, tag)| LocaleVar::new(tag, i == 0))
                .collect(),
            ci: o.infra.ci,
            containers: o.infra.containers,
            agent_docs: o.scaffolding.agent_docs,
            clippy: o.scaffolding.clippy,
            docs: o.scaffolding.docs,
            e2e: o.scaffolding.e2e,
        }
    }
}

/// One file to write, relative to the generated workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedFile {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub files: Vec<PlannedFile>,
}

impl Plan {
    pub fn paths(&self) -> Vec<&str> {
        self.files.iter().map(|f| f.path.as_str()).collect()
    }

    pub fn get(&self, path: &str) -> Option<&PlannedFile> {
        self.files.iter().find(|f| f.path == path)
    }

    fn push(&mut self, path: impl Into<String>, contents: impl Into<String>) {
        self.files.push(PlannedFile {
            path: path.into(),
            contents: contents.into(),
        });
    }
}

/// Parse `templates/when.toml` into `template path -> condition`.
fn conditions() -> Result<BTreeMap<String, String>> {
    let raw = TEMPLATES
        .get_file(WHEN_FILE)
        .context("templates/when.toml is missing from the embedded template tree")?
        .contents_utf8()
        .context("templates/when.toml is not valid UTF-8")?;
    let parsed: BTreeMap<String, String> =
        toml::from_str(raw).context("templates/when.toml does not parse")?;
    Ok(parsed)
}

/// Every template path in the embedded tree, `when.toml` excluded.
fn template_paths() -> Vec<String> {
    fn walk(dir: &Dir<'_>, out: &mut Vec<String>) {
        for f in dir.files() {
            let p = f.path().to_string_lossy().to_string();
            if p != WHEN_FILE {
                out.push(p);
            }
        }
        for d in dir.dirs() {
            walk(d, out);
        }
    }
    let mut out = Vec::new();
    walk(&TEMPLATES, &mut out);
    out.sort();
    out
}

/// Build the complete file plan for a set of options.
pub fn plan(options: &Options, marea: MareaSource) -> Result<Plan> {
    let vars = Vars::new(options);
    let ctx = Ctx::new(options, marea);
    let mut plan = Plan::default();

    // Manifests — structural, never templated.
    plan.push("Cargo.toml", manifest::workspace_manifest(&ctx));
    plan.push("crates/domain/Cargo.toml", manifest::domain_manifest(&ctx));
    plan.push("crates/ui/Cargo.toml", manifest::ui_manifest(&ctx));
    if options.uses_wavesync() {
        plan.push("crates/data/Cargo.toml", manifest::data_manifest(&ctx));
    }
    for app in app_crates(options) {
        plan.push(
            format!("crates/{}/Cargo.toml", app.dir_name()),
            manifest::app_manifest(&ctx, app),
        );
        plan.push(
            format!("crates/{}/Dioxus.toml", app.dir_name()),
            manifest::dioxus_toml(&ctx, app),
        );
    }
    if options.has_target(Target::Web) {
        plan.push(".cargo/config.toml", manifest::cargo_config(&ctx));
    }

    // Templates.
    let env = Environment::new();
    let conds = conditions()?;
    for path in template_paths() {
        if let Some(expr) = conds.get(&path) {
            let compiled = env
                .compile_expression(expr)
                .with_context(|| format!("condition for {path} does not compile: {expr}"))?;
            let keep = compiled
                .eval(&vars)
                .with_context(|| format!("condition for {path} failed to evaluate: {expr}"))?;
            if !keep.is_true() {
                continue;
            }
        }
        let src = TEMPLATES
            .get_file(&path)
            .expect("path came from the template tree")
            .contents_utf8()
            .with_context(|| format!("{path} is not valid UTF-8"))?;
        let rendered = env
            .render_str(src, &vars)
            .with_context(|| format!("rendering {path}"))?;
        plan.push(path, rendered);
    }

    plan.files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(plan)
}

/// The app crate a target's `main.rs` belongs to — used by `when.toml`
/// conditions and by the writer.
pub fn app_crate_dirs(options: &Options) -> Vec<&'static str> {
    app_crates(options)
        .into_iter()
        .map(AppCrate::dir_name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{Auth, Target};
    use std::collections::BTreeSet;

    fn minimal() -> Options {
        Options {
            name: "my-app".into(),
            targets: [Target::Desktop].into_iter().collect::<BTreeSet<_>>(),
            auth: Some(Auth {
                pocketbase_url: "https://admin.almares.es".into(),
            }),
            ..Default::default()
        }
    }

    fn planned(o: &Options) -> Plan {
        plan(
            o,
            MareaSource::Path {
                prefix: "../marea".into(),
            },
        )
        .expect("planning succeeds")
    }

    /// A renamed or deleted template would otherwise leave a dead condition
    /// behind, and the file it was meant to gate would start being emitted
    /// unconditionally.
    #[test]
    fn every_condition_refers_to_a_template_that_exists() {
        let paths = template_paths();
        for key in conditions().expect("when.toml parses").keys() {
            assert!(
                paths.contains(key),
                "when.toml gates `{key}`, which is not in the template tree"
            );
        }
    }

    #[test]
    fn every_condition_is_a_compilable_expression() {
        let env = Environment::new();
        let vars = Vars::new(&minimal());
        for (path, expr) in conditions().expect("when.toml parses") {
            let compiled = env
                .compile_expression(&expr)
                .unwrap_or_else(|e| panic!("condition for {path} does not compile: {expr}: {e}"));
            compiled
                .eval(&vars)
                .unwrap_or_else(|e| panic!("condition for {path} does not evaluate: {expr}: {e}"));
        }
    }

    #[test]
    fn a_minimal_plan_has_the_base_workspace() {
        let o = minimal();
        let p = planned(&o);
        for expected in [
            "Cargo.toml",
            "README.md",
            ".gitignore",
            "crates/domain/Cargo.toml",
            "crates/domain/src/lib.rs",
            "crates/ui/Cargo.toml",
            "crates/ui/src/lib.rs",
            "crates/ui/src/app.rs",
            "crates/ui/src/routes.rs",
            "crates/ui/src/shell.rs",
            "crates/ui/assets/tokens.css",
            "crates/app-desktop/Cargo.toml",
            "crates/app-desktop/src/main.rs",
        ] {
            assert!(
                p.get(expected).is_some(),
                "expected {expected} in the plan, got {:?}",
                p.paths()
            );
        }
    }

    #[test]
    fn a_plan_without_an_engine_has_no_data_crate() {
        let o = minimal();
        let p = planned(&o);
        assert!(
            p.paths().iter().all(|f| !f.starts_with("crates/data/")),
            "engine-less plan should have no data crate: {:?}",
            p.paths()
        );
    }

    #[test]
    fn a_plan_only_contains_the_app_crates_for_the_selected_targets() {
        let o = minimal();
        let p = planned(&o);
        assert!(p.get("crates/app-desktop/src/main.rs").is_some());
        assert!(p.get("crates/app-web/src/main.rs").is_none());
        assert!(p.get("crates/app-mobile/src/main.rs").is_none());
    }

    #[test]
    fn the_permanent_constants_reach_the_generated_bootstrap() {
        let o = minimal();
        let p = planned(&o);
        let app = &p.get("crates/ui/src/app.rs").unwrap().contents;
        assert!(
            app.contains(r#"b"my-app.psk.v1|""#),
            "psk domain missing from generated app.rs:\n{app}"
        );
        assert!(app.contains("my_app_auth_session"));
        assert!(app.contains("https://admin.almares.es"));
    }

    #[test]
    fn nothing_rendered_still_carries_template_syntax() {
        let o = minimal();
        for f in planned(&o).files {
            assert!(
                !f.contents.contains("{{") && !f.contents.contains("{%"),
                "{} still contains unrendered template syntax",
                f.path
            );
        }
    }

    #[test]
    fn the_plan_is_deterministic_and_sorted() {
        let o = minimal();
        let a = planned(&o);
        let b = planned(&o);
        assert_eq!(a.paths(), b.paths());
        let mut sorted = a.paths();
        sorted.sort();
        assert_eq!(a.paths(), sorted);
    }
}
