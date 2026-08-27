# {{ title }} — web build, served as static files.
#
# Only the browser target is containerised. The desktop and mobile builds are
# signed artefacts produced by `dx bundle`, not services.

# ── build ────────────────────────────────────────────────────────────────
FROM rust:1.92-bookworm AS build

# dx compiles the wasm, runs wasm-opt{% if tailwind %} and builds Tailwind{% endif %}.
RUN rustup target add wasm32-unknown-unknown \
 && cargo install dioxus-cli --locked --version 0.7.9

WORKDIR /src

# Copy manifests and the lockfile first so dependency compilation is cached
# independently of source edits. The lockfile matters here beyond caching:
# `wavesyncdb` tracks a branch, and only the lockfile pins the commit this app
# was actually built and tested against.
COPY Cargo.toml Cargo.lock ./
COPY crates/domain/Cargo.toml       crates/domain/Cargo.toml
{%- if wavesync %}
COPY crates/data/Cargo.toml         crates/data/Cargo.toml
{%- endif %}
COPY crates/ui/Cargo.toml           crates/ui/Cargo.toml
COPY crates/app-web/Cargo.toml      crates/app-web/Cargo.toml
{%- if desktop %}
COPY crates/app-desktop/Cargo.toml  crates/app-desktop/Cargo.toml
{%- endif %}
{%- if mobile %}
COPY crates/app-mobile/Cargo.toml   crates/app-mobile/Cargo.toml
{%- endif %}

COPY . .

# `--locked` so a build never silently resolves a newer wavesyncdb than the one
# this app was tested against.
RUN dx bundle --package app-web --platform web --release --locked

# ── serve ────────────────────────────────────────────────────────────────
FROM nginx:alpine

COPY --from=build /src/target/dx/app-web/release/web/public /usr/share/nginx/html
COPY nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 80
