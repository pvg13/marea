//! Android system back gesture / back button → the Dioxus router.
//!
//! Inspired by <https://github.com/CAPGAGA/dioxus-android-back-gesture-example>,
//! extended so the JNI function returns a `jboolean` meaning "handled / yield"
//! rather than always claiming the press.
//!
//! ## Why "handled / yield" matters
//!
//! `OnBackPressedCallback(true)` consumes **every** back press. Always
//! claiming handled — the naïve implementation — is fine while the router has
//! history to pop, and wrong in two situations that both mean "leave the app":
//!
//! 1. no router is mounted yet (the login screen), so nothing is listening;
//! 2. the router is at its root, so there is nothing to pop.
//!
//! With the `jboolean` return both cases bubble back to Kotlin, which decides
//! what "leave" means (see [`MainActivity`](#the-kotlin-half) below).
//!
//! ## Synchronous "can we go back?"
//!
//! The JNI function runs on the Android UI thread and cannot await Dioxus
//! state. [`GlobalBackHandler`] therefore mirrors `Navigator::can_go_back()`
//! into a plain [`AtomicBool`](std::sync::atomic::AtomicBool) on every route
//! change, and JNI makes a single atomic load to decide. Reading the route
//! inside `use_effect` is what subscribes the effect to route changes — it
//! looks like a discarded value but it is the subscription.
//!
//! The handler is route-type-agnostic (it reads `full_route_string()` rather
//! than a concrete `Route`), so the same component mounts inside any app's
//! router.
//!
//! ## State lifetime across login/logout
//!
//! Mount [`GlobalBackHandler`] **inside** the authed router — typically in the
//! layout component that also renders [`NavShell`](crate::nav::NavShell). That
//! subtree unmounts on logout and remounts on the next login, so both the
//! channel and the atomic are reset in `use_drop`; a back press in the gap
//! between the two finds a cleared bridge and yields, instead of tripping a
//! stale sender or a stale `can_go_back` value.
//!
//! ## The Kotlin half
//!
//! This module exports the symbol; the app supplies the activity that calls
//! it. In `MainActivity.kt` (package `dev.dioxus.main`, as dx generates):
//!
//! ```kotlin
//! class MainActivity : WryActivity() {
//!     // Skip WryActivity.onKeyDown's WebView-history shortcut: it calls
//!     // mWebView.goBack() whenever the embedded WebView has history, which
//!     // is a stale path for a Dioxus SPA. Disabling it sends every back
//!     // press to the dispatcher below, giving the router sole control.
//!     override val handleBackNavigation: Boolean = false
//!
//!     // Resolved at link time to the `#[unsafe(no_mangle)]` fn below.
//!     private external fun onBack(): Boolean
//!
//!     override fun onCreate(savedInstanceState: Bundle?) {
//!         super.onCreate(savedInstanceState)
//!         onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
//!             override fun handleOnBackPressed() {
//!                 if (onBack()) return
//!                 moveTaskToBack(true)
//!             }
//!         })
//!     }
//! }
//! ```
//!
//! `moveTaskToBack(true)`, **not** `finish()`. Finishing tears down the WebView
//! and any running sync engine mid-flight and crashes native-side
//! (`pthread_mutex_lock` on a destroyed mutex, which Play Vitals counts as a
//! crash). Backgrounding keeps the engine warm for a faster resume and lets the
//! OS reclaim the process with a clean `SIGKILL` when it needs the memory.
//!
//! The JNI symbol name follows the mangling convention
//! `Java_<package_with_underscores>_<class>_<method>`, so it is tied to the
//! `dev.dioxus.main` package dx generates. An app that renames the package must
//! rename nothing here — it must instead keep its `MainActivity` in that
//! package, which dx's template already does.
//!
//! ## Linking
//!
//! Nothing in Rust ever calls the exported function — only Kotlin does, at
//! runtime — so it is reachable solely by virtue of `#[unsafe(no_mangle)]`.
//! That makes it worth confirming on a real build that the symbol survives
//! into the final `.so`, particularly since marea-ui is usually a *transitive*
//! dependency of the launcher binary (`app → ui → marea-ui`) rather than a
//! direct one:
//!
//! ```sh
//! llvm-nm --defined-only <artifact> | grep MainActivity_onBack
//! ```
//!
//! A missing symbol shows up as an `UnsatisfiedLinkError` the first time the
//! user presses back, not at build time.

use dioxus::prelude::*;

#[cfg(target_os = "android")]
use std::sync::Mutex;
#[cfg(target_os = "android")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "android")]
use tokio::sync::mpsc::{self, UnboundedSender};

#[cfg(target_os = "android")]
static BACK_TX: Mutex<Option<UnboundedSender<()>>> = Mutex::new(None);

/// Mirrors `Navigator::can_go_back()`. Written reactively by
/// [`GlobalBackHandler`]; read by the JNI function on the UI thread.
#[cfg(target_os = "android")]
static CAN_GO_BACK: AtomicBool = AtomicBool::new(false);

/// JNI entry point invoked by `MainActivity.onBack()` on the Android UI
/// thread.
///
/// Returns `JNI_TRUE` when Dioxus will handle the press (the router has
/// history to pop), `JNI_FALSE` when Kotlin should fall back to its own
/// "leave the app" behaviour.
///
/// Every failure path yields rather than claiming the press: a poisoned mutex,
/// an absent sender, or a receiver dropped between unmount and reset would all
/// otherwise strand the user on a screen whose back button does nothing.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "C" fn Java_dev_dioxus_main_MainActivity_onBack(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
) -> jni::sys::jboolean {
    use jni::sys::{JNI_FALSE, JNI_TRUE};

    // No router mounted (login screen, or torn down during logout) — yield.
    if !CAN_GO_BACK.load(Ordering::Acquire) {
        return JNI_FALSE;
    }
    let Ok(guard) = BACK_TX.lock() else {
        return JNI_FALSE;
    };
    let Some(tx) = guard.as_ref() else {
        return JNI_FALSE;
    };
    if tx.send(()).is_err() {
        return JNI_FALSE;
    }
    JNI_TRUE
}

/// Bridges Android back presses into the router. Renders nothing.
///
/// Mount it once inside your `Router`, where `use_navigator()` and the route
/// context are available — see the module docs for placement and for the
/// Kotlin side that calls into it. Inert on every non-Android target, so call
/// sites need no `cfg` of their own.
#[component]
pub fn GlobalBackHandler() -> Element {
    #[cfg(target_os = "android")]
    {
        let navigator = use_navigator();

        // Reading the route string subscribes this effect to route changes;
        // the value itself is unused. `full_route_string()` rather than
        // `current::<Route>()` keeps the component generic over the app's
        // route type.
        use_effect(move || {
            let _ = router().full_route_string();
            CAN_GO_BACK.store(navigator.can_go_back(), Ordering::Release);
        });

        // One-shot: publish the sender and start the listener.
        use_hook(|| {
            let (tx, mut rx) = mpsc::unbounded_channel::<()>();
            if let Ok(mut guard) = BACK_TX.lock() {
                *guard = Some(tx);
            }
            spawn(async move {
                while rx.recv().await.is_some() {
                    // Re-check rather than trusting the atomic: it is written
                    // by an effect that may lag a very fast press.
                    if navigator.can_go_back() {
                        navigator.go_back();
                    }
                }
            });
        });

        use_drop(|| {
            CAN_GO_BACK.store(false, Ordering::Release);
            if let Ok(mut guard) = BACK_TX.lock() {
                *guard = None;
            }
        });
    }
    rsx! {}
}
