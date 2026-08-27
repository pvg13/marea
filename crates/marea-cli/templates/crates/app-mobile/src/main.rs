//! {{ title }} — mobile launcher (Android{% if ios %} and iOS{% endif %}).
//!
//! Launchers stay this small on purpose. Anything that is not "start this
//! platform" belongs in `ui`, where all targets share it.
//!
//!     dx serve --package app-mobile --platform android
{%- if ios %}
//!     dx serve --package app-mobile --platform ios
{%- endif %}

// Only the `mobile` feature build uses `LaunchBuilder`; the plain
// `cargo check` build launches through `dioxus::launch`.
#[cfg(feature = "mobile")]
use dioxus::prelude::*;
{% if android %}
#[cfg(target_os = "android")]
const LOG_FILTER: &str = "info,wavesync=debug";
{% endif %}
{%- if mobile %}
// dioxus-desktop's bundled `prod.index.html` sets a viewport meta with no
// `interactive-widget` hint, so modern Chromium WebViews do NOT shrink the
// layout viewport when the soft keyboard opens — even with the activity
// flagged `windowSoftInputMode="adjustResize"`. Injecting a second viewport
// meta fixes it: per the HTML spec the *last* meta wins.
//
// Delete this and every text field on Android ends up under the keyboard.
#[cfg(feature = "mobile")]
const KEYBOARD_VIEWPORT: &str = "<meta name=\"viewport\" content=\"width=device-width, \
    initial-scale=1, maximum-scale=1, user-scalable=no, viewport-fit=cover, \
    interactive-widget=resizes-content\">";
{%- endif %}

fn main() {
{%- if android %}
    // WaveSyncDB installs `android_logger` only from its JNI entry points, so
    // the in-process foreground engine is invisible to logcat until a push
    // happens to fire the FFI. Installing it here means the engine always
    // shows under `adb logcat -s wavesync`. `init_once` is idempotent.
    #[cfg(target_os = "android")]
    {
        use android_logger::{Config, FilterBuilder};
        let mut filter = FilterBuilder::new();
        filter.parse(LOG_FILTER);
        android_logger::init_once(
            Config::default()
                .with_tag("wavesync")
                .with_max_level(log::LevelFilter::Debug)
                .with_filter(filter.build()),
        );
    }
{%- endif %}

    // Per-app data directory. Must match `DbLocator`'s `app_dir` — both are
    // derived from the project name, and both are permanent once shipped.
    //
    // On iOS, an app that ships a notification-service extension needs this
    // to point at a shared App Group container instead, so the extension can
    // open the same database.
    dioxus_sdk_storage::set_dir!(dioxus_sdk_storage::data_directory().join("{{ app_dir }}"));

    // The custom-head wiring sits behind the `mobile` cargo feature because
    // `dioxus::mobile::Config` only exists when that feature is on.
    #[cfg(feature = "mobile")]
    LaunchBuilder::new()
        .with_cfg(dioxus::mobile::Config::new().with_custom_head(KEYBOARD_VIEWPORT.to_string()))
        .launch(ui::App);

    #[cfg(not(feature = "mobile"))]
    dioxus::launch(ui::App);
}
