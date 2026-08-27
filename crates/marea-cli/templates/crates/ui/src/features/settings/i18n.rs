//! Strings for the `settings` feature.

use crate::i18n::Lang;

pub struct Strings {
    pub title: &'static str,
    pub appearance: &'static str,
    pub theme: &'static str,
    pub language: &'static str,
{%- if auth %}
    pub account: &'static str,
    pub email: &'static str,
    pub user_id: &'static str,
{%- endif %}
{%- if pairing %}
    pub devices: &'static str,
{%- endif %}
}

pub fn strings(lang: Lang) -> Strings {
    match lang {
{%- for loc in locales %}
{%- if loc.tag == "es" %}
        Lang::{{ loc.variant }} => Strings {
            title: "Ajustes",
            appearance: "Apariencia",
            theme: "Tema",
            language: "Idioma",
{%- if auth %}
            account: "Cuenta",
            email: "Correo",
            user_id: "Usuario",
{%- endif %}
{%- if pairing %}
            devices: "Dispositivos",
{%- endif %}
        },
{%- else %}
{%- if not loc.translated %}
        // TODO({{ loc.tag }}): translate — falling back to English copy.
{%- endif %}
        Lang::{{ loc.variant }} => Strings {
            title: "Settings",
            appearance: "Appearance",
            theme: "Theme",
            language: "Language",
{%- if auth %}
            account: "Account",
            email: "Email",
            user_id: "User",
{%- endif %}
{%- if pairing %}
            devices: "Devices",
{%- endif %}
        },
{%- endif %}
{%- endfor %}
    }
}
