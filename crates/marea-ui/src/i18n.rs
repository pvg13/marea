//! Locale mechanism — persistence + context only. Translation tables stay in
//! apps (each defines its own `Locale` type and string lookup).
//!
//! ```rust,ignore
//! #[derive(Clone, Copy, PartialEq, Default)]
//! enum Lang { #[default] Es, En }
//!
//! impl marea_ui::Locale for Lang {
//!     fn as_tag(&self) -> &'static str { match self { Lang::Es => "es", Lang::En => "en" } }
//!     fn from_tag(tag: &str) -> Option<Self> {
//!         match tag { "es" => Some(Lang::Es), "en" => Some(Lang::En), _ => None }
//!     }
//! }
//!
//! // In the app root (above or below AppShell):
//! provide_locale::<Lang>("lang");
//! // Anywhere:
//! let mut lang = use_locale::<Lang>();
//! let t = translations(lang.get());
//! ```

use std::marker::PhantomData;

use dioxus::prelude::*;
use dioxus_sdk_storage::{LocalStorage, use_synced_storage};

/// An app's language type. `Default` is the fallback for unknown/missing
/// stored tags.
pub trait Locale: Copy + PartialEq + Default + 'static {
    /// Persisted identifier, e.g. `"es"` / `"en"`.
    fn as_tag(&self) -> &'static str;
    fn from_tag(tag: &str) -> Option<Self>;
}

/// `Copy` handle on the active locale, backed by persistent storage.
pub struct LocaleCtl<L: Locale> {
    stored: Signal<String>,
    _marker: PhantomData<L>,
}

impl<L: Locale> Clone for LocaleCtl<L> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<L: Locale> Copy for LocaleCtl<L> {}

impl<L: Locale> LocaleCtl<L> {
    /// The active locale (reactive — reads the underlying signal).
    pub fn get(&self) -> L {
        L::from_tag(&(self.stored)()).unwrap_or_default()
    }

    pub fn set(&mut self, locale: L) {
        self.stored.set(locale.as_tag().to_string());
    }
}

/// Provide the locale context, persisted under `storage_key`. Call once at
/// the app root.
pub fn provide_locale<L: Locale>(storage_key: &'static str) -> LocaleCtl<L> {
    let stored = use_synced_storage::<LocalStorage, String>(storage_key.to_string(), || {
        L::default().as_tag().to_string()
    });
    use_context_provider(|| LocaleCtl::<L> {
        stored,
        _marker: PhantomData,
    })
}

pub fn use_locale<L: Locale>() -> LocaleCtl<L> {
    use_context::<LocaleCtl<L>>()
}
