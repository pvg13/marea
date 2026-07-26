# Vendored third-party assets

## `zxing.min.js`

[ZXing-JS](https://github.com/zxing-js/library) (`@zxing/library`), the UMD
minified build, 336 008 bytes.

**License:** Apache License 2.0 — see
<https://github.com/zxing-js/library/blob/master/LICENSE>. Copyright the
ZXing-JS authors.

**Why it's here.** `src/scanner.rs` drives the camera through the WHATWG
[`BarcodeDetector`] API. Android's WebView is Chromium and ships it; iOS's
WebView is WebKit and does **not**. On iOS only, this bundle is `include_str!`'d
and prepended to the scanner eval, where a small shim
(`POLYFILL_INSTALL` in `scanner.rs`) wraps it in a `BarcodeDetector`-shaped
object exposing the same `detect(video) -> [{ rawValue }]` contract. Every other
target compiles it out entirely — it is not part of the desktop, Android or web
bundle.

Vendored rather than fetched at runtime because the artifact must work offline
and a CDN load would be a third-party request from inside a user's app.

[`BarcodeDetector`]: https://developer.mozilla.org/en-US/docs/Web/API/BarcodeDetector
