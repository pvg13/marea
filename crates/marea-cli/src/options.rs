//! The option model: what the wizard asks, what it derives, and what
//! combinations are rejected.
//!
//! Pure — no I/O, no prompting, no rendering. `prompts.rs` fills an
//! [`Options`] in interactively, `--config` deserializes one from TOML, and
//! both then go through the same [`Options::validate`] before anything is
//! written to disk.
//!
//! # Compatibility invariants
//!
//! [`Derived`] mints the values that become an app's permanent on-disk /
//! on-wire contract — `psk_domain`, `session_storage_key`, the [`DbLocator`]
//! arguments and the WaveSync topic. They are derived from the project name
//! exactly once, at generation time; changing the derivation later would
//! orphan every install of an app generated before the change, so these
//! formulas are locked by the tests at the bottom of this file.
//!
//! [`DbLocator`]: https://docs.rs/marea-sync

use std::collections::BTreeSet;

/// A build target the generated workspace ships a binary crate for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    Desktop,
    Android,
    Ios,
    Web,
}

impl Target {
    /// Android and iOS share the `app-mobile` crate.
    pub fn is_mobile(self) -> bool {
        matches!(self, Target::Android | Target::Ios)
    }

    /// Everything except `Web` — i.e. targets where the app runs natively and
    /// owns a local SQLite database rather than talking to a relay.
    pub fn is_native(self) -> bool {
        !matches!(self, Target::Web)
    }
}

/// PocketBase-backed authentication (`marea-auth`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Auth {
    /// Base URL of the PocketBase instance, e.g. `https://admin.almares.es`.
    pub pocketbase_url: String,
}

/// The persistence/sync engine backing the generated app.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Engine {
    /// No engine: screens keep their own state. Generates no `data` crate.
    #[default]
    None,
    /// WaveSyncDB — native SQLite + web relay client, one `SyncHandle` API.
    WaveSync {
        /// libp2p multiaddr of the relay the web build dials. Required as
        /// soon as `Web` is a target, since the browser has no local engine.
        #[serde(default)]
        relay: Option<String>,
    },
}

/// Optional marea features, each gating a cargo feature plus generated wiring.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct Features {
    /// QR pairing: phone → web login handoff (`marea-ui/pairing`).
    pub pairing: bool,
    /// Camera code scanner (`marea-ui/scanner`).
    pub scanner: bool,
    /// Push-token plumbing for mobile notifications.
    pub push: bool,
    /// Android system back gesture → router (`marea-ui/android-back`).
    pub android_back: bool,
    /// Locales the generated `features/*/i18n.rs` tables carry.
    pub locales: Vec<String>,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            pairing: false,
            scanner: false,
            push: false,
            android_back: false,
            locales: vec!["en".to_string()],
        }
    }
}

/// Repository infrastructure: CI gates, containers, deploy docs.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct Infra {
    /// GitHub Actions workflow running fmt/clippy/test plus the four
    /// architecture gates.
    pub ci: bool,
    /// Dockerfile + compose for the web build, and `DEPLOY.md`.
    pub containers: bool,
}

/// Developer scaffolding that is documentation rather than code.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct Scaffolding {
    /// `CLAUDE.md` + `AGENTS.md` seeded with this project's own invariants.
    pub agent_docs: bool,
    /// `clippy.toml`.
    pub clippy: bool,
    /// `docs/architecture.md`.
    pub docs: bool,
    /// A maestro smoke flow for the mobile target.
    pub e2e: bool,
}

/// How generated manifests reference the marea crates themselves.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DepMode {
    /// Path deps if a marea checkout sits beside the target directory,
    /// `git + tag` otherwise.
    #[default]
    Auto,
    Path,
    Git,
}

/// Everything the generator needs to know, before validation.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(default)]
pub struct Options {
    /// Kebab-case project name; also the generated directory's name.
    pub name: String,
    pub targets: BTreeSet<Target>,
    pub auth: Option<Auth>,
    pub engine: Engine,
    pub features: Features,
    pub tailwind: bool,
    pub infra: Infra,
    pub scaffolding: Scaffolding,
    pub deps: DepMode,
    /// Reverse-DNS bundle identifier, e.g. `es.almares.myapp`.
    ///
    /// Required for mobile targets and **permanent**: an Android
    /// `applicationId` can never change once the app is published, and dx
    /// silently defaults to `com.example.<name>` when this is unset.
    pub bundle_id: Option<String>,
    /// Emit the deletable `items` sample slice demonstrating the layering.
    pub with_example: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            name: String::new(),
            targets: BTreeSet::new(),
            auth: None,
            engine: Engine::None,
            features: Features::default(),
            tailwind: false,
            infra: Infra::default(),
            scaffolding: Scaffolding::default(),
            deps: DepMode::Auto,
            bundle_id: None,
            with_example: true,
        }
    }
}

impl Options {
    pub fn has_auth(&self) -> bool {
        self.auth.is_some()
    }

    pub fn uses_wavesync(&self) -> bool {
        matches!(self.engine, Engine::WaveSync { .. })
    }

    pub fn has_target(&self, t: Target) -> bool {
        self.targets.contains(&t)
    }

    pub fn has_mobile(&self) -> bool {
        self.targets.iter().any(|t| t.is_mobile())
    }

    /// True when at least one target runs natively — i.e. the generated
    /// workspace needs the native halves of `data::bootstrap`.
    pub fn has_native(&self) -> bool {
        self.targets.iter().any(|t| t.is_native())
    }

    /// Reject option combinations that would generate a workspace that does
    /// not compile or does not make sense. Returns every problem at once so
    /// the CLI can print a complete list rather than one-at-a-time.
    pub fn validate(&self) -> Result<(), Vec<OptionsError>> {
        let mut errs = Vec::new();

        if !is_valid_name(&self.name) {
            errs.push(OptionsError::InvalidName(self.name.clone()));
        }
        if self.targets.is_empty() {
            errs.push(OptionsError::NoTargets);
        }

        if self.features.pairing {
            if !self.has_auth() {
                errs.push(OptionsError::PairingNeedsAuth);
            }
            if !self.has_target(Target::Web) {
                errs.push(OptionsError::PairingNeedsWeb);
            }
            if !self.has_mobile() {
                errs.push(OptionsError::PairingNeedsMobile);
            }
        }
        if self.features.android_back && !self.has_target(Target::Android) {
            errs.push(OptionsError::AndroidBackNeedsAndroid);
        }
        if self.features.push && !self.has_mobile() {
            errs.push(OptionsError::PushNeedsMobile);
        }
        if self.features.scanner && !self.has_native() {
            errs.push(OptionsError::ScannerNeedsNativeTarget);
        }

        // The browser has no local engine: without a relay to dial, a web
        // build would come up authenticated and permanently empty.
        if let Engine::WaveSync { relay } = &self.engine
            && self.has_target(Target::Web)
            && relay.as_deref().map(str::trim).unwrap_or("").is_empty()
        {
            errs.push(OptionsError::RelayRequired);
        }

        // The sync passphrase is Argon2id over (user_id, password). Without
        // an account there is no key material, and a shared constant would
        // put every install of the app on one encryption key.
        if self.uses_wavesync() && !self.has_auth() {
            errs.push(OptionsError::WaveSyncNeedsAuth);
        }

        // dx derives the Android applicationId from this. Defaulting it
        // would bake `com.example.…` into a published app, permanently.
        match self.bundle_id.as_deref().map(str::trim) {
            None | Some("") if self.has_mobile() => errs.push(OptionsError::BundleIdRequired),
            Some(id) if !id.is_empty() && !is_reverse_dns(id) => {
                errs.push(OptionsError::InvalidBundleId(id.to_string()))
            }
            _ => {}
        }

        if self.features.locales.is_empty() {
            errs.push(OptionsError::NoLocales);
        }

        if errs.is_empty() { Ok(()) } else { Err(errs) }
    }

    /// The names and compatibility constants derived from [`Options::name`].
    pub fn derived(&self) -> Derived {
        Derived::from_name(&self.name)
    }
}

/// Names and permanent constants derived from the project name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Derived {
    /// The kebab-case name as given: `my-app`.
    pub kebab: String,
    /// `my_app` — identifiers and file names.
    pub snake: String,
    /// `MyApp` — the data directory, the Android binary/label.
    pub pascal: String,
    /// `My App` — window titles and generated prose.
    pub title: String,
    /// `DbLocator` argument 1: the directory under the platform data dir.
    pub app_dir: String,
    /// `DbLocator` argument 2: the SQLite file name.
    pub db_file: String,
    /// Argon2id domain-separation tag for `derive_psk`. **Permanent.**
    pub psk_domain: String,
    /// Key the `AuthSession` is persisted under. **Permanent.**
    pub session_storage_key: String,
    /// Gossipsub topic every install of this app shares. **Permanent.**
    pub sync_topic: String,
}

impl Derived {
    /// Derive every name form from a validated kebab-case project name.
    ///
    /// The splitting is done here, by hand, rather than through a
    /// case-conversion crate: `psk_domain`, `session_storage_key`,
    /// `db_file` and `sync_topic` are permanent contracts, and an upstream
    /// change to how a library splits `app2` into words would silently
    /// re-mint them for every app generated afterwards.
    pub fn from_name(name: &str) -> Self {
        let words: Vec<&str> = name.split('-').filter(|w| !w.is_empty()).collect();

        let snake = words.join("_");
        let pascal: String = words.iter().map(|w| capitalize(w)).collect();
        let title = words
            .iter()
            .map(|w| capitalize(w))
            .collect::<Vec<_>>()
            .join(" ");

        Self {
            kebab: name.to_string(),
            app_dir: pascal.clone(),
            db_file: format!("{snake}.db"),
            psk_domain: format!("{name}.psk.v1|"),
            session_storage_key: format!("{snake}_auth_session"),
            sync_topic: name.to_string(),
            snake,
            pascal,
            title,
        }
    }
}

/// Turn a directory name into a valid project name.
///
/// `marea new Roommates` is the natural thing to type — the directory is
/// capitalised, like every other project in this ecosystem — but the project
/// name feeds crate names and permanent identifiers, which are kebab-case.
/// Rather than rejecting it, derive one: `Roommates` → `roommates`,
/// `MediterraneaDiente` → `mediterranea-diente`, `my_app` → `my-app`.
///
/// Returns an empty string when nothing usable is left, which the caller
/// reports rather than silently substituting.
pub fn slugify(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len() + 4);
    let mut prev_lower_or_digit = false;

    for ch in raw.chars() {
        if ch.is_ascii_uppercase() {
            // camelCase boundary: `MediterraneaDiente` reads as two words.
            if prev_lower_or_digit {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
            prev_lower_or_digit = false;
        } else if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            out.push(ch);
            prev_lower_or_digit = true;
        } else {
            // `_`, spaces, dots and anything else become separators.
            out.push('-');
            prev_lower_or_digit = false;
        }
    }

    // Collapse runs of separators, then trim them from both ends, then drop
    // any leading characters that cannot start a crate name.
    let mut collapsed = String::with_capacity(out.len());
    for ch in out.chars() {
        if ch == '-' && collapsed.ends_with('-') {
            continue;
        }
        collapsed.push(ch);
    }
    let trimmed = collapsed.trim_matches('-');
    let start = trimmed
        .find(|c: char| c.is_ascii_lowercase())
        .unwrap_or(trimmed.len());
    trimmed[start..].trim_matches('-').to_string()
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// At least two dot-separated segments, each starting with a letter.
pub fn is_reverse_dns(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|p| {
            p.starts_with(|c: char| c.is_ascii_alphabetic())
                && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
}

/// `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`, at most 64 characters.
///
/// Hand-rolled rather than pulled from `regex`: it is the crate's only
/// pattern match, and the rule is easier to read as code than as a literal.
pub fn is_valid_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    if !name.starts_with(|c: char| c.is_ascii_lowercase()) {
        return false;
    }
    if name.ends_with('-') {
        return false;
    }
    if name.contains("--") {
        return false;
    }
    name.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// A rejected option combination.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OptionsError {
    #[error(
        "project name {0:?} is not valid: use lowercase letters, digits and single dashes, starting with a letter (e.g. `my-app`)"
    )]
    InvalidName(String),
    #[error("pick at least one target (desktop, android, ios, web)")]
    NoTargets,
    #[error("QR pairing hands a session from a phone to a browser, so it needs auth enabled")]
    PairingNeedsAuth,
    #[error("QR pairing needs the `web` target — the browser is the side being paired")]
    PairingNeedsWeb,
    #[error("QR pairing needs a mobile target — the phone is the side holding the credentials")]
    PairingNeedsMobile,
    #[error("the Android back gesture needs the `android` target")]
    AndroidBackNeedsAndroid,
    #[error("push notifications need a mobile target")]
    PushNeedsMobile,
    #[error("the camera scanner needs a native target (desktop, android or ios)")]
    ScannerNeedsNativeTarget,
    #[error("a web build has no local database: set the relay multiaddr the browser should dial")]
    RelayRequired,
    #[error("list at least one locale")]
    NoLocales,
    #[error(
        "a mobile target needs a reverse-DNS bundle id (e.g. es.almares.myapp). It becomes the Android applicationId, which can never be changed after publishing — and dx would otherwise ship `com.example.…`"
    )]
    BundleIdRequired,
    #[error("bundle id {0:?} is not reverse-DNS (expected something like es.almares.myapp)")]
    InvalidBundleId(String),
    #[error(
        "WaveSyncDB derives its per-user encryption passphrase from the account, so sync needs auth enabled"
    )]
    WaveSyncNeedsAuth,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(name: &str, targets: &[Target]) -> Options {
        Options {
            name: name.to_string(),
            targets: targets.iter().copied().collect(),
            bundle_id: Some("es.almares.myapp".into()),
            ..Default::default()
        }
    }

    fn authed(name: &str, targets: &[Target]) -> Options {
        Options {
            auth: Some(Auth {
                pocketbase_url: "https://admin.almares.es".into(),
            }),
            bundle_id: Some("es.almares.myapp".into()),
            ..opts(name, targets)
        }
    }

    // ── derivation ───────────────────────────────────────────────────────
    //
    // These values are an app's permanent contract. If a change to `heck` or
    // to the formulas moves them, every install of an app generated earlier
    // loses its database and its session.

    #[test]
    fn derives_every_name_form_from_a_kebab_project_name() {
        let d = Derived::from_name("my-app");
        assert_eq!(d.kebab, "my-app");
        assert_eq!(d.snake, "my_app");
        assert_eq!(d.pascal, "MyApp");
        assert_eq!(d.title, "My App");
    }

    #[test]
    fn derives_the_permanent_compatibility_constants() {
        let d = Derived::from_name("my-app");
        assert_eq!(d.app_dir, "MyApp");
        assert_eq!(d.db_file, "my_app.db");
        assert_eq!(d.psk_domain, "my-app.psk.v1|");
        assert_eq!(d.session_storage_key, "my_app_auth_session");
        assert_eq!(d.sync_topic, "my-app");
    }

    #[test]
    fn derivation_handles_a_single_word_name() {
        let d = Derived::from_name("ascend");
        assert_eq!(d.snake, "ascend");
        assert_eq!(d.pascal, "Ascend");
        assert_eq!(d.db_file, "ascend.db");
        assert_eq!(d.psk_domain, "ascend.psk.v1|");
    }

    #[test]
    fn derivation_keeps_digits_attached_to_their_word() {
        // `heck` would otherwise be free to split `app2` into `App 2`.
        let d = Derived::from_name("app2-sync");
        assert_eq!(d.snake, "app2_sync");
        assert_eq!(d.pascal, "App2Sync");
    }

    // ── name validation ──────────────────────────────────────────────────

    // ── slugifying a directory name ──────────────────────────────────────

    #[test]
    fn a_capitalised_directory_name_becomes_a_valid_project_name() {
        assert_eq!(slugify("Roommates"), "roommates");
        assert_eq!(slugify("Ascend"), "ascend");
    }

    #[test]
    fn camel_case_directory_names_split_into_words() {
        assert_eq!(slugify("MediterraneaDiente"), "mediterranea-diente");
        assert_eq!(slugify("MyApp"), "my-app");
    }

    #[test]
    fn separators_are_normalised() {
        assert_eq!(slugify("my_app"), "my-app");
        assert_eq!(slugify("my app"), "my-app");
        assert_eq!(slugify("my--app"), "my-app");
        assert_eq!(slugify("-my-app-"), "my-app");
        assert_eq!(slugify("2fast"), "fast");
    }

    #[test]
    fn already_valid_names_are_left_alone() {
        for name in ["my-app", "ascend", "app2-sync"] {
            assert_eq!(slugify(name), name);
        }
    }

    #[test]
    fn slugify_output_always_passes_name_validation() {
        for raw in [
            "Roommates",
            "MediterraneaDiente",
            "my_app",
            "My App!",
            "2fast",
            "  spaced  out  ",
        ] {
            let slug = slugify(raw);
            assert!(
                is_valid_name(&slug),
                "slugify({raw:?}) produced {slug:?}, which is not a valid name"
            );
        }
    }

    #[test]
    fn slugify_reports_when_nothing_usable_is_left() {
        assert_eq!(slugify("123"), "");
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn accepts_well_formed_kebab_names() {
        for name in ["a", "ascend", "my-app", "app2-sync", "a-b-c"] {
            let o = opts(name, &[Target::Desktop]);
            assert_eq!(o.validate(), Ok(()), "expected {name:?} to be accepted");
        }
    }

    #[test]
    fn rejects_malformed_names() {
        for name in [
            "",
            "My-App",        // uppercase
            "my_app",        // snake
            "-my-app",       // leading dash
            "my-app-",       // trailing dash
            "my--app",       // doubled dash
            "2fast",         // leading digit
            "my app",        // space
            "my.app",        // dot
            &"a".repeat(65), // too long
        ] {
            let o = opts(name, &[Target::Desktop]);
            let errs = o.validate().expect_err("expected {name:?} to be rejected");
            assert!(
                errs.contains(&OptionsError::InvalidName(name.to_string())),
                "expected InvalidName for {name:?}, got {errs:?}"
            );
        }
    }

    // ── combination validation ───────────────────────────────────────────

    #[test]
    fn requires_at_least_one_target() {
        let errs = opts("my-app", &[]).validate().unwrap_err();
        assert!(errs.contains(&OptionsError::NoTargets));
    }

    #[test]
    fn pairing_requires_auth_web_and_mobile() {
        let mut o = opts("my-app", &[Target::Desktop]);
        o.features.pairing = true;
        let errs = o.validate().unwrap_err();
        assert!(errs.contains(&OptionsError::PairingNeedsAuth));
        assert!(errs.contains(&OptionsError::PairingNeedsWeb));
        assert!(errs.contains(&OptionsError::PairingNeedsMobile));
    }

    #[test]
    fn pairing_is_accepted_with_auth_web_and_a_phone() {
        let mut o = authed("my-app", &[Target::Android, Target::Web]);
        o.features.pairing = true;
        o.engine = Engine::WaveSync {
            relay: Some("/dns4/relay.example/udp/4011/quic-v1/p2p/12D3Koo".into()),
        };
        assert_eq!(o.validate(), Ok(()));
    }

    #[test]
    fn android_back_requires_the_android_target() {
        let mut o = opts("my-app", &[Target::Ios]);
        o.features.android_back = true;
        assert!(
            o.validate()
                .unwrap_err()
                .contains(&OptionsError::AndroidBackNeedsAndroid)
        );

        o.targets.insert(Target::Android);
        assert_eq!(o.validate(), Ok(()));
    }

    #[test]
    fn push_requires_a_mobile_target() {
        let mut o = opts("my-app", &[Target::Desktop]);
        o.features.push = true;
        assert!(
            o.validate()
                .unwrap_err()
                .contains(&OptionsError::PushNeedsMobile)
        );
    }

    #[test]
    fn scanner_requires_a_native_target() {
        let mut o = opts("my-app", &[Target::Web]);
        o.features.scanner = true;
        assert!(
            o.validate()
                .unwrap_err()
                .contains(&OptionsError::ScannerNeedsNativeTarget)
        );

        o.targets.insert(Target::Desktop);
        assert_eq!(o.validate(), Ok(()));
    }

    #[test]
    fn a_web_build_on_wavesync_needs_a_relay() {
        let mut o = authed("my-app", &[Target::Web]);
        o.engine = Engine::WaveSync { relay: None };
        assert!(
            o.validate()
                .unwrap_err()
                .contains(&OptionsError::RelayRequired)
        );

        o.engine = Engine::WaveSync {
            relay: Some("/dns4/relay.example/udp/4011/quic-v1/p2p/12D3Koo".into()),
        };
        assert_eq!(o.validate(), Ok(()));
    }

    #[test]
    fn a_native_only_wavesync_app_needs_no_relay() {
        let mut o = authed("my-app", &[Target::Desktop]);
        o.engine = Engine::WaveSync { relay: None };
        assert_eq!(o.validate(), Ok(()));
    }

    #[test]
    fn sync_without_auth_is_rejected() {
        let mut o = opts("my-app", &[Target::Desktop]);
        o.engine = Engine::WaveSync { relay: None };
        assert!(
            o.validate()
                .unwrap_err()
                .contains(&OptionsError::WaveSyncNeedsAuth)
        );
    }

    #[test]
    fn a_mobile_target_requires_a_bundle_id() {
        let mut o = opts("my-app", &[Target::Android]);
        o.bundle_id = None;
        assert!(
            o.validate()
                .unwrap_err()
                .contains(&OptionsError::BundleIdRequired)
        );

        // Desktop-only apps do not publish to a store, so it stays optional.
        let mut d = opts("my-app", &[Target::Desktop]);
        d.bundle_id = None;
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn a_bundle_id_must_be_reverse_dns() {
        let mut o = opts("my-app", &[Target::Android]);
        o.bundle_id = Some("myapp".into());
        assert!(
            o.validate()
                .unwrap_err()
                .contains(&OptionsError::InvalidBundleId("myapp".into()))
        );

        o.bundle_id = Some("es.almares.myapp".into());
        assert_eq!(o.validate(), Ok(()));
    }

    #[test]
    fn requires_at_least_one_locale() {
        let mut o = opts("my-app", &[Target::Desktop]);
        o.features.locales = vec![];
        assert!(o.validate().unwrap_err().contains(&OptionsError::NoLocales));
    }

    #[test]
    fn reports_every_problem_at_once() {
        let mut o = opts("My App", &[]);
        o.features.push = true;
        let errs = o.validate().unwrap_err();
        assert!(errs.len() >= 3, "expected several errors, got {errs:?}");
    }
}
