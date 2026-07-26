//! marea showcase — a real app exercising the framework end-to-end:
//! AppShell (auth gate against the shared PocketBase), NavShell with three
//! tabs, the full component gallery, toasts and the theme toggle.
//!
//! Run from this directory:
//!   dx serve --platform desktop
//!   dx serve --platform web

use dioxus::prelude::*;
use marea_auth::AuthConfig;
use marea_ui::{
    use_theme, use_toasts, AppShell, Avatar, AvatarSize, Button, ButtonVariant, Card, CardPad,
    Checkbox, Divider, EmptyState, Eyebrow, InfoRow, Input, LoadingSkeleton, NavItem, NavShell,
    Pill, PillTone, Row, ScreenHeader, SectionHeader, ThemeMode, TopBar, MAREA_CSS,
};

static AUTH: AuthConfig = AuthConfig {
    pocketbase_url: "https://admin.almares.es",
    psk_domain: b"marea-showcase.psk.v1|",
    session_storage_key: "marea_showcase_session",
};

const TOKENS_CSS: Asset = asset!("/assets/tokens.css");

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    dioxus_sdk_storage::set_dir!(dioxus_sdk_storage::data_directory().join("MareaShowcase"));
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
                marea_ui::LoginScreen {
                    title: "marea showcase",
                    subtitle: "Log in with any account on the shared PocketBase — or create a throwaway one.",
                    logo: rsx! { WaveLogo {} },
                }
            },
            Router::<Route> {}
        }
    }
}

#[derive(Clone, Routable, PartialEq, Debug)]
enum Route {
    #[layout(Shell)]
    #[route("/")]
    Gallery {},
    #[route("/forms")]
    Forms {},
    #[route("/pairing")]
    PairingDemo {},
    #[route("/settings")]
    SettingsPage {},
}

fn nav_items() -> Vec<NavItem<Route>> {
    vec![
        NavItem::new("Gallery", rsx! { GridIcon {} }, Route::Gallery {}),
        NavItem::new("Forms", rsx! { InputIcon {} }, Route::Forms {}),
        NavItem::new("Pairing", rsx! { QrIcon {} }, Route::PairingDemo {}),
        NavItem::new("Settings", rsx! { GearIcon {} }, Route::SettingsPage {}),
    ]
}

#[component]
fn Shell() -> Element {
    rsx! {
        NavShell {
            items: nav_items(),
            brand: rsx! { "marea" },
            footer: rsx! { ThemeToggle {} SignOut {} },
            Outlet::<Route> {}
        }
    }
}

#[component]
fn ThemeToggle() -> Element {
    let mut theme = use_theme();
    let label = match theme.mode() {
        ThemeMode::Light => "Dark mode",
        ThemeMode::Dark => "Light mode",
    };
    rsx! {
        Button {
            variant: ButtonVariant::Ghost,
            small: true,
            onclick: move |_| theme.toggle(),
            "{label}"
        }
    }
}

#[component]
fn SignOut() -> Element {
    let auth = marea_auth::use_auth();
    rsx! {
        Button {
            variant: ButtonVariant::Outline,
            small: true,
            onclick: move |_| auth.logout(),
            "Sign out"
        }
    }
}

#[component]
fn Gallery() -> Element {
    rsx! {
        div { class: "page",
            TopBar {
                title: "Component gallery",
                actions: rsx! { ThemeToggle {} },
            }
            div { style: "padding: 16px; display: flex; flex-direction: column; gap: 16px;",
                ScreenHeader {
                    eyebrow: "marea-ui",
                    title: "Everything themable",
                    subtitle: "Swap the token file and all of this reskins.",
                }

                SectionHeader { title: "Buttons" }
                Card { pad: CardPad::Md,
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;",
                        Button { "Primary" }
                        Button { variant: ButtonVariant::Secondary, "Secondary" }
                        Button { variant: ButtonVariant::Soft, "Soft" }
                        Button { variant: ButtonVariant::Ghost, "Ghost" }
                        Button { variant: ButtonVariant::Outline, "Outline" }
                        Button { variant: ButtonVariant::Danger, "Danger" }
                        Button { small: true, "Small" }
                        Button { disabled: true, "Disabled" }
                    }
                }

                SectionHeader { title: "Pills" }
                Card { pad: CardPad::Md,
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;",
                        Pill { "Neutral" }
                        Pill { tone: PillTone::Primary, "Primary" }
                        Pill { tone: PillTone::Accent, "Accent" }
                        Pill { tone: PillTone::Success, "Success" }
                        Pill { tone: PillTone::Warning, "Warning" }
                        Pill { tone: PillTone::Danger, "Danger" }
                        Pill { tone: PillTone::Ghost, "Ghost" }
                    }
                }

                SectionHeader { title: "Rows & lists" }
                Card { pad: CardPad::Md,
                    Row {
                        icon: rsx! { Avatar { name: "Pablo Vazquez", size: AvatarSize::Sm } },
                        title: "Pablo Vazquez",
                        subtitle: "pablo@almares.es",
                        trailing: rsx! { Pill { tone: PillTone::Success, "online" } },
                        onclick: move |_| {},
                    }
                    Row {
                        title: "Plain row",
                        subtitle: "No icon, not interactive",
                    }
                    Divider {}
                    InfoRow { label: "Version", value: "0.1.0" }
                    InfoRow { label: "Crates", value: "auth · sync · ui" }
                }

                SectionHeader { title: "States" }
                Card { pad: CardPad::Md,
                    Eyebrow { "Loading" }
                    LoadingSkeleton { lines: 3 }
                    Divider {}
                    EmptyState {
                        title: "Nothing here yet",
                        description: "This is the shared empty state. The icon slot takes any element.",
                        action: rsx! { Button { small: true, "Do something" } },
                    }
                }
            }
        }
    }
}

#[component]
fn Forms() -> Element {
    let mut name = use_signal(String::new);
    let mut agreed = use_signal(|| false);
    let toasts = use_toasts();
    rsx! {
        div { class: "page",
            TopBar { title: "Forms & feedback" }
            div { style: "padding: 16px; display: flex; flex-direction: column; gap: 16px;",
                Card { pad: CardPad::Lg,
                    div { style: "display:flex;flex-direction:column;gap:12px;",
                        Input {
                            label: "Name",
                            placeholder: "Your name",
                            oninput: move |e: FormEvent| name.set(e.value()),
                        }
                        Input {
                            label: "With an error",
                            error: "This field is having a bad day.",
                            placeholder: "Anything",
                        }
                        Checkbox {
                            label: "I agree to be showcased",
                            checked: agreed(),
                            onchange: move |_| agreed.set(!agreed()),
                        }
                        div { style: "display:flex;gap:8px;",
                            Button {
                                onclick: move |_| toasts.success(format!("Saved, {}!", name())),
                                "Success toast"
                            }
                            Button {
                                variant: ButtonVariant::Danger,
                                onclick: move |_| toasts.error("Something broke (on purpose)."),
                                "Error toast"
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| toasts.info("Just so you know."),
                                "Info toast"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsPage() -> Element {
    let auth = marea_auth::use_auth();
    let email = auth
        .session
        .read()
        .as_ref()
        .map(|s| s.email.clone())
        .unwrap_or_default();
    rsx! {
        div { class: "page",
            TopBar { title: "Settings" }
            div { style: "padding: 16px; display: flex; flex-direction: column; gap: 16px;",
                Card { pad: CardPad::Md,
                    InfoRow { label: "Signed in as", value: "{email}" }
                    InfoRow { label: "PocketBase", value: "admin.almares.es" }
                }
                Card { pad: CardPad::Md,
                    div { style: "display:flex;gap:8px;",
                        ThemeToggle {}
                        SignOut {}
                    }
                }
            }
        }
    }
}

// ── Pairing demo ─────────────────────────────────────────────────────────────

static PAIRING: marea_auth::PairingConfig = marea_auth::PairingConfig {
    url_scheme: "mareashowcase",
    mailbox_collection: "pairing_mailboxes",
    ttl_secs: 300,
};

/// Renders the pairing screen in every state, plus the scan overlay.
///
/// A **render demo, not a live pairing**: the QR below encodes a throwaway key
/// that no phone is waiting on. A real handoff needs two devices running the
/// *same* app — the sealed payload carries that app's session and PSK, so
/// pairing across apps is meaningless even though they share a PocketBase.
/// The live driver is `marea_ui::pairing::PairScreen`, and it only exists on
/// the web build.
#[component]
fn PairingDemo() -> Element {
    use marea_ui::pairing::{PairScreenView, PairState};

    // Generated once so the QR doesn't churn on every state change.
    let keys = use_hook(marea_auth::WebPairingKeys::generate);
    let code = use_hook(marea_auth::generate_code);
    let payload = marea_auth::pair_url(&PAIRING, &code, &keys.pubkey_b64());

    let mut state = use_signal(|| PairState::Waiting);
    let mut scanning = use_signal(|| false);
    // Hooks run at the component top, never inside a handler.
    let toasts = use_toasts();

    let states = [
        ("Creating", PairState::Creating),
        ("Waiting", PairState::Waiting),
        ("Decrypting", PairState::Decrypting),
        ("Success", PairState::Success),
        ("Expired", PairState::Expired),
        ("Failed", PairState::Failed),
    ];

    rsx! {
        div { class: "page",
            TopBar { title: "Pairing" }
            div { style: "padding: 16px; display: flex; flex-direction: column; gap: 16px;",
                Card { pad: CardPad::Md,
                    Eyebrow { "Render demo" }
                    p { style: "margin:8px 0 12px;",
                        "This draws every state of the pairing screen. It is not a live pairing — no phone is waiting on this code, and pairing only ever works between two devices running the same app."
                    }
                    div { style: "display:flex;flex-wrap:wrap;gap:8px;",
                        for (label, s) in states {
                            Button {
                                small: true,
                                variant: if state() == s { ButtonVariant::Primary } else { ButtonVariant::Secondary },
                                onclick: move |_| state.set(s),
                                "{label}"
                            }
                        }
                        Button {
                            small: true,
                            variant: ButtonVariant::Outline,
                            onclick: move |_| scanning.set(true),
                            "Open scanner"
                        }
                    }
                }

                PairScreenView {
                    state: state(),
                    payload,
                    code,
                    brand: "marea showcase".to_string(),
                    logo: rsx! { WaveLogo {} },
                    error: if state() == PairState::Failed {
                        Some("Couldn't start pairing: connection refused".to_string())
                    } else {
                        None
                    },
                }
            }

            if scanning() {
                marea_ui::scanner::QrScanOverlay {
                    title: "Scan a pairing code".to_string(),
                    hint: "Point the camera at the code on the other screen".to_string(),
                    on_close: move |_| scanning.set(false),
                    on_detect: move |scanned: String| {
                        scanning.set(false);
                        toasts.info(format!("Scanned: {scanned}"));
                    },
                }
            }
        }
    }
}

// ── currentColor SVG icons ───────────────────────────────────────────────────

#[component]
fn WaveLogo() -> Element {
    rsx! {
        svg {
            width: "48", height: "48", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round",
            path { d: "M2 12c2.5-3 5-3 7.5 0s5 3 7.5 0 3.5-2 5 0M2 17c2.5-3 5-3 7.5 0s5 3 7.5 0 3.5-2 5 0M2 7c2.5-3 5-3 7.5 0s5 3 7.5 0 3.5-2 5 0" }
        }
    }
}

#[component]
fn GridIcon() -> Element {
    rsx! {
        svg {
            width: "22", height: "22", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round",
            rect { x: "3", y: "3", width: "7", height: "7", rx: "1.5" }
            rect { x: "14", y: "3", width: "7", height: "7", rx: "1.5" }
            rect { x: "3", y: "14", width: "7", height: "7", rx: "1.5" }
            rect { x: "14", y: "14", width: "7", height: "7", rx: "1.5" }
        }
    }
}

#[component]
fn InputIcon() -> Element {
    rsx! {
        svg {
            width: "22", height: "22", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round",
            rect { x: "3", y: "7", width: "18", height: "10", rx: "2" }
            path { d: "M7 12h.01" }
        }
    }
}

#[component]
fn QrIcon() -> Element {
    rsx! {
        svg {
            width: "22", height: "22", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round",
            rect { x: "3", y: "3", width: "7", height: "7", rx: "1.5" }
            rect { x: "3", y: "14", width: "7", height: "7", rx: "1.5" }
            rect { x: "14", y: "3", width: "7", height: "7", rx: "1.5" }
            path { d: "M14 14h3v3h-3zM20 14v3M14 20h3M20 20h1" }
        }
    }
}

#[component]
fn GearIcon() -> Element {
    rsx! {
        svg {
            width: "22", height: "22", view_box: "0 0 24 24", fill: "none",
            stroke: "currentColor", stroke_width: "1.8", stroke_linecap: "round",
            circle { cx: "12", cy: "12", r: "3" }
            path { d: "M12 2v3m0 14v3M4.9 4.9l2.1 2.1m10 10l2.1 2.1M2 12h3m14 0h3M4.9 19.1l2.1-2.1m10-10l2.1-2.1" }
        }
    }
}
