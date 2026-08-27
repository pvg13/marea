//! DELETE ME — sample slice showing the marea layering.
//!
//! Strings for the `items` feature.
//!
//! Note `remaining` is a function, not a field. Counts need agreement in most
//! languages, and "1 elementos" is the kind of detail that makes an app feel
//! machine-made — so the plural rule lives with the strings it applies to.

use crate::i18n::Lang;

pub struct Strings {
    pub title: &'static str,
    pub add: &'static str,
    pub add_label: &'static str,
    pub delete: &'static str,
    pub done: &'static str,
    pub empty_title: &'static str,
    pub empty_body: &'static str,
    remaining_one: &'static str,
    remaining_many: &'static str,
}

impl Strings {
    /// e.g. "1 pendiente" / "3 pendientes".
    pub fn remaining(&self, n: usize) -> String {
        let word = if n == 1 {
            self.remaining_one
        } else {
            self.remaining_many
        };
        format!("{n} {word}")
    }
}

pub fn strings(lang: Lang) -> Strings {
    match lang {
{%- for loc in locales %}
{%- if loc.tag == "es" %}
        Lang::{{ loc.variant }} => Strings {
            title: "Elementos",
            add: "Añadir",
            add_label: "Nuevo elemento",
            delete: "Borrar",
            done: "Hecho",
            empty_title: "Nada todavía",
            empty_body: "Añade uno arriba. Aparecerá en tus otros dispositivos.",
            remaining_one: "pendiente",
            remaining_many: "pendientes",
        },
{%- else %}
{%- if not loc.translated %}
        // TODO({{ loc.tag }}): translate — falling back to English copy.
{%- endif %}
        Lang::{{ loc.variant }} => Strings {
            title: "Items",
            add: "Add",
            add_label: "New item",
            delete: "Delete",
            done: "Done",
            empty_title: "Nothing yet",
            empty_body: "Add one above. It shows up on your other devices.",
            remaining_one: "remaining",
            remaining_many: "remaining",
        },
{%- endif %}
{%- endfor %}
    }
}
