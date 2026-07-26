//! SSR tests for the pairing + scanner screens.
//!
//! `PairScreen` itself is wasm-only (browser timers), which is exactly why the
//! markup lives in the cross-target `PairScreenView` — everything asserted here
//! runs on the host with no browser.

use dioxus::prelude::*;
use marea_auth::{AuthConfig, PairingConfig, provide_auth_state};
use marea_ui::pairing::{PairDeviceSection, PairScreenView, PairState, PairingStrings};
use marea_ui::scanner::{QR_FORMATS, QrScanOverlay};

static TEST_AUTH: AuthConfig = AuthConfig {
    pocketbase_url: "http://localhost:8090",
    psk_domain: b"test.psk.v1|",
    session_storage_key: "test_session",
};

static TEST_PAIRING: PairingConfig = PairingConfig {
    url_scheme: "testapp",
    mailbox_collection: "pairing_mailboxes",
    ttl_secs: 300,
};

fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

// ── PairScreenView ──────────────────────────────────────────────────────────

#[component]
fn ViewFixture() -> Element {
    rsx! {
        PairScreenView {
            state: PairState::Waiting,
            payload: "testapp://pair?code=ABCD2345&pubkey=aGk%2B".to_string(),
            code: "ABCD2345".to_string(),
            brand: "TestApp".to_string(),
            logo: rsx! { span { "LOGO" } },
        }
    }
}

#[test]
fn pair_screen_renders_its_structure() {
    let html = render(ViewFixture);
    for cls in [
        "pair-screen",
        "pair-screen__card",
        "pair-screen__left",
        "pair-screen__right",
        "pair-steps",
        "pair-step-n",
        "pair-status",
        "qr-tile",
    ] {
        assert!(html.contains(cls), "missing {cls} in {html}");
    }
    assert!(html.contains("LOGO") && html.contains("TestApp"), "{html}");
}

/// The tile embeds the generated SVG, and the code is shown in text as the
/// human-readable backup for a camera that won't focus.
#[test]
fn pair_screen_embeds_the_qr_and_the_code() {
    let html = render(ViewFixture);
    assert!(html.contains("qr-tile__code"), "{html}");
    assert!(html.contains("<svg"), "{html}");
    assert!(html.contains("ABCD2345"), "{html}");
}

// Props carrying an `Element` are owner-scoped in Dioxus 0.7, so they can't be
// built outside a component. Feed the state through a thread-local instead —
// each `#[test]` runs on its own thread, so the cases stay isolated.
thread_local! {
    static STATE: std::cell::Cell<PairState> = const { std::cell::Cell::new(PairState::Creating) };
}

#[component]
fn StateFixture() -> Element {
    rsx! {
        PairScreenView {
            state: STATE.with(|s| s.get()),
            payload: "testapp://pair?code=A&pubkey=k".to_string(),
            code: "A".to_string(),
        }
    }
}

fn render_state(state: PairState) -> String {
    STATE.with(|s| s.set(state));
    render(StateFixture)
}

#[test]
fn every_state_renders_its_own_status_label() {
    let s = PairingStrings::default();
    let cases = [
        (PairState::Creating, s.status_creating),
        (PairState::Waiting, s.status_waiting),
        (PairState::Decrypting, s.status_decrypting),
        (PairState::Success, s.status_success),
        (PairState::Expired, s.status_expired),
        (PairState::Failed, s.status_failed),
    ];
    for (state, expected) in cases {
        let html = render_state(state);
        assert!(
            html.contains(expected),
            "state {state:?} should show {expected:?}, got {html}"
        );
    }
}

/// A spent or dead code must look spent — otherwise someone keeps pointing a
/// phone at a QR that can no longer pair.
#[test]
fn terminal_states_dim_the_tile() {
    assert!(render_state(PairState::Success).contains("qr-tile--spent"));
    assert!(render_state(PairState::Expired).contains("qr-tile--dead"));
    assert!(render_state(PairState::Failed).contains("qr-tile--dead"));
    assert!(!render_state(PairState::Waiting).contains("qr-tile--"));
}

/// Both hints are optional and only two states carry one; a hint leaking into
/// the wrong state would contradict the label above it.
#[test]
fn only_waiting_and_expired_show_a_hint() {
    let s = PairingStrings::default();
    assert!(render_state(PairState::Waiting).contains(s.status_waiting_hint));
    assert!(render_state(PairState::Expired).contains(s.status_expired_hint));
    for quiet in [
        PairState::Creating,
        PairState::Decrypting,
        PairState::Success,
        PairState::Failed,
    ] {
        assert!(
            !render_state(quiet).contains("pair-status__hint"),
            "{quiet:?} should not render a hint"
        );
    }
}

#[test]
fn is_terminal_covers_success_and_both_failures() {
    assert!(!PairState::Creating.is_terminal());
    assert!(!PairState::Waiting.is_terminal());
    assert!(!PairState::Decrypting.is_terminal());
    assert!(PairState::Success.is_terminal());
    assert!(PairState::Expired.is_terminal());
    assert!(PairState::Failed.is_terminal());
}

#[component]
fn ErrorFixture() -> Element {
    rsx! {
        PairScreenView {
            state: PairState::Failed,
            payload: "testapp://pair?code=A&pubkey=k".to_string(),
            code: "A".to_string(),
            error: "Couldn't start pairing: connection refused".to_string(),
        }
    }
}

#[test]
fn failure_detail_renders_in_an_error_banner() {
    let html = render(ErrorFixture);
    assert!(html.contains("error-banner"), "{html}");
    assert!(html.contains("connection refused"), "{html}");
}

/// The email fallback is optional: no handler, no button. An app with no
/// password flow shouldn't advertise one.
#[test]
fn email_fallback_button_appears_only_with_a_handler() {
    let s = PairingStrings::default();
    let html = render(ViewFixture);
    assert!(!html.contains(s.use_email), "{html}");

    #[component]
    fn WithFallback() -> Element {
        rsx! {
            PairScreenView {
                state: PairState::Waiting,
                payload: "testapp://pair?code=A&pubkey=k".to_string(),
                code: "A".to_string(),
                on_use_email: move |_| {},
            }
        }
    }
    assert!(render(WithFallback).contains(s.use_email));
}

// ── PairDeviceSection (phone side) ──────────────────────────────────────────

#[component]
fn DeviceFixture() -> Element {
    // Mirrors `shell_ssr.rs`: the section reads its session from AuthState.
    let session = use_signal(|| None);
    provide_auth_state(&TEST_AUTH, session);
    rsx! {
        PairDeviceSection { config: &TEST_PAIRING }
    }
}

#[test]
fn device_section_starts_idle_with_a_scan_button() {
    let s = PairingStrings::default();
    let html = render(DeviceFixture);
    assert!(html.contains("pair-device"), "{html}");
    assert!(html.contains(s.device_section_title), "{html}");
    assert!(html.contains(s.open_scanner), "{html}");
    // The camera must not mount until the user asks for it.
    assert!(!html.contains("scan-overlay"), "{html}");
}

// ── Scanner ─────────────────────────────────────────────────────────────────

#[component]
fn OverlayFixture() -> Element {
    rsx! {
        QrScanOverlay {
            title: "Pair a device".to_string(),
            hint: "Point the camera at the code".to_string(),
            on_close: move |_| {},
            on_detect: move |_: String| {},
        }
    }
}

#[test]
fn scan_overlay_renders_chrome_and_the_video_element() {
    let html = render(OverlayFixture);
    for cls in [
        "scan-overlay",
        "scan-bar",
        "scan-bar__title",
        "scan-frame__box",
        "scan-line",
        "scan-hint",
    ] {
        assert!(html.contains(cls), "missing {cls} in {html}");
    }
    assert!(html.contains("Pair a device"), "{html}");
    assert!(html.contains("Point the camera at the code"), "{html}");
    // The id the scanner JS looks up — a rename on one side only would break
    // the camera silently.
    assert!(html.contains(r#"id="marea-scanner-video""#), "{html}");
}

#[test]
fn qr_formats_are_qr_only() {
    assert_eq!(QR_FORMATS, &["qr_code"]);
}
