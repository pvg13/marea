//! QR rendering for the pairing tile.
//!
//! Generation, not scanning — [`crate::scanner`] handles the camera. It lives
//! under `pairing` because that's the feature that should pull the `qrcode`
//! dependency; apps needing a QR for something else (a group invite, say) can
//! still call [`qr_svg`] from here.

use dioxus::prelude::*;

use super::PairState;

/// Render a payload as an SVG string, or `None` when it exceeds QR capacity.
///
/// Error-correction level **H** (~30% of the symbol can be damaged and still
/// decode) because [`QrTile`] floats a badge over the code's centre — at the
/// default level that occlusion is enough to make it unreadable.
///
/// The generated `<svg>` carries hard-coded `width`/`height` attributes, so the
/// caller must scale it from a wrapping element; `.qr-tile__code svg` in
/// `marea.css` does exactly that.
pub fn qr_svg(payload: &str) -> Option<String> {
    use qrcode::{EcLevel, QrCode, render::svg};
    QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::H)
        .ok()
        .map(|code| {
            code.render::<svg::Color>()
                .min_dimensions(280, 280)
                .quiet_zone(true)
                .build()
        })
}

/// The white QR card, with an optional brand badge floating in its centre.
///
/// Stays white in **both** themes: it is scanned by a camera, and contrast for
/// the decoder beats visual consistency with the surrounding page. Apps that
/// really want to restyle it can set `--c-qr-bg`.
#[component]
pub fn QrTile(
    /// The URL to encode — normally [`marea_auth::pair_url`]'s output.
    payload: String,
    /// Dims/greys the tile once the code is spent or dead.
    #[props(default = PairState::Waiting)]
    state: PairState,
    /// Badge drawn over the code's quiet centre (level-H correction covers it).
    #[props(default)]
    logo: Option<Element>,
    /// Shown instead of the code when encoding fails.
    #[props(default = String::from("Couldn't render the code"))]
    failed_label: String,
) -> Element {
    let svg = qr_svg(&payload);

    let class = match state {
        PairState::Success => "qr-tile qr-tile--spent",
        PairState::Failed | PairState::Expired => "qr-tile qr-tile--dead",
        _ => "qr-tile",
    };

    rsx! {
        div { class: "{class}",
            if let Some(svg) = svg {
                div { class: "qr-tile__code", dangerous_inner_html: "{svg}" }
                if let Some(logo) = logo {
                    div { class: "qr-tile__logo", {logo} }
                }
            } else {
                div { class: "qr-tile__failed", "{failed_label}" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_a_realistic_pair_url() {
        let svg = qr_svg("myapp://pair?code=ABCD2345&pubkey=aGVsbG8%2Bd29ybGQ%2F").unwrap();
        // The renderer prepends an XML prolog before the root element; both go
        // into `dangerous_inner_html` verbatim.
        assert!(svg.contains("<svg"), "{svg}");
        assert!(svg.contains("</svg>"), "{svg}");
    }

    /// Level-H correction spends ~30% of the symbol on redundancy, so capacity
    /// is well under the format's headline maximum. Returning `None` rather
    /// than panicking is what lets [`QrTile`] fall back to a message.
    #[test]
    fn oversized_payload_is_none_not_a_panic() {
        assert!(qr_svg(&"x".repeat(4000)).is_none());
    }

    #[test]
    fn distinct_payloads_produce_distinct_codes() {
        let a = qr_svg("myapp://pair?code=AAAA1111&pubkey=k").unwrap();
        let b = qr_svg("myapp://pair?code=BBBB2222&pubkey=k").unwrap();
        assert_ne!(a, b);
    }
}
