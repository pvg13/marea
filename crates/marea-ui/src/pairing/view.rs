//! The web pairing screen's presentation, with no I/O.
//!
//! Split out of [`PairScreen`](super::PairScreen) — which is wasm-only, since
//! it polls with browser timers — so the layout and every status state can be
//! rendered and asserted on the host (see `tests/pairing_ssr.rs`). The driver
//! owns the keypair, the mailbox and the polling; this owns the markup.

use dioxus::prelude::*;

use super::{PairingStrings, QrTile};

/// Where the handoff has got to. Drives the status line and the tile's
/// dimming; the three terminal states (`Success`, `Expired`, `Failed`) stop
/// the poll loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PairState {
    /// Creating the mailbox — the QR isn't live yet.
    #[default]
    Creating,
    /// Mailbox up, polling for the phone's payload.
    Waiting,
    /// Payload arrived, unsealing it.
    Decrypting,
    /// Session persisted; the shell is about to swap to the authed subtree.
    Success,
    /// TTL elapsed, or the mailbox vanished, with no payload.
    Expired,
    /// Unrecoverable — the accompanying error message says why.
    Failed,
}

impl PairState {
    /// Whether the poll loop should stop. Not `is_error`: `Success` also ends
    /// the loop.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Success | Self::Expired | Self::Failed)
    }

    /// Status label and optional hint for this state.
    fn labels(&self, s: &PairingStrings) -> (&'static str, &'static str) {
        match self {
            Self::Creating => (s.status_creating, ""),
            Self::Waiting => (s.status_waiting, s.status_waiting_hint),
            Self::Decrypting => (s.status_decrypting, ""),
            Self::Success => (s.status_success, ""),
            Self::Expired => (s.status_expired, s.status_expired_hint),
            Self::Failed => (s.status_failed, ""),
        }
    }
}

/// Two-column pairing card: what-to-do on the left, the live code on the
/// right. Collapses to one column under 900px (`marea.css`).
#[component]
pub fn PairScreenView(
    state: PairState,
    /// The `app://pair?…` URL encoded into the QR.
    payload: String,
    /// The pairing code, shown under the tile as a human-readable backup.
    code: String,
    #[props(default)] strings: PairingStrings,
    /// Failure detail, rendered above the status line.
    #[props(default)] error: Option<String>,
    /// Brand mark for the top bar.
    #[props(default)] logo: Option<Element>,
    /// Brand name for the top bar.
    #[props(default)] brand: Option<String>,
    /// Smaller mark floated over the QR's centre.
    #[props(default)] qr_badge: Option<Element>,
    /// Offers the email/password fallback when set.
    #[props(default)] on_use_email: Option<EventHandler<()>>,
) -> Element {
    let (label, hint) = state.labels(&strings);

    rsx! {
        div { class: "pair-screen",
            if logo.is_some() || brand.is_some() {
                div { class: "pair-screen__top",
                    if let Some(logo) = logo {
                        span { class: "pair-screen__mark", {logo} }
                    }
                    if let Some(brand) = brand {
                        span { class: "pair-screen__brand", "{brand}" }
                    }
                }
            }

            div { class: "pair-screen__card",
                div { class: "pair-screen__left",
                    h1 { class: "pair-screen__title",
                        "{strings.title_line_1}"
                        br {}
                        "{strings.title_line_2}"
                    }
                    p { class: "pair-screen__lede", "{strings.lede}" }
                    ol { class: "pair-steps",
                        li {
                            span { class: "pair-step-n", "1" }
                            span { "{strings.step_1}" }
                        }
                        li {
                            span { class: "pair-step-n", "2" }
                            span { "{strings.step_2}" }
                        }
                        li {
                            span { class: "pair-step-n", "3" }
                            span { "{strings.step_3}" }
                        }
                    }
                    div { class: "pair-screen__shield",
                        ShieldIcon {}
                        span { "{strings.shield_note}" }
                    }
                }

                div { class: "pair-screen__right",
                    QrTile {
                        payload,
                        state,
                        logo: qr_badge,
                        failed_label: strings.qr_failed.to_string(),
                    }

                    if let Some(err) = error {
                        div { class: "error-banner pair-screen__error", "{err}" }
                    }

                    div { class: "pair-status",
                        div { class: "pair-status__label", "{label}" }
                        if !hint.is_empty() {
                            div { class: "pair-status__hint", "{hint}" }
                        }
                        div { class: "pair-status__code", "{strings.code_label}: {code}" }
                    }

                    if let Some(on_use_email) = on_use_email {
                        button {
                            r#type: "button",
                            class: "btn-ghost btn-sm",
                            onclick: move |_| on_use_email.call(()),
                            "{strings.use_email}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ShieldIcon() -> Element {
    rsx! {
        svg {
            class: "pair-screen__shield-icon",
            width: "16",
            height: "16",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.8",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            path { d: "M12 3l7 3v6c0 4.4-3 7.8-7 9-4-1.2-7-4.6-7-9V6l7-3z" }
            path { d: "M9 12l2 2 4-4" }
        }
    }
}
