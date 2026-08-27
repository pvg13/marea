# CLAUDE.md — {{ title }}

A marea app. The framework (`marea-auth`, `marea-ui`{% if wavesync %}, `marea-sync`{% endif %}) supplies
auth, the UI shell, the component library{% if wavesync %} and the sync conventions{% endif %};
this repository owns the domain.

## Layout

```
crates/domain    pure Rust: types + rules. No dioxus, no sea-orm{% if wavesync %}, no wavesyncdb{% endif %}.
{%- if wavesync %}
crates/data      synced entities · typed repositories · sync bootstrap
                 (the ONLY place with #[cfg(target_arch)])
{%- endif %}
crates/ui        app.rs · routes.rs · shell.rs · features/<name>/ · components/
{%- if desktop %}
crates/app-desktop  launcher only (~40 lines)
{%- endif %}
{%- if mobile %}
crates/app-mobile   launcher only
{%- endif %}
{%- if web %}
crates/app-web      launcher only
{%- endif %}
```

## Hard rules

- **One `Route` enum, one screen set.** marea's `NavShell` is responsive
  (sidebar ≥768px, bottom tabs below), so phone and desktop share every
  screen. A difference is a branch *inside* a screen — never a second router
  and never a parallel view layer. This is the rule whose violation turned an
  earlier app in this ecosystem into three view layers over one domain.
- **No `#[cfg(target_arch)]` in `crates/ui`.** {% if wavesync %}Screens call
  `data`'s repositories, which are one API on both sides.{% else %}Targets differ in
  their launcher, not in their screens.{% endif %} CI greps for this.
- **`domain` stays pure.** No UI, no storage. CI asserts it with
  `cargo tree -p domain`.
- **A component belongs to its feature** until a second feature needs it, then
  it moves to `ui/src/components/`. Same for strings: `features/*/i18n.rs`,
  never one central table.
{%- if auth %}
- **Compatibility invariants are permanent.** `psk_domain`
  (`{{ psk_domain }}`), the session key (`{{ session_storage_key }}`){% if wavesync %}, the
  `DbLocator` arguments (`{{ app_dir }}` / `{{ db_file }}`) and the sync topic
  (`{{ sync_topic }}`){% endif %} are on-disk and on-wire contracts. Changing one after
  the first install orphans every existing user's data and login. Never "fix"
  one of these to make something work.
{%- endif %}
- **Dependency specs must stay identical to marea's.** `wavesyncdb`,
  `dioxus`, `sea-orm` and `dioxus-sdk-storage` must each resolve once —
  two copies are two distinct sets of types, and `use_context` then finds
  nothing at runtime. The lockfile pins the exact commits marea was built
  against; do not `cargo update` those without rebuilding against marea.
{%- if wavesync %}
- **Writes go through the repositories** in `data::repo`, which use marea's
  `SyncHandleExt`. Upstream `SyncHandle::submit` is broken on native for
  user-minted primary keys.
- **Entity changes are additive only.** Adding a nullable column is safe;
  renaming or dropping one orphans data already on every install.
{%- endif %}
- Dioxus 0.7: no signal read/write guards held across `.await`; hooks
  unconditionally at the top of a component; `consume_context` in handlers.

## Commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
{%- if web %}
cargo check -p ui{% if wavesync %} -p data{% endif %} --target wasm32-unknown-unknown
{%- endif %}
{%- if desktop %}
dx serve --package app-desktop --platform desktop
{%- endif %}
{%- if android %}
dx serve --package app-mobile --platform android
{%- endif %}
{%- if web %}
dx serve --package app-web --platform web
{%- endif %}
```

## marea features enabled here
{% if pairing or scanner or push or android_back %}
| Feature | What it gives you | Where it is wired |
|---|---|---|
{%- if pairing %}
| `pairing` | QR phone → browser login | `app-web` mounts `PairScreen`; Settings mounts `PairDeviceSection`. Config: `ui::app::PAIRING` (scheme `{{ name }}`, collection `pairing_mailbox`). Needs that collection to exist in PocketBase. |
{%- endif %}
{%- if android_back %}
| `android-back` | System back gesture pops the router | `ui::shell::Shell` renders `GlobalBackHandler`. Requires the Android `MainActivity.onBack()` to call the exported JNI symbol. |
{%- endif %}
{%- if scanner %}
| `scanner` | Camera code scanner (`marea_ui::scanner`) | **Capability only — no call site generated.** Mount it from the feature slice that needs it. Asks for camera permission, so declare that in the Android manifest / iOS `Info.plist`. |
{%- endif %}
{%- if push %}
| `push` | Push-token context (`marea_ui::use_push_token`) | **Capability only — no call site generated.** The device-token plumbing (FCM `google-services.json` on Android, APNs + a notification service extension on iOS) is app- and account-specific, so it is deliberately not scaffolded. |
{%- endif %}

Enabling a feature adds the dependency and its weight; if one of these turns
out not to be used, drop it from `[workspace.dependencies]` rather than
leaving it on.
{% else %}
None beyond the defaults. Adding one means enabling the cargo feature on
`marea-ui` (or `marea-auth`) in `[workspace.dependencies]` and wiring its
component where it belongs.
{% endif %}
## Styling

`crates/ui/assets/tokens.css` is the design system — every marea colour
resolves through it. `crates/ui/assets/app.css` holds this app's own layout.
{% if tailwind %}Tailwind is available for app screens; marea's own components never use
utility classes.{% endif %} Use the `--c-*` tokens rather than literal colours: a
hardcoded hex is invisible to the theme toggle and to any future reskin.
