# Scaffolding new apps

```sh
cargo install --path crates/marea-cli      # binary: `marea`
marea new ~/Projects/my-app
```

The wizard asks for targets, auth, engine, marea features, styling and
repository extras; then it writes the workspace, pins the lockfile, builds it
and runs the architecture gates. Non-interactively:

```sh
marea new ~/Projects/my-app --config combo.toml
```

## What it generates

```
my-app/
├── Cargo.toml               [workspace] · pinned deps · profiles
├── crates/
│   ├── domain/              pure Rust: types + rules
│   ├── data/                entities · repositories · sync bootstrap   [wavesyncdb]
│   ├── ui/                  app · routes · shell · features/*
│   └── app-{desktop,mobile,web}/   launchers, ~40 lines each
└── crates/ui/assets/tokens.css     the design system
```

The generated architecture, and why, is documented in the app's own
`docs/architecture.md`. The short version: dependencies point one way
(`domain ← data ← ui ← app-*`), there is one `Route` enum for every target,
and `data` owns every `#[cfg(target_arch)]` in the project.

## The four gates

`marea new` runs these against what it just wrote, and the generated CI runs
them on every commit:

| Gate | Catches |
|---|---|
| One copy of `wavesyncdb` / `dioxus` / `sea-orm` / `dioxus-sdk-storage` | The duplicate-resolution failure that breaks `use_context` at runtime |
| `cargo tree -p domain` free of dioxus / sea-orm / wavesyncdb | The domain layer growing a UI or storage dependency |
| No `#[cfg(target_arch)]` under `crates/ui/src` | Screens learning which target they are on — the first step toward a second view layer |
| fmt · clippy · tests · wasm check | The usual |

Note the first gate names specific crates rather than asserting `cargo tree -d`
is empty. It never is: this very workspace carries ~86 duplicate leaves from
the dioxus desktop stack, and none of them matter. What matters is the handful
whose types cross the app/framework boundary.

## How the generator is built

Two mechanisms, split by failure mode.

**Manifests are built structurally** (`src/manifest.rs`, `toml_edit`). A
template with fifteen conditionals around dependency lines fails by emitting a
manifest that parses but wires the wrong thing, and the symptom appears much
later as a duplicate resolution. Built as code, the specs come from one table
and the tests assert on the parsed result.

**Everything else is a real file** under `crates/marea-cli/templates/`,
rendered with minijinja. `templates/when.toml` maps a path to the condition
under which it is emitted; anything unlisted is always emitted. Prefer a
feature's own file over an `{% if %}` inside a shared one — a template that
reads like the code it becomes is a template people keep correct.

### Two pins, and why they are not the same pin

`src/pins.rs` holds the dependency **specs** — version reqs, git URLs,
branches. A unit test compares them against this workspace's own
`[workspace.dependencies]` and fails if they drift, because differing source
coordinates for one crate resolve twice.

`src/lockfile.rs` handles the **commits**. `wavesyncdb` is a `branch = "dev"`
dependency, and a freshly generated project has no lockfile, so it would
resolve to whatever the branch tip is that morning — not the code marea was
built against. The build script reads this workspace's `Cargo.lock`, bakes the
resolved revisions into the binary, and the generator pins the new project's
lockfile to them.

The spec cannot simply be changed to a `rev =`: that would differ from marea's
own `branch =` spec and cause the very double resolution the first pin exists
to prevent. The lockfile is the correct place for the commit.

### Adding to the templates

- A new generated file: drop it under `templates/`, add a `when.toml` entry if
  it is conditional. A dangling condition fails a test, and so does a template
  whose condition does not compile.
- A new dependency: add it to `pins.rs`. If no marea crate carries it, list it
  in `APP_ONLY` — otherwise the parity test cannot tell a deliberate app-only
  dependency from a typo.
- A new option: `options.rs` (plus validation and a test), `prompts.rs`, and
  `render::Vars` if templates need to see it.

`tests/combos.rs` renders four representative combinations and asserts the
rules hold in the output. `cargo test -p marea-cli` is fast — it never
compiles a generated project. Compiling them is what `marea new` itself does,
and what the CI matrix does.

## Verifying a generated project by hand

```sh
cd ~/Projects/my-app
cargo check --workspace
cargo check -p ui -p data --target wasm32-unknown-unknown
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
dx serve --package app-desktop --platform desktop
```
