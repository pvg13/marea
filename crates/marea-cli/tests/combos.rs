//! Generation-level tests across the option combinations that matter.
//!
//! These stop short of compiling the output — that takes minutes and needs a
//! network — but they catch the failures that are cheap to catch: a template
//! that stops rendering, a condition that stops firing, a file that appears in
//! a combo it has no business being in, or a rule the generated code is
//! supposed to follow and quietly stopped following.
//!
//! Compiling every combo is the CI matrix's job (`.github/workflows/`), and
//! `marea new` itself compiles what it writes unless told not to.

use std::collections::BTreeSet;

use marea_cli::manifest::MareaSource;
use marea_cli::options::{Auth, Engine, Options, Target};
use marea_cli::render::{Plan, plan};

fn marea() -> MareaSource {
    MareaSource::Path {
        prefix: "../marea".into(),
    }
}

fn generate(o: &Options) -> Plan {
    o.validate()
        .unwrap_or_else(|e| panic!("combo should be valid, got {e:?}"));
    plan(o, marea()).expect("planning succeeds")
}

fn auth() -> Option<Auth> {
    Some(Auth {
        pocketbase_url: "https://admin.almares.es".into(),
    })
}

fn wavesync() -> Engine {
    Engine::WaveSync {
        relay: Some("/dns4/relay.example/tcp/443/wss/p2p/12D3Koo".into()),
    }
}

/// Auth, one target, no engine.
fn minimal() -> Options {
    Options {
        name: "my-app".into(),
        targets: [Target::Desktop].into_iter().collect::<BTreeSet<_>>(),
        auth: auth(),
        with_example: false,
        ..Default::default()
    }
}

/// The shape most real apps in this ecosystem have.
fn synced() -> Options {
    Options {
        targets: [Target::Android, Target::Web]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        engine: wavesync(),
        bundle_id: Some("es.almares.myapp".into()),
        with_example: true,
        ..minimal()
    }
}

/// Everything on.
fn full() -> Options {
    let mut o = synced();
    o.targets = [Target::Desktop, Target::Android, Target::Ios, Target::Web]
        .into_iter()
        .collect();
    o.tailwind = true;
    o.features.pairing = true;
    o.features.scanner = true;
    o.features.push = true;
    o.features.android_back = true;
    o.features.locales = vec!["es".into(), "en".into()];
    o.infra.ci = true;
    o.infra.containers = true;
    o.scaffolding.agent_docs = true;
    o.scaffolding.clippy = true;
    o.scaffolding.docs = true;
    o.scaffolding.e2e = true;
    o
}

/// No accounts, no engine, browser only.
fn no_auth() -> Options {
    Options {
        name: "my-app".into(),
        targets: [Target::Web].into_iter().collect::<BTreeSet<_>>(),
        auth: None,
        engine: Engine::None,
        with_example: false,
        ..Default::default()
    }
}

fn combos() -> Vec<(&'static str, Options)> {
    vec![
        ("minimal", minimal()),
        ("synced", synced()),
        ("full", full()),
        ("no-auth", no_auth()),
    ]
}

#[test]
fn every_combo_renders() {
    for (name, o) in combos() {
        let p = generate(&o);
        assert!(!p.files.is_empty(), "{name} produced no files");
    }
}

/// A stray `{{` in a template renders as nothing useful and compiles as
/// nothing at all.
#[test]
fn no_combo_leaks_template_syntax() {
    for (name, o) in combos() {
        for f in generate(&o).files {
            assert!(
                !f.contents.contains("{{") && !f.contents.contains("{%"),
                "{name}: {} still contains unrendered template syntax",
                f.path
            );
        }
    }
}

/// The rule the whole layering rests on. Checked on the generated text rather
/// than only in CI, so a template can never introduce the first violation.
#[test]
fn no_combo_puts_a_target_cfg_in_the_ui_crate() {
    for (name, o) in combos() {
        for f in generate(&o).files {
            if !f.path.starts_with("crates/ui/src") {
                continue;
            }
            for (n, line) in f.contents.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") || code.starts_with('*') {
                    continue;
                }
                assert!(
                    !code.contains("#[cfg(target_arch")
                        && !code.contains("#[cfg(not(target_arch")
                        && !code.contains("cfg!(target_arch"),
                    "{name}: {}:{} has a target cfg in `ui`",
                    f.path,
                    n + 1
                );
            }
        }
    }
}

/// `ui` reaching the engine directly is how screens start knowing about sync.
#[test]
fn no_combo_lets_the_ui_crate_name_the_engine() {
    for (name, o) in combos() {
        for f in generate(&o).files {
            if f.path != "crates/ui/Cargo.toml" {
                continue;
            }
            // Parse rather than grep: the manifest's own comments explain
            // this very rule, and matching raw text flags the explanation.
            let doc: toml_edit::DocumentMut = f.contents.parse().expect("ui manifest parses");
            let deps = doc["dependencies"].as_table().expect("[dependencies]");
            for forbidden in ["wavesyncdb", "sea-orm"] {
                assert!(
                    deps.get(forbidden).is_none(),
                    "{name}: ui depends on {forbidden}"
                );
            }
        }
    }
}

/// Every manifest must parse — a broken one fails at `cargo check`, far from
/// the template that produced it.
#[test]
fn every_generated_manifest_parses() {
    for (name, o) in combos() {
        for f in generate(&o).files {
            if !f.path.ends_with(".toml") {
                continue;
            }
            f.contents
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_else(|e| {
                    panic!(
                        "{name}: {} does not parse: {e}\n---\n{}",
                        f.path, f.contents
                    )
                });
        }
    }
}

/// The compatibility constants must reach the files that own them, in every
/// combo that has them. Getting one wrong is unrecoverable after release.
#[test]
fn the_permanent_constants_reach_the_generated_code() {
    let p = generate(&synced());

    let app = &p.get("crates/ui/src/app.rs").expect("app.rs").contents;
    assert!(app.contains(r#"b"my-app.psk.v1|""#), "psk domain missing");
    assert!(app.contains("my_app_auth_session"), "session key missing");

    let boot = &p
        .get("crates/data/src/bootstrap/native.rs")
        .expect("native bootstrap")
        .contents;
    assert!(
        boot.contains(r#"DbLocator::new("MyApp", "my_app.db")"#),
        "locator missing"
    );
    assert!(boot.contains(r#""my-app""#), "sync topic missing");

    let web = &p
        .get("crates/data/src/bootstrap/web.rs")
        .expect("web bootstrap")
        .contents;
    assert!(
        web.contains(r#"const SYNC_TOPIC: &str = "my-app""#),
        "the web build must use the same topic as native, or the two never meet"
    );
}

#[test]
fn a_combo_only_contains_what_it_asked_for() {
    let files = |o: &Options| -> Vec<String> {
        generate(o).paths().iter().map(|s| s.to_string()).collect()
    };

    let m = files(&minimal());
    assert!(!m.iter().any(|f| f.starts_with("crates/data/")));
    assert!(!m.iter().any(|f| f.starts_with("crates/app-web/")));
    assert!(!m.iter().any(|f| f == "CLAUDE.md"));
    assert!(!m.iter().any(|f| f.ends_with("tailwind.css")));

    let f = files(&full());
    for expected in [
        "crates/data/src/bootstrap/web.rs",
        "crates/app-mobile/Dioxus.toml",
        "crates/ui/assets/tailwind.css",
        "CLAUDE.md",
        "clippy.toml",
        "docs/architecture.md",
        ".github/workflows/ci.yml",
        "Dockerfile",
        "DEPLOY.md",
        "maestro/smoke.yaml",
    ] {
        assert!(
            f.contains(&expected.to_string()),
            "full combo is missing {expected}"
        );
    }
}

/// The sample slice is scaffolding, and has to read as scaffolding.
#[test]
fn every_sample_file_says_it_is_deletable() {
    let p = generate(&synced());
    for f in p.files {
        let is_sample = f.path.contains("/item")
            || f.path.contains("/items")
            || f.path.ends_with("entities/item.rs");
        if is_sample && f.path.ends_with(".rs") {
            assert!(
                f.contents.contains("DELETE ME"),
                "{} is part of the sample slice but does not say so",
                f.path
            );
        }
    }
}

/// dx would otherwise publish the app as `com.example.…`, permanently.
#[test]
fn every_mobile_combo_carries_its_bundle_identifier() {
    for (name, o) in combos() {
        if !o.has_mobile() {
            continue;
        }
        let p = generate(&o);
        let toml = &p
            .get("crates/app-mobile/Dioxus.toml")
            .unwrap_or_else(|| panic!("{name}: mobile combo has no Dioxus.toml"))
            .contents;
        assert!(
            toml.contains("[bundle]") && toml.contains("identifier"),
            "{name}: mobile Dioxus.toml has no bundle identifier"
        );
    }
}
