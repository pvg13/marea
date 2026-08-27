# Deploying {{ title }}

## Web

```sh
docker compose up --build         # local, http://localhost:8080
```

The image is a two-stage build: `dx bundle --platform web --release --locked`
produces the wasm and assets, nginx serves them. `--locked` is load-bearing —
`wavesyncdb` tracks a branch, so an unlocked build could ship a different sync
engine than the one this app was tested against.

`nginx.conf` does two things worth knowing about:

- **SPA fallback.** The Dioxus router owns every path, so anything that is not
  a real file returns `index.html`. Without it, refreshing on any page but `/`
  is a 404.
- **Cache split.** Hashed assets are immutable and cached for a year;
  `index.html` is `no-store`. Get this backwards and a deploy leaves browsers
  running the previous wasm bundle indefinitely.
{% if wavesync %}
### Relay

The browser has no local database — it syncs entirely through the relay at:

```
{{ relay }}
```

Override per build with `RELAY_WS=…`, or per tab with `?relay=<multiaddr>` for
pointing a session at a locally-running relay.

`wss://` works from a plain-HTTP `dx serve` origin, so development needs no TLS
of its own; browsers only block `ws://` from an `https://` page.
{% endif %}
{%- if desktop %}
## Desktop

```sh
dx bundle --package app-desktop --platform desktop --release
```
{%- endif %}
{%- if mobile %}
## Mobile

```sh
dx bundle --package app-mobile --platform android --release
{%- if ios %}
dx bundle --package app-mobile --platform ios --release
{%- endif %}
```

The bundle identifier is `{{ bundle_id }}`. It is the Android `applicationId`
and can never change once published — releasing under the wrong one means
publishing a second, unrelated app.

Signing keys belong in CI secrets, never in the repository; `.gitignore`
already excludes `*.keystore`, `*.jks` and the Google service files.
{%- endif %}

## Before the first release

{% if auth %}Check the compatibility invariants in `README.md`. `psk_domain`, the session
key{% if wavesync %}, the database location and the sync topic{% endif %} become permanent the moment a
real user installs this. After that, changing one orphans their data and their
login with no migration path.{% else %}Nothing here is on-disk permanent yet. If this app later grows accounts or
sync, the keys it mints at that point become permanent — see marea's docs.{% endif %}
