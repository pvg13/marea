# {{ title }}

Built on [marea](https://github.com/pvg13/marea): auth, sync conventions, the
UI shell and the component library come from the framework; this repository
owns the domain.

## Layout

```
crates/
  domain/      pure Rust — types and rules. No dioxus, no database.
{%- if wavesync %}
  data/        synced entities · repositories · sync bootstrap
{%- endif %}
  ui/          screens, routes, shell — one set for every target
{%- if desktop %}
  app-desktop/ launcher
{%- endif %}
{%- if mobile %}
  app-mobile/  launcher (Android{% if ios %} + iOS{% endif %})
{%- endif %}
{%- if web %}
  app-web/     launcher (wasm)
{%- endif %}
```

## Running it

```sh
{%- if desktop %}
dx serve --package app-desktop --platform desktop
{%- endif %}
{%- if android %}
dx serve --package app-mobile  --platform android
{%- endif %}
{%- if ios %}
dx serve --package app-mobile  --platform ios
{%- endif %}
{%- if web %}
dx serve --package app-web     --platform web
{%- endif %}
```

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
{%- if web %}
cargo check -p ui{% if wavesync %} -p data{% endif %} --target wasm32-unknown-unknown
{%- endif %}
```

## The four rules

They are what keep this from turning into one big `ui` crate. CI enforces the
last two.

1. A component lives in its feature until a **second** feature needs it, then
   it moves to `crates/ui/src/components/`.
2. Strings live per feature (`features/*/i18n.rs`), never in one central table.
3. One `Route` enum. Phone/desktop differences are responsive branches inside
   a screen, never a parallel router.
4. `ui` contains no `#[cfg(target_arch)]`{% if wavesync %} — screens reach data through
   `data`'s repositories, which are the same API natively and in the browser{% endif %}.

## Theming

`crates/ui/assets/tokens.css` is the design system. Every colour, radius and
shadow marea renders resolves through those variables, so reskinning the whole
app — components, nav, login screen, toasts — is editing that one file.
`crates/ui/assets/app.css` is for this app's own layout.
{% if auth %}
## Compatibility invariants

Permanent from the first shipped install. Changing any of them orphans
existing users' data and logins:

| What | Value | Where |
|---|---|---|
| PSK domain | `{{ psk_domain }}` | `crates/ui/src/app.rs` |
| Session key | `{{ session_storage_key }}` | `crates/ui/src/app.rs` |
{%- if wavesync %}
| Data dir | `{{ app_dir }}` | `crates/data/src/bootstrap/native.rs` + each launcher |
| Database | `{{ db_file }}` | `crates/data/src/bootstrap/native.rs` |
| Sync topic | `{{ sync_topic }}` | `crates/data/src/bootstrap/{native,web}.rs` |
{%- endif %}
{% endif %}
## Dependency rule

`wavesyncdb`, `dioxus`, `sea-orm` and `dioxus-sdk-storage` must each resolve to
exactly one copy. Two resolutions of the same crate are two distinct sets of
types, so a context provided by one is invisible to the other — and it fails at
runtime, not at compile time. Keep the specs in `[workspace.dependencies]`
identical to marea's, and let the lockfile pin the rest.
