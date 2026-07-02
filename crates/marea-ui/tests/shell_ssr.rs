//! SSR structure tests for the shell-layer pieces that don't need a router.

use dioxus::prelude::*;
use marea_auth::{AuthConfig, provide_auth_state};

static TEST_AUTH: AuthConfig = AuthConfig {
    pocketbase_url: "http://localhost:8090",
    psk_domain: b"test.psk.v1|",
    session_storage_key: "test_session",
};

fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[component]
fn LoginFixture() -> Element {
    // LoginScreen reads AuthState from context; provide it with an
    // in-memory (non-persistent) session signal.
    let session = use_signal(|| None);
    provide_auth_state(&TEST_AUTH, session);
    rsx! {
        marea_ui::LoginScreen {
            title: "Welcome to TestApp",
            subtitle: "Log in to sync.",
            logo: rsx! { span { "LOGO" } },
        }
    }
}

#[test]
fn login_screen_renders_branded_structure() {
    let html = render(LoginFixture);
    assert!(html.contains("auth-screen"), "{html}");
    assert!(html.contains("auth-screen__logo") && html.contains("LOGO"));
    assert!(html.contains("Welcome to TestApp"));
    assert!(html.contains("Log in to sync."));
    assert!(html.contains(r#"type="email""#));
    assert!(html.contains(r#"type="password""#));
    assert!(html.contains("btn-primary"));
    assert!(
        html.contains("Create account"),
        "register enabled by default"
    );
    assert!(!html.contains("error-banner"), "no error initially");
}

#[component]
fn LoginNoRegisterFixture() -> Element {
    let session = use_signal(|| None);
    provide_auth_state(&TEST_AUTH, session);
    rsx! {
        marea_ui::LoginScreen { allow_register: false }
    }
}

#[test]
fn login_screen_can_hide_register() {
    let html = render(LoginNoRegisterFixture);
    assert!(!html.contains("Create account"));
}

#[component]
fn ShellFixture() -> Element {
    rsx! {
        marea_ui::AppShell {
            auth: &TEST_AUTH,
            phone_frame: true,
            div { "AUTHED CONTENT" }
        }
    }
}

#[test]
fn app_shell_unauthenticated_shows_login_in_phone_frame() {
    // AppShell persists the session via dioxus-sdk-storage, which requires a
    // data dir before first access — exactly what real apps set in main().
    let dir = std::env::temp_dir().join(format!("marea-ui-shell-ssr-{}", std::process::id()));
    dioxus_sdk_storage::set_directory(dir);

    // No persisted session in the test environment → login screen. The
    // phone-frame wrapper and toast host must be present either way.
    let html = render(ShellFixture);
    assert!(html.contains("app-frame--phone"), "{html}");
    assert!(html.contains("toast-host"));
    assert!(html.contains("auth-screen"));
    assert!(!html.contains("AUTHED CONTENT"));
}
