//! Phone side of the handoff: scan the computer's QR, confirm, seal the
//! current session into the mailbox.
//!
//! Native-only — this is the device that *holds* the credentials, and it needs
//! a camera. The wasm build gets an inert stub so call sites don't need a
//! `cfg` of their own.

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use dioxus::prelude::*;
    use marea_auth::{PairingConfig, parse_pair_url, seal_session, use_auth};

    use super::super::PairingStrings;
    use crate::components::{Button, ButtonVariant, Card, CardPad, SectionHeader};
    use crate::scanner::QrScanOverlay;

    #[derive(Clone, PartialEq, Eq)]
    enum PairPhase {
        Idle,
        Scanning,
        /// Scanned a valid link; waiting for the user to approve the handoff.
        Confirm {
            code: String,
            pubkey: String,
        },
        Sending {
            code: String,
        },
        Done,
        Failed(String),
    }

    /// "Pair a device" section for an app's settings screen.
    ///
    /// Reads the session to share from [`use_auth`], so it must be mounted
    /// inside the authenticated subtree.
    #[component]
    pub fn PairDeviceSection(
        config: &'static PairingConfig,
        #[props(default)] strings: PairingStrings,
    ) -> Element {
        let auth = use_auth();
        let mut phase = use_signal(|| PairPhase::Idle);

        rsx! {
            Card { pad: CardPad::Md, class: "pair-device".to_string(),
                SectionHeader { title: strings.device_section_title.to_string() }
                p { class: "pair-device__hint", "{strings.device_section_hint}" }

                match phase() {
                    PairPhase::Idle => rsx! {
                        Button {
                            onclick: move |_| phase.set(PairPhase::Scanning),
                            "{strings.open_scanner}"
                        }
                    },

                    PairPhase::Scanning => {
                        let strings = strings.clone();
                        rsx! {
                            QrScanOverlay {
                                title: strings.device_section_title.to_string(),
                                hint: strings.point_qr.to_string(),
                                on_close: move |_| {
                                    // Only back out of the scan itself; a
                                    // detection that already advanced the
                                    // phase must not be undone by the
                                    // overlay's teardown.
                                    if phase() == PairPhase::Scanning {
                                        phase.set(PairPhase::Idle);
                                    }
                                },
                                on_detect: move |raw: String| {
                                    match parse_pair_url(config, &raw) {
                                        Some((code, pubkey)) => {
                                            phase.set(PairPhase::Confirm { code, pubkey })
                                        }
                                        None => phase.set(
                                            PairPhase::Failed(strings.err_invalid_qr.to_string()),
                                        ),
                                    }
                                },
                            }
                        }
                    }

                    PairPhase::Confirm { code, pubkey } => {
                        let strings = strings.clone();
                        rsx! {
                            ConfirmCard {
                                code: code.clone(),
                                pubkey: pubkey.clone(),
                                strings: strings.clone(),
                                on_cancel: move |_| phase.set(PairPhase::Idle),
                                on_confirm: move |_| {
                                    let Some(session) = (auth.session)() else {
                                        phase.set(PairPhase::Failed(
                                            strings.err_no_session.to_string(),
                                        ));
                                        return;
                                    };
                                    let (code, pubkey) = (code.clone(), pubkey.clone());
                                    let strings = strings.clone();
                                    let pb = auth.client();
                                    phase.set(PairPhase::Sending { code: code.clone() });
                                    spawn(async move {
                                        let result = async {
                                            let mailbox = pb
                                                .read_pairing_mailbox(config, &code)
                                                .await
                                                .map_err(|e| {
                                                    format!("{}: {e}", strings.err_mailbox_lookup)
                                                })?
                                                .ok_or_else(|| {
                                                    strings.err_code_expired.to_string()
                                                })?;
                                            // The QR and the mailbox must name
                                            // the same key. A mismatch means a
                                            // stale QR (or a swapped one) —
                                            // sealing to it would hand the
                                            // session to the wrong recipient.
                                            if mailbox.pubkey != pubkey {
                                                return Err(
                                                    strings.err_pubkey_mismatch.to_string()
                                                );
                                            }
                                            let payload = seal_session(&pubkey, &session)
                                                .map_err(|e| {
                                                    format!("{}: {e}", strings.err_encrypt)
                                                })?;
                                            pb.fill_pairing_mailbox(config, &mailbox.id, &payload)
                                                .await
                                                .map_err(|e| {
                                                    format!("{}: {e}", strings.err_upload)
                                                })?;
                                            Ok::<(), String>(())
                                        }
                                        .await;
                                        match result {
                                            Ok(()) => phase.set(PairPhase::Done),
                                            Err(e) => {
                                                log::warn!("pairing: handoff failed: {e}");
                                                phase.set(PairPhase::Failed(e))
                                            }
                                        }
                                    });
                                },
                            }
                        }
                    }

                    PairPhase::Sending { code } => rsx! {
                        div { class: "pair-device__status",
                            "{strings.sending} "
                            span { class: "pair-device__code", "{code}" }
                        }
                    },

                    PairPhase::Done => rsx! {
                        div { class: "pair-device__status pair-device__status--ok",
                            "{strings.done}"
                        }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| phase.set(PairPhase::Idle),
                            "{strings.close}"
                        }
                    },

                    PairPhase::Failed(msg) => rsx! {
                        div { class: "error-banner", "{msg}" }
                        Button {
                            variant: ButtonVariant::Ghost,
                            onclick: move |_| phase.set(PairPhase::Idle),
                            "{strings.close}"
                        }
                    },
                }
            }
        }
    }

    /// What's about to be shared, and with whom. Shows the code and a truncated
    /// key so the user can eyeball that it matches the screen they scanned.
    #[component]
    fn ConfirmCard(
        code: String,
        pubkey: String,
        strings: PairingStrings,
        on_cancel: EventHandler<()>,
        on_confirm: EventHandler<()>,
    ) -> Element {
        let pubkey_short: String = pubkey.chars().take(12).collect();
        rsx! {
            div { class: "pair-confirm",
                div { class: "pair-confirm__text",
                    span { class: "pair-confirm__title", "{strings.confirm_title}" }
                    p { class: "pair-confirm__body", "{strings.confirm_body}" }
                }
                div { class: "pair-confirm__ids",
                    div { "{strings.code_label}: {code}" }
                    div { "key: {pubkey_short}…" }
                }
                div { class: "pair-confirm__actions",
                    Button {
                        variant: ButtonVariant::Secondary,
                        onclick: move |_| on_cancel.call(()),
                        "{strings.cancel}"
                    }
                    Button {
                        onclick: move |_| on_confirm.call(()),
                        "{strings.confirm_button}"
                    }
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::PairDeviceSection;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use dioxus::prelude::*;
    use marea_auth::PairingConfig;

    use super::super::PairingStrings;

    /// Inert on the web: a browser tab is the device being paired *to*, never
    /// the one handing a session over. Exists so app settings screens can
    /// mount `PairDeviceSection` unconditionally.
    #[component]
    pub fn PairDeviceSection(
        config: &'static PairingConfig,
        #[props(default)] strings: PairingStrings,
    ) -> Element {
        let _ = (config, strings);
        rsx! {}
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::PairDeviceSection;
