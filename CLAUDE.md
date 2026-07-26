# CLAUDE.md — marea

Shared framework workspace for the ecosystem's Dioxus 0.7 apps
(MediterraneaDiente, Ascend, roommates_dx). See `README.md` for the crate map
and consumption model, `docs/theme-contract.md` for the styling contract.

## Layout

```
crates/marea-auth    PocketBase + AuthSession + PSK + pairing crypto (+dioxus hooks feature)
crates/marea-sync    WaveSyncDB conventions: DbLocator, registry_name!, schema, write facades
crates/marea-ui      AppShell, NavShell, LoginScreen, components, toasts, theme, push, i18n
                     (+ `scanner` feature: camera; `pairing` feature: QR login screens)
examples/showcase    real app exercising everything (dx serve from its dir)
docs/                theme-contract.md
dioxus-template/ …   legacy pre-marea dirs, untracked/gitignored, pending rework
```

## Hard rules

- **Compatibility invariants are sacred.** `derive_psk` params/salt layout,
  `AuthSession` serde field names, storage keys, `DbLocator` path shapes —
  all are on-disk/on-wire contracts of shipped apps, locked by golden-vector
  and shape tests. Never "fix" an expected value in those tests.
- **Dependency specs must stay byte-identical to the apps'**:
  `wavesyncdb` git branch `dev`, `dioxus-sdk-storage` pinned rev, sea-orm
  req compatible with `=2.0.0-rc.38` (rustc 1.92 ceiling — rc.41 needs 1.94).
  Different specs for the same git URL resolve TWICE and break shared types
  at runtime. Gate: `cargo tree -d`.
- **marea-ui rsx uses semantic `marea.css` classes only** — no Tailwind
  utilities, no inline styles (except inherently dynamic values), every
  color in `marea.css` is `var(--c-*)`, SVG icons use `currentColor`.
- **Write paths**: apps go through `SyncDbExt::submit_upsert`/`delete_row`
  (or cross-target `SyncHandleExt`); upstream `SyncHandle::submit` is broken
  for user-minted PKs (see module docs in marea-sync).
- Dioxus 0.7 discipline: no signal read/write guards across `.await`; hooks
  unconditionally at component top; `consume_context` in handlers.

## Commands

```sh
cargo test --workspace                     # all crates (~30 unit/SSR tests + facades)
cargo check -p marea-auth -p marea-sync -p marea-ui --target wasm32-unknown-unknown
cargo clippy --workspace --all-features -- -D warnings
cd examples/showcase && dx serve --platform desktop
```

## Consumers

Apps take marea via path deps today (`TODO(marea)` markers), git tags after
the first release. Ascend is migrated (branch `marea-migration` in
~/Projects/Ascend) and is the reference migration diff; the README's
playbook covers Mediterranea (needs pairing UI + back-gesture ports first)
and Roommates (needs WaveSyncDB branch alignment + session-key shim).
