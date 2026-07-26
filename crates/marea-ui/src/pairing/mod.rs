//! Phone→web QR pairing screens.
//!
//! The web tab has no credentials; the phone does. Rather than make the user
//! type a password into a shared computer, the phone hands the session over
//! through a sealed-box PocketBase mailbox — the crypto and the mailbox CRUD
//! live in [`marea_auth::pairing`]; this module is the UI over them.
//!
//! Two entry points, one per side of the handoff:
//!
//! - [`PairScreen`] (web/wasm): generates the ephemeral keypair, creates the
//!   mailbox, renders the QR and polls until the phone fills it. Pass it to
//!   [`AppShell`](crate::shell::AppShell)'s `login` slot.
//! - [`PairDeviceSection`] (native): scans a QR, shows what's about to be
//!   shared, and on confirmation seals the current session into the mailbox.
//!   Mount it in the app's settings screen.
//!
//! ```rust,ignore
//! static PAIRING: PairingConfig = PairingConfig {
//!     url_scheme: "myapp",
//!     mailbox_collection: "pairing_mailboxes",
//!     ttl_secs: 300,
//! };
//!
//! // Web login:
//! AppShell {
//!     auth: &AUTH,
//!     login: rsx! { PairScreen { config: &PAIRING, strings: my_strings() } },
//!     Router::<Route> {}
//! }
//!
//! // Phone settings screen:
//! PairDeviceSection { config: &PAIRING, strings: my_strings() }
//! ```
//!
//! Both sides take the same [`PairingStrings`], so an app builds it once from
//! its own translation table. The default is English.

mod qr;
pub use qr::{QrTile, qr_svg};

#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
pub use web::PairScreen;

mod view;
// The generated props struct is public so callers (and tests) can build props
// programmatically rather than only through rsx.
pub use view::{PairScreenView, PairScreenViewProps, PairState};

mod device;
pub use device::PairDeviceSection;

/// Every user-visible string in the pairing screens.
///
/// One struct rather than ~30 individual props: the screens carry a lot of
/// copy, and a struct keeps call sites to `strings: t.pairing()` while letting
/// the compiler catch a missing field when this grows. Translation tables stay
/// in apps (same rule as [`crate::i18n`]) — [`Default`] is English so a new app
/// renders sensibly before it has any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingStrings {
    // ── Web screen ──────────────────────────────────────────────────────
    /// First line of the headline, above [`Self::title_line_2`].
    pub title_line_1: &'static str,
    pub title_line_2: &'static str,
    /// One-paragraph explanation under the headline.
    pub lede: &'static str,
    pub step_1: &'static str,
    pub step_2: &'static str,
    pub step_3: &'static str,
    /// Reassurance line next to the shield mark.
    pub shield_note: &'static str,
    /// Label prefixing the pairing code under the QR.
    pub code_label: &'static str,
    /// Shown in place of the QR when the payload can't be encoded.
    pub qr_failed: &'static str,
    /// Switches to the email/password fallback.
    pub use_email: &'static str,
    /// Switches back from the fallback to the QR.
    pub use_qr: &'static str,

    // ── Web status line ─────────────────────────────────────────────────
    pub status_creating: &'static str,
    pub status_waiting: &'static str,
    pub status_waiting_hint: &'static str,
    pub status_decrypting: &'static str,
    pub status_success: &'static str,
    pub status_expired: &'static str,
    pub status_expired_hint: &'static str,
    pub status_failed: &'static str,

    // ── Phone side ──────────────────────────────────────────────────────
    pub device_section_title: &'static str,
    pub device_section_hint: &'static str,
    pub open_scanner: &'static str,
    /// Hint pill inside the scan overlay.
    pub point_qr: &'static str,
    pub confirm_title: &'static str,
    pub confirm_body: &'static str,
    pub confirm_button: &'static str,
    pub sending: &'static str,
    pub done: &'static str,
    pub cancel: &'static str,
    pub close: &'static str,

    // ── Errors ──────────────────────────────────────────────────────────
    // Each is shown with the underlying error appended, so they read as a
    // prefix ("Couldn't create the pairing mailbox: <cause>").
    pub err_mailbox_create: &'static str,
    pub err_decrypt: &'static str,
    pub err_mailbox_lookup: &'static str,
    pub err_code_expired: &'static str,
    pub err_pubkey_mismatch: &'static str,
    pub err_encrypt: &'static str,
    pub err_upload: &'static str,
    pub err_invalid_qr: &'static str,
    pub err_no_session: &'static str,
}

impl Default for PairingStrings {
    fn default() -> Self {
        Self {
            title_line_1: "Use this app",
            title_line_2: "on your computer",
            lede: "Scan the code with your phone to sign in here. Your password never leaves your phone.",
            step_1: "Open the app on your phone",
            step_2: "Go to Settings → Pair a device",
            step_3: "Point the camera at this code",
            shield_note: "End-to-end encrypted. The server never sees your session.",
            code_label: "code",
            qr_failed: "Couldn't render the code",
            use_email: "Sign in with email instead",
            use_qr: "Back to the QR code",

            status_creating: "Preparing…",
            status_waiting: "Waiting for your phone",
            status_waiting_hint: "The code stays valid for a few minutes.",
            status_decrypting: "Signing you in…",
            status_success: "Paired",
            status_expired: "This code expired",
            status_expired_hint: "Reload the page to get a fresh one.",
            status_failed: "Pairing failed",

            device_section_title: "Pair a device",
            device_section_hint:
                "Scan the code shown on the computer to sign in there without typing your password.",
            open_scanner: "Scan code",
            point_qr: "Point the camera at the code on your screen",
            confirm_title: "Sign in on that device?",
            confirm_body:
                "Your session will be encrypted and sent to the device showing this code. Only that device can read it.",
            confirm_button: "Sign in there",
            sending: "Sending to",
            done: "Signed in on the other device",
            cancel: "Cancel",
            close: "Close",

            err_mailbox_create: "Couldn't start pairing",
            err_decrypt: "Pairing failed while decrypting",
            err_mailbox_lookup: "Couldn't look up that code",
            err_code_expired: "That pairing code has expired",
            err_pubkey_mismatch: "This code doesn't match the waiting device — refresh it and scan again",
            err_encrypt: "Couldn't encrypt the session",
            err_upload: "Couldn't send the session",
            err_invalid_qr: "That isn't a pairing code for this app",
            err_no_session: "You need to be signed in on this device first",
        }
    }
}
