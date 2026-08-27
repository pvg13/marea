//! Strings for the `home` feature.
//!
//! Per-feature, not one central table. A single app-wide `i18n.rs` grows to
//! thousands of lines, every feature edits it, and dead strings become
//! impossible to spot. Here the owner of a string is obvious, and deleting
//! the feature deletes its strings.

use crate::i18n::Lang;

pub struct Strings {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub body: &'static str,
}

pub fn strings(lang: Lang) -> Strings {
    match lang {
{%- for loc in locales %}
{%- if loc.tag == "es" %}
        Lang::{{ loc.variant }} => Strings {
            title: "Funciona",
            subtitle: "Ahora construye la parte que solo esta app puede ser.",
            body: "Esto es un slice de funcionalidad: pantalla, componentes, estado y textos juntos. Copia su forma.",
        },
{%- elif loc.translated %}
        Lang::{{ loc.variant }} => Strings {
            title: "It works",
            subtitle: "Now build the part only this app can be.",
            body: "This is a feature slice: screen, components, state and strings together. Copy its shape.",
        },
{%- else %}
        // TODO({{ loc.tag }}): translate — falling back to English copy.
        Lang::{{ loc.variant }} => Strings {
            title: "It works",
            subtitle: "Now build the part only this app can be.",
            body: "This is a feature slice: screen, components, state and strings together. Copy its shape.",
        },
{%- endif %}
{%- endfor %}
    }
}
