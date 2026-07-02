//! {{project-name}} — a marea app.
//!
//! Everything infrastructural (auth gate, session persistence, navigation
//! shell, components, toasts, theming) comes from the framework; this file
//! plus your screens is the whole app.
//!
//!   dx serve --platform desktop
//!   dx serve --platform web --features web --no-default-features

use dioxus::prelude::*;
use marea_auth::AuthConfig;
use marea_ui::{AppShell, Card, CardPad, NavItem, NavShell, ScreenHeader, TopBar, MAREA_CSS};

/// Compatibility invariants — once this ships, never change the PSK domain
/// or the storage key (existing users' sync passphrases/sessions depend on
/// them).
static AUTH: AuthConfig = AuthConfig {
    pocketbase_url: "https://admin.almares.es",
    psk_domain: b"{{project-name}}.psk.v1|",
    session_storage_key: "{{project-name}}_auth_session",
};

/// Your skin: edit assets/tokens.css (see marea's docs/theme-contract.md).
const TOKENS_CSS: Asset = asset!("/assets/tokens.css");

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_sdk_storage::set_dir!(dioxus_sdk_storage::data_directory().join("{{project-name}}"));
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Stylesheet { href: MAREA_CSS }
        document::Stylesheet { href: TOKENS_CSS }
        AppShell {
            auth: &AUTH,
            login: rsx! {
                marea_ui::LoginScreen { title: "Welcome to {{project-name}}" }
            },
            Router::<Route> {}
        }
    }
}

#[derive(Clone, Routable, PartialEq, Debug)]
enum Route {
    #[layout(Shell)]
    #[route("/")]
    Home {},
    #[route("/settings")]
    Settings {},
}

#[component]
fn Shell() -> Element {
    rsx! {
        NavShell {
            items: vec![
                NavItem::new("Home", icon("M3 11l9-8 9 8M5 10v10a1 1 0 001 1h4v-6h4v6h4a1 1 0 001-1V10"), Route::Home {}),
                NavItem::new("Settings", icon("M12 2v3m0 14v3M4.9 4.9l2.1 2.1m10 10l2.1 2.1M2 12h3m14 0h3M4.9 19.1l2.1-2.1m10-10l2.1-2.1M12 9a3 3 0 100 6 3 3 0 000-6z"), Route::Settings {}),
            ],
            brand: rsx! { "{{project-name}}" },
            Outlet::<Route> {}
        }
    }
}

#[component]
fn Home() -> Element {
    rsx! {
        TopBar { title: "Home" }
        div { style: "padding:16px;",
            ScreenHeader { title: "It works", subtitle: "Now build the part only this app can be." }
            Card { pad: CardPad::Lg, "Your domain goes here." }
        }
    }
}

#[component]
fn Settings() -> Element {
    let auth = marea_auth::use_auth();
    rsx! {
        TopBar { title: "Settings" }
        div { style: "padding:16px;",
            Card { pad: CardPad::Md,
                marea_ui::Button {
                    variant: marea_ui::ButtonVariant::Outline,
                    onclick: move |_| auth.logout(),
                    "Sign out"
                }
            }
        }
    }
}

fn icon(path: &'static str) -> Element {
    rsx! {
        svg {
            width: "22", height: "22", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round",
            path { d: "{path}" }
        }
    }
}
