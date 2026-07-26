//! Web side of the handoff: mint a keypair, publish a mailbox, show the QR,
//! poll until the phone fills it.
//!
//! wasm-only. Not because the crypto is browser-specific — it isn't — but
//! because the flow exists to spare someone typing a password into a computer,
//! and because the poll loop uses browser timers (`std::time::Instant` panics
//! on `wasm32-unknown-unknown`, and there's no tokio runtime here). The markup
//! lives in [`PairScreenView`](super::PairScreenView), which is cross-target.

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use marea_auth::{
    AuthSession, MailboxRecord, PairingConfig, PocketBase, WebPairingKeys, generate_code, pair_url,
    unseal_session, use_auth,
};

use super::{PairScreenView, PairState, PairingStrings};

/// How often the tab re-reads the mailbox. 2 s balances perceived latency
/// against load: the phone side needs a human to tap confirm, so polling
/// faster doesn't make pairing feel quicker.
const POLL_INTERVAL_MS: u32 = 2_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Qr,
    Email,
}

/// Pre-login screen for the web build: QR pairing, with an email/password
/// escape hatch.
///
/// Pass it to [`AppShell`](crate::shell::AppShell)'s `login` slot — it reads
/// and writes the session through [`use_auth`], so a successful pairing makes
/// the shell swap to the authed subtree with no extra wiring.
///
/// Switching to the email fallback and back mints a **fresh** keypair, code and
/// mailbox. That's deliberate: the abandoned code should stop working.
#[component]
pub fn PairScreen(
    config: &'static PairingConfig,
    #[props(default)] strings: PairingStrings,
    /// Brand mark for the top bar.
    #[props(default)] logo: Option<Element>,
    /// Brand name for the top bar.
    #[props(default)] brand: Option<String>,
    /// Smaller mark floated over the QR's centre.
    #[props(default)] qr_badge: Option<Element>,
    /// Shown when the user picks "sign in with email". Defaults to
    /// [`LoginScreen`](crate::auth_screens::LoginScreen). Pass `None` to
    /// offer no fallback at all — the button then disappears.
    #[props(default = Some(rsx! { crate::auth_screens::LoginScreen {} }))]
    email_fallback: Option<Element>,
) -> Element {
    let mut mode = use_signal(|| Mode::Qr);
    let has_fallback = email_fallback.is_some();

    if mode() == Mode::Email && let Some(fallback) = email_fallback {
        return rsx! {
            div { class: "pair-screen__fallback",
                {fallback}
                button {
                    r#type: "button",
                    class: "btn-ghost btn-sm pair-screen__back",
                    onclick: move |_| mode.set(Mode::Qr),
                    "{strings.use_qr}"
                }
            }
        };
    }

    rsx! {
        QrPairing {
            config,
            strings,
            logo,
            brand,
            qr_badge,
            on_use_email: if has_fallback {
                Some(EventHandler::new(move |_| mode.set(Mode::Email)))
            } else {
                None
            },
        }
    }
}

#[component]
fn QrPairing(
    config: &'static PairingConfig,
    strings: PairingStrings,
    logo: Option<Element>,
    brand: Option<String>,
    qr_badge: Option<Element>,
    on_use_email: Option<EventHandler<()>>,
) -> Element {
    let auth = use_auth();

    // `use_hook`, NOT `use_signal`: every poll tick re-renders this component,
    // and a keypair regenerated mid-flight would leave us unable to decrypt the
    // payload the phone sealed against the pubkey already in the QR.
    let keys = use_hook(WebPairingKeys::generate);
    let code = use_hook(generate_code);

    let pubkey_b64 = keys.pubkey_b64();
    let payload = pair_url(config, &code, &pubkey_b64);

    let state = use_signal(PairState::default);
    let error = use_signal(|| Option::<String>::None);

    // Mailbox lifecycle, once per mount: create, then poll to a terminal state.
    {
        let code = code.clone();
        let pubkey_b64 = pubkey_b64.clone();
        let keys = keys.clone();
        let strings = strings.clone();
        let client = auth.client();
        let session = auth.session;
        use_hook(move || {
            spawn(async move {
                run_pairing(
                    client, config, code, pubkey_b64, keys, strings, state, error, session,
                )
                .await;
            });
        });
    }

    rsx! {
        PairScreenView {
            state: state(),
            payload,
            code,
            strings,
            error: error(),
            logo,
            brand,
            qr_badge,
            on_use_email,
        }
    }
}

/// Create the mailbox, then poll it. Split from the component so the whole
/// flow is one linear function rather than nested closures.
#[allow(clippy::too_many_arguments)]
async fn run_pairing(
    pb: PocketBase,
    cfg: &'static PairingConfig,
    code: String,
    pubkey_b64: String,
    keys: WebPairingKeys,
    strings: PairingStrings,
    mut state: Signal<PairState>,
    mut error: Signal<Option<String>>,
    session: Signal<Option<AuthSession>>,
) {
    let record = match pb.create_pairing_mailbox(cfg, &code, &pubkey_b64).await {
        Ok(r) => r,
        Err(e) => {
            log::error!("pairing: create_pairing_mailbox failed: {e}");
            error.set(Some(format!("{}: {e}", strings.err_mailbox_create)));
            state.set(PairState::Failed);
            return;
        }
    };
    state.set(PairState::Waiting);
    poll_until_filled(&pb, cfg, &record, &keys, &strings, state, error, session).await;
}

/// Drain the mailbox until the phone fills it or the TTL fires.
///
/// Owns the session write on the happy path, so the shell's reactivity does the
/// rest. Transient read errors are logged and retried — a flaky network while
/// someone walks to their phone is expected; a persistent failure surfaces as
/// the TTL expiring rather than as a scary error.
#[allow(clippy::too_many_arguments)]
async fn poll_until_filled(
    pb: &PocketBase,
    cfg: &PairingConfig,
    record: &MailboxRecord,
    keys: &WebPairingKeys,
    strings: &PairingStrings,
    mut state: Signal<PairState>,
    mut error: Signal<Option<String>>,
    mut session: Signal<Option<AuthSession>>,
) {
    let deadline = js_sys::Date::now() + (cfg.ttl_secs as f64) * 1000.0;

    loop {
        TimeoutFuture::new(POLL_INTERVAL_MS).await;

        if js_sys::Date::now() >= deadline {
            log::warn!("pairing: TTL expired with no payload");
            state.set(PairState::Expired);
            return;
        }

        match pb.read_pairing_mailbox(cfg, &record.code).await {
            Ok(Some(fresh)) => {
                let Some(payload) = fresh.payload.as_deref().filter(|p| !p.is_empty()) else {
                    continue; // mailbox still empty — keep waiting
                };
                state.set(PairState::Decrypting);
                match unseal_session(&keys.secret, payload) {
                    Ok(paired) => {
                        log::info!("pairing: paired user={}", paired.user_id);
                        let token = paired.token.clone();
                        session.set(Some(paired));
                        state.set(PairState::Success);
                        // Best-effort cleanup, authed by the token we were just
                        // handed so the delete ACL passes. A leftover row is
                        // harmless (it holds only a spent pubkey + ciphertext).
                        if let Err(e) = pb.delete_pairing_mailbox(cfg, &fresh.id, &token).await {
                            log::warn!("pairing: mailbox cleanup failed: {e}");
                        }
                        return;
                    }
                    Err(e) => {
                        log::error!("pairing: unseal failed: {e}");
                        error.set(Some(format!("{}: {e}", strings.err_decrypt)));
                        state.set(PairState::Failed);
                        return;
                    }
                }
            }
            // The row is gone — an admin or the TTL cron removed it. Nothing
            // left to wait for.
            Ok(None) => {
                state.set(PairState::Expired);
                return;
            }
            Err(e) => log::warn!("pairing: mailbox poll error (retrying): {e}"),
        }
    }
}
