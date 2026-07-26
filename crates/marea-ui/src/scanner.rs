//! Camera code scanner — live feed plus continuous detection.
//!
//! Two components over one JS pipeline:
//!
//! - [`InlineScanner`]: the bare `<video>` feed, filling its parent, firing
//!   `on_detect` for every code it sees. No chrome — the caller supplies the
//!   viewfinder. Use it when the scanner is embedded in an existing sheet.
//! - [`QrScanOverlay`]: a full-screen QR takeover (dark scrim, framed
//!   viewfinder, sweeping scan line, close button) built on `InlineScanner`.
//!   Use it for context flows — pairing a device, joining a group.
//!
//! Detection pipeline:
//! 1. JS calls `navigator.mediaDevices.getUserMedia` for the rear camera.
//! 2. [`BarcodeDetector`] polls at ~5 Hz. On each hit it sends the code to
//!    Rust, then enters a 3-second cooldown **for that same value** so a code
//!    held in frame doesn't fire repeatedly; a different code is sent
//!    immediately. The stream keeps running — no stop/restart cycle.
//! 3. Rust receives codes over `eval.recv` and fires `on_detect`.
//!
//! The camera is released on unmount ([`InlineScanner`]'s `use_drop`) and by
//! the `__mareaScannerStop` global the JS installs.
//!
//! ## iOS
//!
//! WebKit has no `BarcodeDetector`. On iOS **only**, a vendored ZXing-JS build
//! (`src/vendor/zxing.min.js`, Apache-2.0 — see `src/vendor/README.md`) is
//! prepended to the eval and [`POLYFILL_INSTALL`] wraps it in an object with
//! the same `detect(video) -> [{ rawValue }]` shape, so the loop below runs
//! unchanged. `getUserMedia` itself works in WKWebView from iOS 14.3 given an
//! `NSCameraUsageDescription`. Every other target compiles the bundle out.
//!
//! [`BarcodeDetector`]: https://developer.mozilla.org/en-US/docs/Web/API/BarcodeDetector

use dioxus::prelude::*;
use serde::Deserialize;

/// QR only — pairing, invites, anything where a stray EAN would be noise.
pub const QR_FORMATS: &[&str] = &["qr_code"];

/// Retail product codes. Includes `qr_code` deliberately: many retailers
/// (Spanish supermarkets in particular) print QR rather than EAN on own-brand
/// packaging, and a product scanner that ignored them would look broken.
pub const BARCODE_FORMATS: &[&str] = &["ean_13", "ean_8", "upc_a", "upc_e", "qr_code"];

/// Continuous-scan driver. The `__FORMATS__` placeholder is substituted by
/// [`scanner_js_for`].
///
/// Notes that are load-bearing rather than stylistic:
/// - the stream is never stopped by this script (only by `__mareaScannerStop`
///   or the Rust-side `use_drop`), so a detection doesn't tear down the camera;
/// - the cooldown is keyed on the code's *value*, not on time alone, so
///   scanning a second item is instant while a lingering one stays quiet.
const SCANNER_JS: &str = r#"
const send = (kind, value) => {
    try { dioxus.send({ kind, value: value || "" }); } catch (e) {}
};

if (!('BarcodeDetector' in window)) {
    send('error', 'BarcodeDetector unsupported on this device');
    return;
}

let stream = null;
let stopped = false;
let lastCode = '';
let lastCodeTime = 0;
const COOLDOWN_MS = 3000;

window.__mareaScannerStop = () => {
    stopped = true;
    if (stream) {
        stream.getTracks().forEach(t => t.stop());
        stream = null;
    }
};

let video;
try {
    video = document.getElementById('marea-scanner-video');
    if (!video) {
        send('error', 'Camera element not found');
        return;
    }
    stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: 'environment' }
    });
    if (stopped) {
        stream.getTracks().forEach(t => t.stop());
        return;
    }
    video.srcObject = stream;
    await video.play();
    send('started', '');
} catch (e) {
    send('error', String((e && e.message) || e));
    return;
}

const detector = new BarcodeDetector({ formats: __FORMATS__ });

while (!stopped) {
    try {
        const codes = await detector.detect(video);
        if (codes && codes.length > 0) {
            const value = codes[0].rawValue;
            const now = Date.now();
            if (value !== lastCode || (now - lastCodeTime) > COOLDOWN_MS) {
                lastCode = value;
                lastCodeTime = now;
                send('detected', value);
            }
        }
    } catch (e) {}
    await new Promise(r => setTimeout(r, 200));
}
"#;

/// One message from [`SCANNER_JS`]: `started`, `detected` or `error`.
#[derive(Debug, Deserialize)]
struct ScannerMsg {
    kind: String,
    #[serde(default)]
    value: String,
}

// ── iOS BarcodeDetector polyfill ─────────────────────────────────────────────

#[cfg(target_os = "ios")]
const ZXING_JS: &str = include_str!("vendor/zxing.min.js");

#[cfg(target_os = "ios")]
const POLYFILL_INSTALL: &str = r#"
(function(){
  if ('BarcodeDetector' in window) return;      // native (won't happen on WebKit)
  var Z = window.ZXing;
  if (!Z) return;                                // lib missing → SCANNER_JS guard errors out
  var FMT = { ean_13:'EAN_13', ean_8:'EAN_8', upc_a:'UPC_A', upc_e:'UPC_E',
              qr_code:'QR_CODE', code_128:'CODE_128', code_39:'CODE_39',
              codabar:'CODABAR', itf:'ITF', code_93:'CODE_93' };
  var BD = function(opts){
    var hints = new Map();
    var fmts = (((opts && opts.formats) || [])
      .map(function(f){ return Z.BarcodeFormat[FMT[f] || String(f).toUpperCase()]; })
      .filter(function(v){ return v !== undefined; }));
    if (fmts.length) hints.set(Z.DecodeHintType.POSSIBLE_FORMATS, fmts);
    this._reader = new Z.MultiFormatReader();
    this._reader.setHints(hints);
    this._canvas = document.createElement('canvas');
  };
  BD.prototype.detect = function(video){
    var w = video.videoWidth, h = video.videoHeight;
    if (!w || !h) return Promise.resolve([]);
    this._canvas.width = w; this._canvas.height = h;
    this._canvas.getContext('2d').drawImage(video, 0, 0, w, h);
    try {
      var src = new Z.HTMLCanvasElementLuminanceSource(this._canvas);
      var bitmap = new Z.BinaryBitmap(new Z.HybridBinarizer(src));
      var result = this._reader.decode(bitmap);
      return Promise.resolve(result ? [{ rawValue: result.getText() }] : []);
    } catch (e) {
      return Promise.resolve([]);                // NotFoundException: no code this frame
    } finally {
      this._reader.reset();
    }
  };
  window.BarcodeDetector = BD;
})();
"#;

/// JS prepended to the scanner eval to make `BarcodeDetector` available on
/// WebKit. Empty on every other target, where the API is native.
#[cfg(target_os = "ios")]
fn polyfill_prelude() -> String {
    format!("{ZXING_JS}\n{POLYFILL_INSTALL}\n")
}

#[cfg(not(target_os = "ios"))]
fn polyfill_prelude() -> &'static str {
    ""
}

/// Build the eval source for a given format list.
fn scanner_js_for(formats: &[&str]) -> String {
    let json = format!(
        "[{}]",
        formats
            .iter()
            .map(|f| format!("'{f}'"))
            .collect::<Vec<_>>()
            .join(",")
    );
    format!(
        "{}{}",
        polyfill_prelude(),
        SCANNER_JS.replace("__FORMATS__", &json)
    )
}

/// Bare camera feed: just the live `<video>`, filling its parent, running the
/// detection loop and firing `on_detect` per code. Stops the camera on unmount.
///
/// Only one may be mounted at a time — they'd contend for the same element id
/// and the same `__mareaScannerStop` global.
#[component]
pub fn InlineScanner(
    on_detect: EventHandler<String>,
    /// Code formats to look for. Defaults to [`BARCODE_FORMATS`]; pass
    /// [`QR_FORMATS`] for QR-only flows.
    #[props(default = BARCODE_FORMATS.iter().map(|s| s.to_string()).collect())]
    formats: Vec<String>,
) -> Element {
    let formats_for_js = formats.clone();
    use_effect(move || {
        let formats: Vec<&str> = formats_for_js.iter().map(|s| s.as_str()).collect();
        let mut eval = document::eval(&scanner_js_for(&formats));
        spawn(async move {
            while let Ok(msg) = eval.recv::<ScannerMsg>().await {
                match msg.kind.as_str() {
                    "detected" if !msg.value.is_empty() => on_detect.call(msg.value),
                    "error" => {
                        log::warn!("marea scanner: {}", msg.value);
                        return;
                    }
                    _ => {}
                }
            }
        });
    });
    use_drop(|| {
        document::eval("if (window.__mareaScannerStop) { window.__mareaScannerStop(); }");
    });
    rsx! {
        video {
            id: "marea-scanner-video",
            class: "scan-video",
            autoplay: "true",
            playsinline: "true",
            muted: "true",
        }
    }
}

/// Full-screen QR scan takeover: dark scrim over the live feed, a square
/// viewfinder with a sweeping scan line, a close button and a hint pill.
///
/// The caller supplies the context (`title` for the bar, `hint` for the pill)
/// and handles the scanned string — this component neither interprets nor
/// validates it.
#[component]
pub fn QrScanOverlay(
    title: String,
    hint: String,
    on_close: EventHandler<()>,
    on_detect: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "scan-overlay",
            div { class: "scan-overlay__feed",
                InlineScanner {
                    on_detect: move |code: String| on_detect.call(code),
                    formats: QR_FORMATS.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
                }
            }
            // Diagonal grain over the feed — softens the camera image so the
            // chrome above it stays legible against any scene.
            div { class: "scan-overlay__grain" }

            div { class: "scan-bar",
                button {
                    r#type: "button",
                    class: "scan-bar__btn",
                    aria_label: "{title}",
                    onclick: move |_| on_close.call(()),
                    CloseIcon {}
                }
                span { class: "scan-bar__title", "{title}" }
                // Balances the close button so the title stays optically
                // centered without a grid.
                span { class: "scan-bar__btn scan-bar__btn--spacer", aria_hidden: "true" }
            }

            div { class: "scan-frame",
                div { class: "scan-frame__box",
                    div { class: "scan-line" }
                }
                span { class: "scan-hint", "{hint}" }
            }
        }
    }
}

#[component]
fn CloseIcon() -> Element {
    rsx! {
        svg {
            width: "20",
            height: "20",
            view_box: "0 0 24 24",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "2",
            stroke_linecap: "round",
            path { d: "M18 6L6 18M6 6l12 12" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_js_substitutes_the_format_list() {
        let js = scanner_js_for(QR_FORMATS);
        assert!(js.contains("new BarcodeDetector({ formats: ['qr_code'] })"), "{js}");
        assert!(!js.contains("__FORMATS__"));
    }

    #[test]
    fn scanner_js_joins_multiple_formats() {
        let js = scanner_js_for(BARCODE_FORMATS);
        assert!(
            js.contains("['ean_13','ean_8','upc_a','upc_e','qr_code']"),
            "{js}"
        );
    }

    /// The element id and the stop global are a contract between the JS and
    /// `InlineScanner`'s rsx/`use_drop`. They were renamed from Mediterranea's
    /// app-branded names during the port; a partial rename would leave the
    /// camera running after unmount.
    #[test]
    fn js_uses_the_debranded_names() {
        let js = scanner_js_for(QR_FORMATS);
        assert!(js.contains("marea-scanner-video"), "{js}");
        assert!(js.contains("__mareaScannerStop"), "{js}");
        assert!(!js.contains("mediterranea"), "{js}");
    }
}
