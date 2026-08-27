//! The interactive wizard.
//!
//! Two rules it follows, both learned the hard way:
//!
//! **Validate each answer where it is given.** Collecting fourteen answers and
//! then reporting that the first one was malformed wastes the whole wizard and
//! tells the user nothing they could have acted on earlier. Every prompt here
//! carries the validator for its own field, so a bad answer is rejected on the
//! spot, with the cursor still on the question.
//!
//! **Never ask a question whose answer is already implied.** No pairing
//! question without both a phone and a browser target; no relay question
//! without a web build; no bundle identifier without a mobile target. The
//! cross-field rules in [`Options::validate`] are therefore unreachable from
//! this path — it stays as the guarantee for `--config`, which has no wizard
//! to enforce them.

use std::collections::BTreeSet;

use anyhow::Result;
use inquire::list_option::ListOption;
use inquire::validator::Validation;
use inquire::{MultiSelect, Select, Text};

use crate::options::{self, Auth, Engine, Options, Target};

const DEFAULT_POCKETBASE: &str = "https://admin.almares.es";

/// A yes/no question as a two-item list rather than a typed `y/n`.
///
/// Matches `dx new`: arrow keys and enter, with the default highlighted, so
/// the whole wizard is answered the same way instead of switching between
/// typing and selecting.
fn confirm(message: &str, default: bool) -> Result<bool> {
    let answer = Select::new(message, vec!["Yes", "No"])
        .with_starting_cursor(if default { 0 } else { 1 })
        .prompt()?;
    Ok(answer == "Yes")
}

// `+ Clone` is required: inquire's `StringValidator` is object-safe via
// `DynClone`, and an opaque `impl Fn` does not expose `Clone` without it.
fn non_empty(
    label: &'static str,
) -> impl Fn(&str) -> Result<Validation, inquire::CustomUserError> + Clone {
    move |input: &str| {
        if input.trim().is_empty() {
            Ok(Validation::Invalid(label.into()))
        } else {
            Ok(Validation::Valid)
        }
    }
}

/// `default_name` is the slug derived from the target directory, so
/// `marea new Roommates` offers `roommates` rather than rejecting it.
pub fn ask(default_name: &str) -> Result<Options> {
    let mut o = Options::default();

    // ── name ─────────────────────────────────────────────────────────────
    // Normalised rather than rejected. `Roommates` is what a person types, and
    // a tool that can derive `roommates` from it should, instead of bouncing
    // the answer back — especially since inquire keeps the rejected text in
    // the buffer, so the user then has to clear it by hand. Only genuinely
    // unusable input (nothing left after slugifying) is refused.
    let typed = Text::new("Project name")
        .with_default(default_name)
        .with_help_message("becomes crate names and permanent identifiers (kebab-case)")
        .with_validator(|input: &str| {
            if options::slugify(input).is_empty() {
                Ok(Validation::Invalid(
                    "needs at least one letter (e.g. my-app)".into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()?;
    o.name = options::slugify(&typed);
    if o.name != typed.trim() {
        // Never silently: the name becomes permanent identifiers.
        println!("  → project name: {}", o.name);
    }

    // ── targets ──────────────────────────────────────────────────────────
    let chosen = MultiSelect::new(
        "Which targets should this app ship?",
        vec!["desktop", "android", "ios", "web"],
    )
    .with_default(&[0])
    .with_validator(|picked: &[ListOption<&&str>]| {
        if picked.is_empty() {
            Ok(Validation::Invalid("pick at least one target".into()))
        } else {
            Ok(Validation::Valid)
        }
    })
    .prompt()?;
    o.targets = chosen
        .iter()
        .filter_map(|t| match *t {
            "desktop" => Some(Target::Desktop),
            "android" => Some(Target::Android),
            "ios" => Some(Target::Ios),
            "web" => Some(Target::Web),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    // ── auth ─────────────────────────────────────────────────────────────
    if confirm("Accounts and login (marea-auth over PocketBase)?", true)? {
        o.auth = Some(Auth {
            pocketbase_url: Text::new("PocketBase URL")
                .with_default(DEFAULT_POCKETBASE)
                .with_validator(non_empty("a PocketBase URL is required"))
                .prompt()?,
        });
    }

    // ── engine ───────────────────────────────────────────────────────────
    // Only offered with auth: the sync passphrase is derived from the account,
    // so there is no key material without one.
    if o.has_auth() && confirm("Offline-first sync (WaveSyncDB)?", true)? {
        // Only the browser needs a relay — native builds hold their own
        // database and gossip peer-to-peer.
        let relay = if o.has_target(Target::Web) {
            Some(
                Text::new("Relay multiaddr the web build dials")
                    .with_help_message("e.g. /dns4/relay.example.com/tcp/443/wss/p2p/12D3Koo…")
                    .with_validator(|input: &str| {
                        if input.trim().is_empty() {
                            Ok(Validation::Invalid(
                                "a web build has no local database, so it needs a relay".into(),
                            ))
                        } else if !input.trim().starts_with('/') {
                            Ok(Validation::Invalid(
                                "expected a libp2p multiaddr, which starts with `/`".into(),
                            ))
                        } else {
                            Ok(Validation::Valid)
                        }
                    })
                    .prompt()?,
            )
        } else {
            None
        };
        o.engine = Engine::WaveSync { relay };
    }

    // ── bundle identity ──────────────────────────────────────────────────
    if o.has_mobile() {
        let default = format!("com.example.{}", o.derived().snake);
        o.bundle_id = Some(
            Text::new("Bundle identifier (reverse-DNS)")
                .with_help_message("Becomes the Android applicationId. PERMANENT once published.")
                .with_default(&default)
                .with_validator(|input: &str| {
                    if options::is_reverse_dns(input.trim()) {
                        Ok(Validation::Valid)
                    } else {
                        Ok(Validation::Invalid(
                            "expected reverse-DNS, e.g. es.almares.myapp".into(),
                        ))
                    }
                })
                .prompt()?,
        );
    }

    // ── marea features ───────────────────────────────────────────────────
    if o.has_auth() && o.has_target(Target::Web) && o.has_mobile() {
        o.features.pairing = confirm("QR pairing (log into the web app from the phone)?", false)?;
    }
    if o.has_native() {
        o.features.scanner = confirm("Camera code scanner?", false)?;
    }
    if o.has_mobile() {
        o.features.push = confirm("Push notifications?", false)?;
    }
    if o.has_target(Target::Android) {
        o.features.android_back = confirm("Android back gesture pops the router?", true)?;
    }

    let locales = Text::new("Locales (comma separated, first is the default)")
        .with_default("en")
        .with_validator(|input: &str| {
            if input.split(',').any(|s| !s.trim().is_empty()) {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid("list at least one locale".into()))
            }
        })
        .prompt()?;
    o.features.locales = locales
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    // ── styling and repo furniture ───────────────────────────────────────
    o.tailwind = confirm("Tailwind v4 for this app's own screens?", false)?;

    let extras = MultiSelect::new(
        "Repository extras",
        vec![
            "CI workflow (fmt, clippy, tests, architecture gates)",
            "Containers and deploy docs",
            "Agent docs (CLAUDE.md / AGENTS.md) seeded with this app's invariants",
            "clippy.toml",
            "docs/architecture.md",
            "maestro end-to-end smoke flow",
        ],
    )
    .with_default(&[0, 2, 3, 4])
    .prompt()?;
    let picked = |needle: &str| extras.iter().any(|e| e.starts_with(needle));
    o.infra.ci = picked("CI workflow");
    o.infra.containers = picked("Containers");
    o.scaffolding.agent_docs = picked("Agent docs");
    o.scaffolding.clippy = picked("clippy.toml");
    o.scaffolding.docs = picked("docs/");
    o.scaffolding.e2e = picked("maestro");

    o.with_example = confirm("Include the deletable sample feature slice?", true)?;

    Ok(o)
}
