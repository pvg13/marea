//! Host-side guarantees for the Android back-gesture bridge.
//!
//! The bridge itself can only be exercised on a device — what's checked here is
//! the contract every *other* target depends on: that mounting the handler is
//! free and safe. The Android branch is verified by compiling for
//! `aarch64-linux-android` and confirming the JNI symbol is exported (see the
//! module docs).

use dioxus::prelude::*;
use marea_ui::back_gesture::GlobalBackHandler;

fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[component]
fn Bare() -> Element {
    rsx! {
        GlobalBackHandler {}
    }
}

/// Off Android the component must render nothing at all, and must do so
/// without a `Router` in scope.
///
/// Both matter because apps mount it unconditionally in their shell: stray
/// markup would land in every desktop and web build, and `use_navigator()`
/// panics without router context — which is precisely why the Android branch
/// is the only place it is called.
#[test]
fn renders_nothing_and_needs_no_router_off_android() {
    assert_eq!(render(Bare), "");
}

#[component]
fn InsideLayout() -> Element {
    rsx! {
        div { class: "app-shell",
            GlobalBackHandler {}
            main { "content" }
        }
    }
}

/// Mounting it must not disturb its siblings.
#[test]
fn does_not_affect_surrounding_markup() {
    let html = render(InsideLayout);
    assert!(html.contains("<main>content</main>"), "{html}");
    assert!(!html.contains("back"), "{html}");
}
