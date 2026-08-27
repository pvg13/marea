//! Manifest builders.
//!
//! Every `Cargo.toml`, `Dioxus.toml`, `.cargo/config.toml` and
//! `rust-toolchain.toml` in a generated workspace is built here, structurally,
//! with `toml_edit` — never rendered from a text template.
//!
//! The reason is failure mode. A template with fifteen nested conditionals
//! around dependency lines fails by emitting a manifest that parses but wires
//! the wrong thing, and the symptom shows up as a duplicate resolution at
//! runtime. Built structurally, dependency specs come from [`crate::pins`],
//! the shape is checked by the type system, and the tests below assert on the
//! *parsed* result rather than on text.

use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, Value, value};

use crate::options::{Derived, Options, Target};
use crate::pins::{self, GitRef, Source};

/// How generated manifests reach the marea crates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MareaSource {
    /// A checkout beside the generated project. `prefix` is relative to the
    /// generated workspace root, e.g. `"../marea"`.
    Path {
        prefix: String,
    },
    Git {
        url: String,
        tag: String,
    },
}

/// Everything a manifest builder needs.
pub struct Ctx<'a> {
    pub options: &'a Options,
    pub derived: Derived,
    pub marea: MareaSource,
}

impl<'a> Ctx<'a> {
    pub fn new(options: &'a Options, marea: MareaSource) -> Self {
        Self {
            derived: options.derived(),
            options,
            marea,
        }
    }
}

/// The binary crates a generated workspace can contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCrate {
    Desktop,
    Mobile,
    Web,
}

impl AppCrate {
    pub fn dir_name(self) -> &'static str {
        match self {
            AppCrate::Desktop => "app-desktop",
            AppCrate::Mobile => "app-mobile",
            AppCrate::Web => "app-web",
        }
    }

    /// The cargo feature dx enables for this platform.
    pub fn platform_feature(self) -> &'static str {
        match self {
            AppCrate::Desktop => "desktop",
            AppCrate::Mobile => "mobile",
            AppCrate::Web => "web",
        }
    }
}

/// The app crates implied by the selected targets.
pub fn app_crates(options: &Options) -> Vec<AppCrate> {
    let mut v = Vec::new();
    if options.has_target(Target::Desktop) {
        v.push(AppCrate::Desktop);
    }
    if options.has_mobile() {
        v.push(AppCrate::Mobile);
    }
    if options.has_target(Target::Web) {
        v.push(AppCrate::Web);
    }
    v
}

// ── small builders ───────────────────────────────────────────────────────

/// `version.workspace = true` — a dotted key, matching how the marea and
/// Ascend manifests are written by hand.
fn inherit() -> Item {
    let mut t = InlineTable::new();
    t.insert("workspace", Value::from(true));
    t.set_dotted(true);
    Item::Value(Value::InlineTable(t))
}

fn features_array(features: &[&str]) -> Value {
    let mut arr = Array::new();
    for f in features {
        arr.push(*f);
    }
    Value::Array(arr)
}

/// `name = { workspace = true }`, optionally adding features on top of the
/// workspace entry's own list (cargo unions them).
fn from_workspace(extra_features: &[&str]) -> Item {
    let mut t = InlineTable::new();
    t.insert("workspace", Value::from(true));
    if !extra_features.is_empty() {
        t.insert("features", features_array(extra_features));
    }
    Item::Value(Value::InlineTable(t))
}

fn path_dep(path: &str) -> Item {
    let mut t = InlineTable::new();
    t.insert("path", Value::from(path));
    Item::Value(Value::InlineTable(t))
}

/// A `[workspace.dependencies]` entry built from the pin table, so the source
/// coordinates can only ever come from one place.
fn pin_dep(name: &str, extra_features: &[&str]) -> Item {
    let p = pins::pin(name);
    let mut feats: Vec<&str> = p.features.to_vec();
    for f in extra_features {
        if !feats.contains(f) {
            feats.push(f);
        }
    }

    // The common case — a bare version requirement with nothing to qualify —
    // reads better as `log = "0.4"` than as an inline table.
    if let Source::Version(v) = p.source
        && feats.is_empty()
        && p.default_features.is_none()
    {
        return value(v);
    }

    let mut t = InlineTable::new();
    match p.source {
        Source::Version(v) => {
            t.insert("version", Value::from(v));
        }
        Source::Git { url, git_ref } => {
            t.insert("git", Value::from(url));
            match git_ref {
                GitRef::Branch(b) => t.insert("branch", Value::from(b)),
                GitRef::Rev(r) => t.insert("rev", Value::from(r)),
                GitRef::Tag(tg) => t.insert("tag", Value::from(tg)),
            };
        }
    }
    if let Some(false) = p.default_features {
        t.insert("default-features", Value::from(false));
    }
    if !feats.is_empty() {
        t.insert("features", features_array(&feats));
    }
    Item::Value(Value::InlineTable(t))
}

fn marea_dep(ctx: &Ctx, crate_name: &str, features: &[&str]) -> Item {
    let mut t = InlineTable::new();
    match &ctx.marea {
        MareaSource::Path { prefix } => {
            t.insert("path", Value::from(format!("{prefix}/crates/{crate_name}")));
        }
        MareaSource::Git { url, tag } => {
            t.insert("git", Value::from(url.as_str()));
            t.insert("tag", Value::from(tag.as_str()));
        }
    }
    if !features.is_empty() {
        t.insert("features", features_array(features));
    }
    Item::Value(Value::InlineTable(t))
}

fn multiline_array<I: IntoIterator<Item = String>>(items: I) -> Array {
    let mut arr = Array::new();
    for s in items {
        let mut v = Value::from(s);
        v.decor_mut().set_prefix("\n    ");
        arr.push_formatted(v);
    }
    arr.set_trailing("\n");
    arr.set_trailing_comma(true);
    arr
}

/// Attach an explanatory comment above a key. The comments are the reason a
/// pin exists; a generated app that loses them loses the reason not to bump.
fn comment_key(table: &mut Table, key: &str, text: &str) {
    let prefix: String = text.lines().map(|l| format!("# {l}\n")).collect::<String>();
    if let Some(mut k) = table.key_mut(key) {
        k.leaf_decor_mut().set_prefix(prefix);
    }
}

fn package_table(name: &str, description: &str) -> Table {
    let mut pkg = Table::new();
    pkg["name"] = value(name);
    pkg["description"] = value(description);
    pkg["version"] = inherit();
    pkg["edition"] = inherit();
    pkg["publish"] = value(false);
    pkg
}

/// Fetch (creating if absent) a *header* table at the document root.
///
/// Indexing a missing key and assigning into it — `doc["target"][cfg] = …` —
/// silently produces a top-level inline value (`target = { 'cfg(…)' = { … } }`).
/// That parses, and cargo even accepts it, so structural assertions miss it
/// entirely; it just leaves an unreadable manifest. Creating the parent as an
/// implicit `Table` is what makes the child render as a `[header]` section.
fn ensure_table<'a>(doc: &'a mut DocumentMut, key: &str) -> &'a mut Table {
    if !doc.contains_table(key) {
        let mut t = Table::new();
        t.set_implicit(true);
        doc.insert(key, Item::Table(t));
    }
    doc[key].as_table_mut().expect("just inserted a table")
}

/// `[target.'cfg(...)'.dependencies]`
fn target_deps(doc: &mut DocumentMut, cfg: &str, deps: Table) {
    let mut cfg_table = Table::new();
    cfg_table.set_implicit(true);
    cfg_table.insert("dependencies", Item::Table(deps));
    ensure_table(doc, "target").insert(cfg, Item::Table(cfg_table));
}

fn platform_features(app: AppCrate) -> Table {
    let mut feats = Table::new();
    feats["default"] = value(Array::new());
    feats[app.platform_feature()] = value({
        let mut arr = Array::new();
        arr.push(format!("dioxus/{}", app.platform_feature()));
        arr
    });
    feats
}

// ── the manifests ────────────────────────────────────────────────────────

/// Workspace root `Cargo.toml`: members, the pinned dependency table, and the
/// build profiles.
pub fn workspace_manifest(ctx: &Ctx) -> String {
    let o = ctx.options;
    let mut doc = DocumentMut::new();

    let mut members: Vec<String> = vec!["crates/domain".into()];
    if o.uses_wavesync() {
        members.push("crates/data".into());
    }
    members.push("crates/ui".into());
    members.extend(
        app_crates(o)
            .iter()
            .map(|a| format!("crates/{}", a.dir_name())),
    );

    let mut ws = Table::new();
    ws["resolver"] = value("3");
    ws["members"] = value(multiline_array(members));
    doc["workspace"] = Item::Table(ws);

    let mut pkg = Table::new();
    pkg["version"] = value("0.1.0");
    pkg["edition"] = value("2024");
    // A floor, not a pin. The real constraint is on sea-orm: rc.41 pulls
    // sqlx 0.9 and needs rustc 1.94, so this workspace holds rc.38 — but a
    // newer toolchain builds it fine, and a `rust-toolchain.toml` pinning
    // 1.92 would force every contributor to download that exact compiler
    // forever.
    pkg["rust-version"] = value("1.92");
    doc["workspace"]["package"] = Item::Table(pkg);

    let mut deps = Table::new();

    // Internal crates.
    deps["domain"] = path_dep("crates/domain");
    if o.uses_wavesync() {
        deps["data"] = path_dep("crates/data");
    }
    deps["ui"] = path_dep("crates/ui");

    // The framework.
    let mut ui_feats: Vec<&str> = Vec::new();
    if o.features.pairing {
        ui_feats.push("pairing");
    }
    if o.features.scanner {
        ui_feats.push("scanner");
    }
    if o.features.android_back {
        ui_feats.push("android-back");
    }
    deps["marea-ui"] = marea_dep(ctx, "marea-ui", &ui_feats);
    if o.has_auth() {
        let mut auth_feats = vec!["dioxus"];
        if o.features.pairing {
            auth_feats.push("pairing");
        }
        deps["marea-auth"] = marea_dep(ctx, "marea-auth", &auth_feats);
    }
    if o.uses_wavesync() {
        deps["marea-sync"] = marea_dep(ctx, "marea-sync", &[]);
    }
    if matches!(ctx.marea, MareaSource::Path { .. }) {
        comment_key(
            &mut deps,
            "marea-ui",
            "TODO(marea): switch to { git = \"…\", tag = \"v0.1.0\" } once published.",
        );
    }

    // Pinned externals. Only what the selected options actually use.
    deps["dioxus"] = pin_dep("dioxus", &[]);
    if o.has_native() {
        deps["dioxus-sdk-storage"] = pin_dep("dioxus-sdk-storage", &[]);
        comment_key(
            &mut deps,
            "dioxus-sdk-storage",
            pins::pin("dioxus-sdk-storage").comment.unwrap_or_default(),
        );
    }
    if o.uses_wavesync() {
        let ws_feats: &[&str] = if o.has_target(Target::Web) {
            &["web"]
        } else {
            &[]
        };
        deps["wavesyncdb"] = pin_dep("wavesyncdb", ws_feats);
        comment_key(
            &mut deps,
            "wavesyncdb",
            pins::pin("wavesyncdb").comment.unwrap_or_default(),
        );
        deps["sea-orm"] = pin_dep("sea-orm", &[]);
        comment_key(
            &mut deps,
            "sea-orm",
            pins::pin("sea-orm").comment.unwrap_or_default(),
        );
        deps["uuid"] = pin_dep("uuid", &[]);
    }
    deps["serde"] = pin_dep("serde", &[]);
    deps["chrono"] = pin_dep("chrono", &[]);
    deps["thiserror"] = pin_dep("thiserror", &[]);
    deps["log"] = pin_dep("log", &[]);
    if o.has_target(Target::Desktop) {
        deps["tracing-log"] = pin_dep("tracing-log", &[]);
        comment_key(
            &mut deps,
            "tracing-log",
            pins::pin("tracing-log").comment.unwrap_or_default(),
        );
    }
    if o.has_target(Target::Android) {
        deps["android_logger"] = pin_dep("android_logger", &[]);
        comment_key(
            &mut deps,
            "android_logger",
            pins::pin("android_logger").comment.unwrap_or_default(),
        );
    }
    if o.has_target(Target::Web) {
        deps["getrandom"] = pin_dep("getrandom", &[]);
        comment_key(
            &mut deps,
            "getrandom",
            pins::pin("getrandom").comment.unwrap_or_default(),
        );
        deps["web-sys"] = pin_dep("web-sys", &[]);
        deps["wasm-logger"] = pin_dep("wasm-logger", &[]);
        deps["console_error_panic_hook"] = pin_dep("console_error_panic_hook", &[]);
    }

    doc["workspace"]["dependencies"] = Item::Table(deps);

    // Profiles. Each one is here because a real build broke without it.
    let web = o.has_target(Target::Web);
    if o.has_mobile() || web {
        let mut dev = Table::new();
        if o.has_mobile() {
            dev["strip"] = value("debuginfo");
            dev.decor_mut().set_prefix(
                "\n# Strip DWARF from dev binaries. An Android cdylib weighs ~520 MB per ABI\n\
                 # with debug info; stripping cuts it to ~170 MB while keeping function\n\
                 # names in panic backtraces. Costs nothing — strip runs at link time.\n",
            );
        } else {
            // No direct keys of its own: keep the bare `[profile.dev]` header
            // out of the manifest.
            dev.set_implicit(true);
        }
        if web {
            let mut star = Table::new();
            star["opt-level"] = value(1);
            star.decor_mut().set_prefix(
                "\n# wasm dev builds at opt-level 0 run 5-10x slower because the allocator,\n\
                 # string ops and HashMap internals don't inline. Level 1 adds ~3s to\n\
                 # incremental builds and makes the browser usable.\n",
            );
            let mut package = Table::new();
            package.set_implicit(true);
            package.insert("*", Item::Table(star));
            dev.insert("package", Item::Table(package));
        }

        let mut rel = Table::new();
        rel["strip"] = value("debuginfo");
        rel.decor_mut().set_prefix(
            "\n# Drop DWARF before wasm-opt runs — rustc's debug sections trip binaryen's\n\
             # \"unsupported version of DWARF\" abort, which silently downgrades the web\n\
             # build to UNOPTIMIZED wasm.\n",
        );

        let profile = ensure_table(&mut doc, "profile");
        profile.insert("dev", Item::Table(dev));
        profile.insert("release", Item::Table(rel));
    }

    doc.to_string()
}

/// `crates/domain/Cargo.toml` — pure Rust, no dioxus, no database.
pub fn domain_manifest(ctx: &Ctx) -> String {
    let mut doc = DocumentMut::new();
    doc["package"] = Item::Table(package_table(
        "domain",
        &format!(
            "{} domain model: types and rules, independent of UI and storage",
            ctx.derived.title
        ),
    ));

    let mut deps = Table::new();
    deps["serde"] = from_workspace(&[]);
    deps["chrono"] = from_workspace(&[]);
    deps["thiserror"] = from_workspace(&[]);
    deps.decor_mut().set_prefix(
        "\n# Nothing here may depend on dioxus, sea-orm or wavesyncdb. `domain` is the\n\
         # part of the app that survives a rewrite of everything else, and CI asserts\n\
         # the boundary with `cargo tree -p domain`.\n",
    );
    doc["dependencies"] = Item::Table(deps);

    doc.to_string()
}

/// `crates/data/Cargo.toml` — entities, repos, and both bootstrap paths.
pub fn data_manifest(ctx: &Ctx) -> String {
    let o = ctx.options;
    let mut doc = DocumentMut::new();
    doc["package"] = Item::Table(package_table(
        "data",
        &format!(
            "{} persistence: synced entities, typed repositories, sync bootstrap",
            ctx.derived.title
        ),
    ));

    let mut deps = Table::new();
    deps["domain"] = from_workspace(&[]);
    deps["dioxus"] = from_workspace(&[]);
    // The sync passphrase is derived from the account, so the bootstrap needs
    // the session. This is also why `Options::validate` rejects sync without
    // auth: there would be no key material.
    deps["marea-auth"] = from_workspace(&[]);
    deps["marea-sync"] = from_workspace(&[]);
    deps["wavesyncdb"] = from_workspace(&[]);
    deps["serde"] = from_workspace(&[]);
    deps["uuid"] = from_workspace(&[]);
    deps["chrono"] = from_workspace(&[]);
    deps["thiserror"] = from_workspace(&[]);
    deps["log"] = from_workspace(&[]);
    comment_key(
        &mut deps,
        "wavesyncdb",
        "Declared directly, not reached through `marea_sync::wavesyncdb`: the\n\
         derive macro expands to absolute `::wavesyncdb::…` paths and offers no\n\
         crate-path override. The workspace pins one spec, so this still\n\
         resolves to the single copy marea-sync uses.",
    );
    doc["dependencies"] = Item::Table(deps);

    let mut native = Table::new();
    native["sea-orm"] = from_workspace(&[]);
    native["dioxus-sdk-storage"] = from_workspace(&[]);
    target_deps(&mut doc, r#"cfg(not(target_arch = "wasm32"))"#, native);

    if o.has_target(Target::Web) {
        let mut wasm = Table::new();
        wasm["wavesyncdb"] = from_workspace(&["web"]);
        wasm["getrandom"] = from_workspace(&[]);
        // Reading `?relay=` off the page URL.
        wasm["web-sys"] = from_workspace(&[]);
        target_deps(&mut doc, r#"cfg(target_arch = "wasm32")"#, wasm);
    }

    doc.to_string()
}

/// `crates/ui/Cargo.toml` — screens for every target, no engine vocabulary.
pub fn ui_manifest(ctx: &Ctx) -> String {
    let o = ctx.options;
    let mut doc = DocumentMut::new();
    doc["package"] = Item::Table(package_table(
        "ui",
        &format!(
            "{} screens, routes and shell — one set for every target",
            ctx.derived.title
        ),
    ));

    let mut deps = Table::new();
    deps["dioxus"] = from_workspace(&[]);
    deps["domain"] = from_workspace(&[]);
    if o.uses_wavesync() {
        deps["data"] = from_workspace(&[]);
        // Screens mint the string UUID primary keys synced rows require.
        deps["uuid"] = from_workspace(&[]);
    }
    deps["marea-ui"] = from_workspace(&[]);
    if o.has_auth() {
        deps["marea-auth"] = from_workspace(&[]);
    }
    deps["serde"] = from_workspace(&[]);
    deps["log"] = from_workspace(&[]);
    deps.decor_mut().set_prefix(
        "\n# No wavesyncdb, no sea-orm, and no `#[cfg(target_arch)]` anywhere under\n\
         # src/: screens reach data through `data`'s repositories, which are the same\n\
         # API on native and in the browser. CI asserts this.\n",
    );
    doc["dependencies"] = Item::Table(deps);

    doc.to_string()
}

/// `crates/app-*/Cargo.toml`.
pub fn app_manifest(ctx: &Ctx, app: AppCrate) -> String {
    let o = ctx.options;
    let mut doc = DocumentMut::new();
    let platform = match app {
        AppCrate::Desktop => "desktop",
        AppCrate::Mobile => "mobile",
        AppCrate::Web => "web",
    };
    doc["package"] = Item::Table(package_table(
        app.dir_name(),
        &format!("{} {platform} binary — launcher only", ctx.derived.title),
    ));

    if app == AppCrate::Mobile {
        // dx derives `<application android:label>` from the executable
        // target's name, so without this the home-screen label would read
        // "App Mobile". The package name stays `app-mobile` so
        // `dx serve --package app-mobile` still selects it.
        let mut bin = Table::new();
        bin["name"] = value(&ctx.derived.pascal);
        bin["path"] = value("src/main.rs");
        bin.decor_mut().set_prefix(
            "\n# The binary name becomes the Android home-screen label (dx derives\n\
             # `<application android:label>` from the executable target). Without this\n\
             # the launcher would read \"App Mobile\".\n",
        );
        let mut bins = ArrayOfTables::new();
        bins.push(bin);
        doc["bin"] = Item::ArrayOfTables(bins);
    }

    let mut deps = Table::new();
    deps["dioxus"] = from_workspace(&[]);
    deps["ui"] = from_workspace(&[]);
    match app {
        AppCrate::Desktop => {
            deps["dioxus-sdk-storage"] = from_workspace(&[]);
            deps["tracing-log"] = from_workspace(&[]);
        }
        AppCrate::Mobile => {
            deps["dioxus-sdk-storage"] = from_workspace(&[]);
            deps["log"] = from_workspace(&[]);
        }
        AppCrate::Web => {
            if o.features.pairing {
                // The browser's entry screen is marea's QR pairing flow,
                // mounted here rather than in `ui` because `PairScreen` is
                // wasm-only and `ui` stays target-agnostic.
                deps["marea-ui"] = from_workspace(&[]);
            }
        }
    }
    doc["dependencies"] = Item::Table(deps);

    if app == AppCrate::Mobile && o.has_target(Target::Android) {
        let mut android = Table::new();
        android["android_logger"] = from_workspace(&[]);
        target_deps(&mut doc, r#"cfg(target_os = "android")"#, android);
    }
    if app == AppCrate::Web {
        let mut wasm = Table::new();
        wasm["console_error_panic_hook"] = from_workspace(&[]);
        wasm["wasm-logger"] = from_workspace(&[]);
        wasm["getrandom"] = from_workspace(&[]);
        target_deps(&mut doc, r#"cfg(target_arch = "wasm32")"#, wasm);
    }

    doc["features"] = Item::Table(platform_features(app));

    doc.to_string()
}

/// `crates/app-*/Dioxus.toml`.
///
/// One per app crate, not one at the workspace root: dx reads the file next to
/// the crate it is serving, and both the Tailwind paths and the bundle
/// identifier are per-binary settings.
pub fn dioxus_toml(ctx: &Ctx, app: AppCrate) -> String {
    let o = ctx.options;
    let mut doc = DocumentMut::new();

    let mut application = Table::new();
    if o.tailwind {
        // dx 0.7 compiles Tailwind itself on serve/build. Input is the shared
        // source in `ui`; output lands in this crate's assets, where
        // `asset!("/assets/tailwind.css")` picks it up.
        // Output lands in `ui/assets`, not this crate's: the
        // `asset!("/assets/tailwind.css")` that loads it is expanded in `ui`,
        // and `asset!` resolves relative to the crate it appears in. Writing
        // it here instead would ship the uncompiled source to the webview.
        application["tailwind_input"] = value("../ui/assets/tailwind.in.css");
        application["tailwind_output"] = value("../ui/assets/tailwind.css");
    }
    doc["application"] = Item::Table(application);

    if let Some(id) = ctx.options.bundle_id.as_deref() {
        let mut bundle = Table::new();
        bundle["identifier"] = value(id);
        bundle.decor_mut().set_prefix(
            "\n# Reverse-DNS app id: `applicationId` on Android, the bundle id on\n\
             # iOS and desktop. PERMANENT once published — a Play Store\n\
             # applicationId can never be changed. Without it dx falls back to\n\
             # `com.example.…`.\n",
        );
        doc["bundle"] = Item::Table(bundle);
    }

    if app == AppCrate::Web {
        let mut web_app = Table::new();
        web_app["title"] = value(&ctx.derived.title);
        ensure_table(&mut doc, "web").insert("app", Item::Table(web_app));
    }

    doc.to_string()
}

pub fn cargo_config(ctx: &Ctx) -> String {
    let mut doc = DocumentMut::new();
    if ctx.options.has_target(Target::Web) {
        let mut t = Table::new();
        let mut flags = Array::new();
        flags.push(r#"--cfg=getrandom_backend="wasm_js""#);
        t["rustflags"] = value(flags);
        t.decor_mut().set_prefix(
            "# Selects the browser entropy backend for the `getrandom 0.3` pulled in\n\
             # transitively. The `js` feature on our own `getrandom 0.2` covers the\n\
             # direct dependency; this rustflag covers the rest.\n",
        );
        ensure_table(&mut doc, "target").insert("wasm32-unknown-unknown", Item::Table(t));
    }
    doc.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{Auth, Engine};
    use std::collections::BTreeSet;

    fn parse(s: &str) -> DocumentMut {
        s.parse()
            .unwrap_or_else(|e| panic!("generated manifest does not parse: {e}\n---\n{s}"))
    }

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

    fn synced() -> Options {
        Options {
            targets: [Target::Android, Target::Web]
                .into_iter()
                .collect::<BTreeSet<_>>(),
            bundle_id: Some("es.almares.myapp".into()),
            engine: Engine::WaveSync {
                relay: Some("/dns4/relay.example/udp/4011/quic-v1/p2p/12D3Koo".into()),
            },
            ..minimal()
        }
    }

    fn ctx(o: &Options) -> Ctx<'_> {
        Ctx::new(
            o,
            MareaSource::Path {
                prefix: "../marea".into(),
            },
        )
    }

    fn git_ctx(o: &Options) -> Ctx<'_> {
        Ctx::new(
            o,
            MareaSource::Git {
                url: "https://github.com/pvg13/marea".into(),
                tag: "v0.1.0".into(),
            },
        )
    }

    fn dep_names(doc: &DocumentMut, table: &str) -> Vec<String> {
        doc.get(table)
            .and_then(|t| t.as_table())
            .map(|t| t.iter().map(|(k, _)| k.to_string()).collect())
            .unwrap_or_default()
    }

    // ── workspace ────────────────────────────────────────────────────────

    #[test]
    fn workspace_members_follow_the_selected_targets() {
        let o = minimal();
        let doc = parse(&workspace_manifest(&ctx(&o)));
        let members: Vec<String> = doc["workspace"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            members,
            vec!["crates/domain", "crates/ui", "crates/app-desktop"]
        );
    }

    #[test]
    fn a_synced_multi_target_workspace_gains_data_and_both_app_crates() {
        let o = synced();
        let doc = parse(&workspace_manifest(&ctx(&o)));
        let members: Vec<String> = doc["workspace"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            members,
            vec![
                "crates/domain",
                "crates/data",
                "crates/ui",
                "crates/app-mobile",
                "crates/app-web",
            ]
        );
    }

    #[test]
    fn marea_crates_are_path_deps_when_a_sibling_checkout_exists() {
        let o = minimal();
        let doc = parse(&workspace_manifest(&ctx(&o)));
        let dep = &doc["workspace"]["dependencies"]["marea-ui"];
        assert_eq!(
            dep.get("path").and_then(|p| p.as_str()),
            Some("../marea/crates/marea-ui")
        );
    }

    #[test]
    fn marea_crates_are_git_tag_deps_without_a_sibling_checkout() {
        let o = minimal();
        let doc = parse(&workspace_manifest(&git_ctx(&o)));
        let dep = &doc["workspace"]["dependencies"]["marea-ui"];
        assert_eq!(
            dep.get("git").and_then(|g| g.as_str()),
            Some("https://github.com/pvg13/marea")
        );
        assert_eq!(dep.get("tag").and_then(|t| t.as_str()), Some("v0.1.0"));
    }

    #[test]
    fn engine_crates_appear_only_when_wavesync_is_selected() {
        let plain = minimal();
        let names = dep_names(&parse(&workspace_manifest(&ctx(&plain))), "workspace");
        let _ = names; // members table, not deps — checked below
        let doc = parse(&workspace_manifest(&ctx(&plain)));
        let deps = doc["workspace"]["dependencies"].as_table().unwrap();
        assert!(deps.get("wavesyncdb").is_none());
        assert!(deps.get("marea-sync").is_none());
        assert!(deps.get("sea-orm").is_none());

        let o = synced();
        let doc = parse(&workspace_manifest(&ctx(&o)));
        let deps = doc["workspace"]["dependencies"].as_table().unwrap();
        assert!(deps.get("wavesyncdb").is_some());
        assert!(deps.get("marea-sync").is_some());
        assert!(deps.get("sea-orm").is_some());
    }

    #[test]
    fn pinned_specs_are_copied_verbatim_from_the_pin_table() {
        let o = synced();
        let doc = parse(&workspace_manifest(&ctx(&o)));
        let deps = &doc["workspace"]["dependencies"];
        assert_eq!(
            pins::coords_of(&deps["wavesyncdb"]),
            Some(pins::pin("wavesyncdb").source.coords())
        );
        assert_eq!(
            pins::coords_of(&deps["dioxus-sdk-storage"]),
            Some(pins::pin("dioxus-sdk-storage").source.coords())
        );
    }

    // ── per-crate ────────────────────────────────────────────────────────

    #[test]
    fn domain_declares_no_ui_or_database_dependency() {
        let o = synced();
        let doc = parse(&domain_manifest(&ctx(&o)));
        let deps = dep_names(&doc, "dependencies");
        for forbidden in ["dioxus", "sea-orm", "wavesyncdb", "marea-ui", "data"] {
            assert!(
                !deps.iter().any(|d| d == forbidden),
                "`domain` must stay pure Rust but declares {forbidden}: {deps:?}"
            );
        }
    }

    #[test]
    fn data_declares_wavesyncdb_directly_for_the_derive_macro() {
        // The derive expands to absolute `::wavesyncdb::…` paths with no
        // crate-path override, so reaching it only through the marea-sync
        // re-export would not compile.
        let o = synced();
        let doc = parse(&data_manifest(&ctx(&o)));
        assert!(
            dep_names(&doc, "dependencies")
                .iter()
                .any(|d| d == "wavesyncdb")
        );
    }

    #[test]
    fn data_keeps_sea_orm_on_the_native_side_only() {
        let o = synced();
        let doc = parse(&data_manifest(&ctx(&o)));
        assert!(
            !dep_names(&doc, "dependencies")
                .iter()
                .any(|d| d == "sea-orm")
        );
        let native = &doc["target"][r#"cfg(not(target_arch = "wasm32"))"#]["dependencies"];
        assert!(
            native.get("sea-orm").is_some(),
            "sea-orm should be native-only"
        );
    }

    #[test]
    fn ui_never_names_the_sync_engine() {
        let o = synced();
        let text = ui_manifest(&ctx(&o));
        let doc = parse(&text);
        for table in ["dependencies", "dev-dependencies"] {
            for d in dep_names(&doc, table) {
                assert!(
                    d != "wavesyncdb" && d != "sea-orm",
                    "`ui` must not know the engine, found {d}"
                );
            }
        }
        assert!(dep_names(&doc, "dependencies").iter().any(|d| d == "data"));
    }

    #[test]
    fn the_mobile_crate_renames_its_binary_for_the_android_launcher_label() {
        // dx derives `<application android:label>` from the executable
        // target's name; without this the home screen would read "App Mobile".
        let o = synced();
        let doc = parse(&app_manifest(&ctx(&o), AppCrate::Mobile));
        let bin = doc["bin"].as_array_of_tables().unwrap().get(0).unwrap();
        assert_eq!(bin["name"].as_str(), Some("MyApp"));
    }

    #[test]
    fn each_app_crate_declares_its_platform_feature_and_defaults_to_none() {
        let o = synced();
        for app in [AppCrate::Mobile, AppCrate::Web] {
            let doc = parse(&app_manifest(&ctx(&o), app));
            let feats = doc["features"].as_table().unwrap();
            assert!(
                feats.get(app.platform_feature()).is_some(),
                "{} should declare the `{}` feature",
                app.dir_name(),
                app.platform_feature()
            );
            assert_eq!(feats["default"].as_array().unwrap().len(), 0);
        }
    }

    #[test]
    fn the_web_crate_puts_browser_glue_behind_the_wasm_cfg() {
        let o = synced();
        let doc = parse(&app_manifest(&ctx(&o), AppCrate::Web));
        let wasm = &doc["target"][r#"cfg(target_arch = "wasm32")"#]["dependencies"];
        assert!(wasm.get("console_error_panic_hook").is_some());
        assert!(wasm.get("wasm-logger").is_some());
    }

    // ── the other generated manifests ────────────────────────────────────

    #[test]
    fn the_web_crates_dioxus_toml_carries_the_page_title() {
        let o = synced();
        let doc = parse(&dioxus_toml(&ctx(&o), AppCrate::Web));
        assert_eq!(doc["web"]["app"]["title"].as_str(), Some("My App"));
    }

    /// dx derives the Android applicationId from this, and it can never be
    /// changed after publishing.
    #[test]
    fn the_mobile_crate_declares_the_bundle_identifier() {
        let mut o = synced();
        o.bundle_id = Some("es.almares.myapp".into());
        let text = dioxus_toml(&ctx(&o), AppCrate::Mobile);
        let doc = parse(&text);
        assert_eq!(
            doc["bundle"]["identifier"].as_str(),
            Some("es.almares.myapp")
        );
        assert!(
            text.contains("[bundle]"),
            "expected a section, got:\n{text}"
        );
    }

    #[test]
    fn tailwind_paths_appear_only_when_tailwind_was_chosen() {
        let mut o = synced();
        assert!(
            parse(&dioxus_toml(&ctx(&o), AppCrate::Web))["application"]
                .get("tailwind_input")
                .is_none()
        );

        o.tailwind = true;
        let doc = parse(&dioxus_toml(&ctx(&o), AppCrate::Web));
        assert_eq!(
            doc["application"]["tailwind_input"].as_str(),
            Some("../ui/assets/tailwind.in.css")
        );
        // The compiled file must land where `ui`'s `asset!` looks for it.
        assert_eq!(
            doc["application"]["tailwind_output"].as_str(),
            Some("../ui/assets/tailwind.css")
        );
    }

    #[test]
    fn cargo_config_selects_the_wasm_entropy_backend_for_web_builds() {
        let o = synced();
        let doc = parse(&cargo_config(&ctx(&o)));
        let flags = doc["target"]["wasm32-unknown-unknown"]["rustflags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert!(
            flags.iter().any(|f| f.contains("getrandom_backend")),
            "expected the getrandom wasm backend rustflag, got {flags:?}"
        );
    }

    // Structural assertions pass even when a table is emitted as a top-level
    // inline value (`profile = { dev.strip = ... }`), which cargo reads but no
    // human wants to edit. These assert the rendered shape.

    #[test]
    fn profiles_render_as_sections_not_as_an_inline_table() {
        let o = synced();
        let text = workspace_manifest(&ctx(&o));
        assert!(
            text.contains("[profile.dev]"),
            "expected a [profile.dev] section, got:\n{text}"
        );
        assert!(
            text.contains(r#"[profile.dev.package."*"]"#),
            "expected a [profile.dev.package.\"*\"] section, got:\n{text}"
        );
        assert!(
            text.contains("[profile.release]"),
            "expected a [profile.release] section, got:\n{text}"
        );
        assert!(
            !text.starts_with("profile ="),
            "profiles leaked out as a top-level inline table:\n{text}"
        );
    }

    #[test]
    fn target_dependencies_render_as_sections() {
        let o = synced();
        let text = data_manifest(&ctx(&o));
        assert!(
            text.contains(r#"[target.'cfg(not(target_arch = "wasm32"))'.dependencies]"#),
            "expected a native target section, got:\n{text}"
        );
        assert!(
            !text.starts_with("target ="),
            "target deps leaked out as a top-level inline table:\n{text}"
        );
    }

    #[test]
    fn a_web_only_workspace_still_gets_the_wasm_opt_level_without_an_empty_dev_profile() {
        let mut o = minimal();
        o.targets = [Target::Web].into_iter().collect();
        o.engine = Engine::WaveSync {
            relay: Some("/dns4/relay.example/udp/4011/quic-v1/p2p/12D3Koo".into()),
        };
        let text = workspace_manifest(&ctx(&o));
        assert!(text.contains(r#"[profile.dev.package."*"]"#));
        assert!(
            !text.contains("[profile.dev]\n\n") && !text.contains("[profile.dev]\n["),
            "empty [profile.dev] header emitted:\n{text}"
        );
    }

    #[test]
    fn dioxus_toml_renders_the_web_app_table_as_a_section() {
        let o = synced();
        let text = dioxus_toml(&ctx(&o), AppCrate::Web);
        assert!(
            text.contains("[web.app]"),
            "expected a [web.app] section, got:\n{text}"
        );
    }

    #[test]
    fn the_workspace_declares_a_rust_version_floor_without_pinning_a_toolchain() {
        let o = minimal();
        let doc = parse(&workspace_manifest(&ctx(&o)));
        assert_eq!(
            doc["workspace"]["package"]["rust-version"].as_str(),
            Some("1.92")
        );
    }
}
