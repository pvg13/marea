# {{ title }} architecture

## Why the crates are split this way

```
domain ← {% if wavesync %}data ← {% endif %}ui ← app-*
```

Dependencies point one way. Nothing below reaches up.

**`domain`** holds the types, invariants and calculations that define what this
app *is*. It depends on `serde`, `chrono` and nothing else — no dioxus, no
database. Rules tested here need no runtime, no fixtures and no async, and they
survive a rewrite of everything above them. CI asserts the boundary with
`cargo tree -p domain`.
{% if wavesync %}
**`data`** owns persistence: the synced entities, one typed repository per
aggregate, and the sync bootstrap. It is the only crate in the workspace that
knows which target it is running on.
{% endif %}
**`ui`** holds every screen, one `Route` enum and the shell. It is written once
and runs on every target.

**`app-*`** are launchers: set the storage directory, install a logger, call
`dioxus::launch(ui::App)`. Around forty lines each. Anything longer has escaped
from `ui`.
{% if wavesync %}
## How one screen serves phone, desktop and browser

```
app-mobile/main.rs → ui::App
  └ marea_ui::AppShell            session gate · theme · toasts,
  │                               re-keyed by user id
      └ data::bootstrap::SyncProvider     ← the only cfg fork in the app
          │   native: DbLocator → WaveSyncDb → schema sync → SyncHandle
          │   web:    WebSyncClient::connect_via_relay → SyncHandle
          └ ui::shell::Shell (NavShell + Router::<Route>)
              └ features/<name>/screen.rs
                    let items = use_items();      // reads, live
                    items.save(&item).await?;     // writes
```

Natively the app owns a SQLite file and gossips with the account's other
devices; in the browser there is no database at all and everything goes through
the relay. Both paths end by providing the same `SyncHandle`, and both
`use_synced_table` (reads) and `SyncHandleExt` (writes) exist on both targets —
so the repositories are written once and the screens never learn which side
they are on.

Isolation comes from the passphrase, not the topic: every install shares the
topic `{{ sync_topic }}`, and peers whose Argon2id-derived keys differ ignore
each other. That is also why sync requires an account — the key material is
derived from it.
{% endif %}
## Feature slices

```
ui/src/features/<name>/
  mod.rs          re-exports the screen under its route name
  screen.rs       the routed screen
  components.rs   components only this feature uses
  state.rs        hooks and signals local to this feature
  i18n.rs         this feature's strings
```

Not `pages/` + `components/` + `data/` + one central `i18n.rs`. That layout
splits a feature across four directories, so every change touches all four and
the shared files grow without bound — one app in this ecosystem reached a
5000-line `i18n.rs` and a 1000-line shared components module that way. Here,
deleting a feature is deleting a directory.

A component is promoted to `ui/src/components/` when a *second* feature needs
it. Promoting is a two-line change; splitting an overgrown shared module later
is not.

## Adding a feature

1. `crates/domain/src/<thing>.rs` — the type and its rules, with unit tests.
{%- if wavesync %}
2. `crates/data/src/entities/<thing>.rs` — the synced table. String UUID
   primary key, minted by the app.
3. Register it in `bootstrap::native`'s schema sync.
4. `crates/data/src/repo/<things>.rs` — the vocabulary screens need.
5. `crates/ui/src/features/<name>/` — the slice.
6. Add the variant to `ui/src/routes.rs` and a `NavItem` in `ui/src/shell.rs`.
{%- else %}
2. `crates/ui/src/features/<name>/` — the slice.
3. Add the variant to `ui/src/routes.rs` and a `NavItem` in `ui/src/shell.rs`.
{%- endif %}

## What CI enforces

| Gate | Why |
|---|---|
| One copy of `wavesyncdb` / `dioxus` / `sea-orm` / `dioxus-sdk-storage` | Two resolutions are two distinct sets of types; contexts silently stop matching at runtime |
| `domain` pulls no dioxus / sea-orm{% if wavesync %} / wavesyncdb{% endif %} | Keeps the domain testable and portable |
| No `#[cfg(target_arch)]` under `crates/ui/src` | The moment screens branch on target, the second view layer starts |
| fmt · clippy · tests{% if web %} · wasm check{% endif %} | The usual |

Note that the duplicate check names specific crates rather than asserting
`cargo tree -d` is empty. It never is — the dioxus desktop stack alone carries
dozens of duplicate leaves, and none of them matter. What matters is the
handful whose types cross the app/framework boundary.
