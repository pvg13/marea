# marea

Shared framework for the Dioxus apps in this ecosystem (MediterraneaDiente,
Ascend, roommates_dx, and everything new). Apps keep their domain — entities,
screens, engines — and take auth, sync conventions, the UI shell and the
component library from here.

## Crates

| Crate | What it owns |
|---|---|
| `marea-auth` | PocketBase client (login/register/refresh, structured field errors), `AuthSession`, Argon2id PSK derivation with per-app domain salt, JWT freshness, QR-pairing crypto (`pairing` feature), Dioxus session hooks (`dioxus` feature) |
| `marea-sync` | `DbLocator` (per-app/per-account SQLite locations), `registry_name!`, idempotent `schema::ensure_columns`, the safe CRDT write facades (`SyncDbExt` on `WaveSyncDb`, cross-target `SyncHandleExt` on `SyncHandle`), re-exported `wavesyncdb` |
| `marea-ui` | `AppShell` (theme restore, providers, auth gate, uid-keyed subtree), `NavShell` (sidebar ≥768px / bottom tabs, `mobile_only` mode), `LoginScreen`, component library, toasts, `ThemeCtl`, push-token context, `Locale` |

## How an app plugs in

```toml
# workspace Cargo.toml — specs must be BYTE-IDENTICAL across apps + marea
marea-auth = { path = "../dioxus/crates/marea-auth", features = ["dioxus"] }  # TODO: git tag
marea-ui   = { path = "../dioxus/crates/marea-ui" }
marea-sync = { path = "../dioxus/crates/marea-sync" }
dioxus-sdk-storage = { git = "https://github.com/DioxusLabs/sdk.git", rev = "15525043cadc3c467afb15888ce804bdc9b6a46e" }
wavesyncdb = { git = "https://github.com/pvg13/WaveSyncDB.git", branch = "dev" }
```

```rust
static AUTH: AuthConfig = AuthConfig {
    pocketbase_url: "https://admin.almares.es",
    psk_domain: b"myapp.psk.v1|",            // NEVER change after shipping
    session_storage_key: "myapp_auth_session", // ditto
};

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: marea_ui::MAREA_CSS }
        document::Stylesheet { href: asset!("/assets/tokens.css") } // your skin
        AppShell { auth: &AUTH, AuthedApp {} }
    }
}
```

Theming: define the `--c-*` tokens (see `docs/theme-contract.md`; start from
`crates/marea-ui/assets/tokens.reference.css`). Identical components render
as parchment-olive, teal, or sage purely through that file.

## Rules that keep the mesh working

1. **Duplicate-dep gate**: `cargo tree -d` must show one copy of
   `wavesyncdb`, `dioxus`, `sea-orm`, `dioxus-sdk-storage`. Different git
   specs (branch vs rev) for the same URL resolve twice and break contexts
   at runtime.
2. **Compatibility invariants** (per migrated app, locked with tests):
   PSK salt domain + Argon2id params, session storage key + `AuthSession`
   JSON shape, DB paths (`DbLocator` args), schema-registry name, WaveSync
   topic.
3. All writes go through the marea-sync facades — never raw
   `ActiveModel::insert/update`, never upstream `SyncHandle::submit`.
4. Framework rsx uses semantic `marea.css` classes only; app screens can use
   anything (the apps use Tailwind v4 via `dx`).

## Showcase

```sh
cd examples/showcase
dx serve --platform desktop     # or: --platform web --features web --no-default-features
```

Logs in against the real shared PocketBase and exercises every component,
the nav shell, toasts and the theme toggle.

## Migration playbook (Mediterranea / Roommates — pending)

Ascend is migrated (branch `marea-migration`) and is the reference diff.
For the next two:

- **Mediterranea**: `dto` → marea-auth (`psk_domain: b"mediterranea.psk.v1|"`,
  key `"auth_session"`, `pairing` feature for QR login — port
  `pair_web.rs`/scanner UI into marea-ui `pairing` feature first, plus the
  Android back-gesture module). `ui::sync_ext` → marea-sync `SyncHandleExt`
  (already a byte-compatible port). `session.rs` → `DbLocator("Mediterranea",
  "mediterranea.db")`. Keep household invites app-side on
  `marea_auth::seal_value`.
- **Roommates**: uses its own WaveSyncDB fork on `branch = "main"` — align to
  the shared `dev` branch first. Two-key session storage (`"user"` +
  `"tokens"`) needs a one-time read-old/write-new shim. Group scoping maps to
  `DbLocator::scoped_db_url` + `scoped_topic("roommates", gid)`.
