//! The URL the app was opened with (`deeplink` feature).
//!
//! Android hands an app a URL two ways and they are not interchangeable: a
//! **cold** open arrives on the launch intent, before any Rust has run, and a
//! **warm** one arrives at `onNewIntent` with the app already on screen. A
//! handler that only covers the second misses every tap on a notification the
//! app was not already running for — which is most of them.
//!
//! So this buffers. A link that arrives with nothing mounted is held until a
//! [`DeeplinkHandler`] appears and drains it, which means the Kotlin half can
//! push from `onCreate` without caring whether Dioxus has started yet, and an
//! app whose router lives behind a login gate still lands on the right screen
//! once the gate is passed.
//!
//! ## What it does *not* do
//!
//! Decide what a URL means. The handler is given the string; mapping it to a
//! route is the app's, because only the app knows its routes — and because a
//! pure `&str -> Option<Route>` is testable without a device, which is where
//! that logic belongs.
//!
//! A URL this app does not recognise is therefore not an error here. Apps
//! share a scheme across features — marea's own pairing uses one — so
//! ignoring what you do not recognise is the normal case, not a failure.
//!
//! ## The Kotlin half
//!
//! This module exports the symbol; the app supplies the activity that calls
//! it, exactly as [`crate::back_gesture`] does. In `MainActivity.kt` (package
//! `dev.dioxus.main`, as dx generates):
//!
//! ```kotlin
//! class MainActivity : WryActivity() {
//!     private external fun onDeeplink(url: String)
//!
//!     override fun onCreate(savedInstanceState: Bundle?) {
//!         super.onCreate(savedInstanceState)
//!         deliver(intent)
//!     }
//!
//!     // A warm tap. Without `setIntent` the activity keeps reporting the
//!     // intent it was created with, so anything reading `getIntent()` later
//!     // sees a stale URL.
//!     override fun onNewIntent(intent: Intent) {
//!         super.onNewIntent(intent)
//!         setIntent(intent)
//!         deliver(intent)
//!     }
//!
//!     // Wrapped: the symbol resolves lazily on first call, and an app opened
//!     // by a link before the library finished loading would otherwise die of
//!     // an UnsatisfiedLinkError on its own launch path.
//!     private fun deliver(intent: Intent?) {
//!         val url = intent?.data?.toString() ?: return
//!         try {
//!             onDeeplink(url)
//!         } catch (e: Throwable) {
//!             Log.w("MareaDeeplink", "could not deliver '$url': ${e.message}")
//!         }
//!     }
//! }
//! ```
//!
//! ## Linking
//!
//! Nothing in Rust calls the exported function — only Kotlin does — so it is
//! reachable solely by `#[unsafe(no_mangle)]`, and worth confirming on a real
//! build, especially as marea-ui is usually a *transitive* dependency:
//!
//! ```sh
//! llvm-nm --defined-only <artifact> | grep MainActivity_onDeeplink
//! ```

use dioxus::prelude::*;

#[cfg(target_os = "android")]
use std::sync::Mutex;
#[cfg(target_os = "android")]
use tokio::sync::mpsc::{self, UnboundedSender};

/// The mounted handler's sender, if one is mounted.
#[cfg(target_os = "android")]
static LINK_TX: Mutex<Option<UnboundedSender<String>>> = Mutex::new(None);

/// A link that arrived with nothing mounted to receive it.
///
/// One slot, not a queue: these come from a person tapping something, and two
/// taps before the app has started mean they want the second one.
#[cfg(target_os = "android")]
static PENDING: Mutex<Option<String>> = Mutex::new(None);

/// JNI entry point invoked by `MainActivity` for both the launch intent and
/// any later one.
///
/// Every failure path drops the link rather than panicking: this runs on the
/// Android UI thread inside the app's own startup, and an unwind here would
/// take the launch with it.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_dev_dioxus_main_MainActivity_onDeeplink(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    url: jni::objects::JString,
) {
    let Ok(url) = env.get_string(&url) else {
        return;
    };
    let url: String = url.into();
    if url.is_empty() {
        return;
    }

    if let Ok(guard) = LINK_TX.lock()
        && let Some(tx) = guard.as_ref()
        && tx.send(url.clone()).is_ok()
    {
        return;
    }
    // Nothing mounted yet — hold it for the handler that is about to appear.
    if let Ok(mut pending) = PENDING.lock() {
        *pending = Some(url);
    }
}

/// Delivers the URL the app was opened with. Renders nothing.
///
/// Mount it once inside your `Router`, alongside
/// [`GlobalBackHandler`](crate::back_gesture::GlobalBackHandler). `on_link`
/// fires once per link — on mount for anything that arrived first, and
/// immediately for anything that arrives while mounted. Inert on every
/// non-Android target, so call sites need no `cfg` of their own.
#[component]
pub fn DeeplinkHandler(on_link: EventHandler<String>) -> Element {
    #[cfg(target_os = "android")]
    {
        use_hook(|| {
            let (tx, mut rx) = mpsc::unbounded_channel::<String>();
            if let Ok(mut guard) = LINK_TX.lock() {
                *guard = Some(tx);
            }
            // Anything that arrived before this mounted — the cold-start case,
            // which is every tap on a notification for an app that was not
            // already running.
            let buffered = PENDING.lock().ok().and_then(|mut p| p.take());

            spawn(async move {
                if let Some(url) = buffered {
                    on_link.call(url);
                }
                while let Some(url) = rx.recv().await {
                    on_link.call(url);
                }
            });
        });

        use_drop(|| {
            if let Ok(mut guard) = LINK_TX.lock() {
                *guard = None;
            }
        });
    }
    #[cfg(not(target_os = "android"))]
    let _ = on_link;
    rsx! {}
}
