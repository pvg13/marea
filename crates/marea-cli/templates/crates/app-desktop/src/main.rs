//! {{ title }} — desktop launcher.
//!
//! Launchers stay this small on purpose. Anything that is not "start this
//! platform" belongs in `ui`, where all three targets share it.
//!
//!     dx serve --package app-desktop --platform desktop

fn main() {
    // dx installs a `tracing` subscriber, but the sync engine and several
    // libraries log through the `log` crate. Without this bridge their output
    // is silently dropped and the engine looks inert.
    tracing_log::LogTracer::init().ok();

    // Per-app data directory. Must match `DbLocator`'s `app_dir` — both are
    // derived from the project name, and both are permanent once shipped.
    dioxus_sdk_storage::set_dir!(dioxus_sdk_storage::data_directory().join("{{ app_dir }}"));

    dioxus::launch(ui::App);
}
