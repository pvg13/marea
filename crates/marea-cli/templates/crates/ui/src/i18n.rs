//! This app's language type.
//!
//! marea supplies the *mechanism* — persistence and context — and each app
//! supplies its own enum and string tables. The tables themselves live with
//! the features that use them (`features/<name>/i18n.rs`), never in one
//! central file.

use marea_ui::Locale;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Lang {
{%- for loc in locales %}
    {% if loc.default %}#[default]
    {% endif %}{{ loc.variant }},
{%- endfor %}
}

impl Locale for Lang {
    fn as_tag(&self) -> &'static str {
        match self {
{%- for loc in locales %}
            Lang::{{ loc.variant }} => "{{ loc.tag }}",
{%- endfor %}
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match tag {
{%- for loc in locales %}
            "{{ loc.tag }}" => Some(Lang::{{ loc.variant }}),
{%- endfor %}
            _ => None,
        }
    }
}

/// Storage key for the chosen language. Safe to change — unlike the auth and
/// database keys, losing this only resets the language to the default.
pub const LANG_STORAGE_KEY: &str = "{{ snake }}_lang";

impl Lang {
    /// The language's own name, for a language picker.
    ///
    /// Endonyms, not English names: someone looking for Spanish is looking
    /// for "Español". Never render `{:?}` here — the Rust variant name is a
    /// developer detail, and "Es" is not a word.
    pub fn label(self) -> &'static str {
        match self {
{%- for loc in locales %}
{%- if loc.tag == "es" %}
            Lang::{{ loc.variant }} => "Español",
{%- elif loc.tag == "en" %}
            Lang::{{ loc.variant }} => "English",
{%- else %}
            // TODO({{ loc.tag }}): replace with the language's own name.
            Lang::{{ loc.variant }} => "{{ loc.tag }}",
{%- endif %}
{%- endfor %}
        }
    }

    /// Every language this app ships, in menu order.
    pub const ALL: &'static [Lang] = &[
{%- for loc in locales %}
        Lang::{{ loc.variant }},
{%- endfor %}
    ];
}

{% if auth %}/// Used by both the nav footer and the settings card, so it lives here
/// rather than in either feature.
pub fn sign_out(lang: Lang) -> &'static str {
    match lang {
{%- for loc in locales %}
{%- if loc.tag == "es" %}
        Lang::{{ loc.variant }} => "Cerrar sesión",
{%- else %}
        Lang::{{ loc.variant }} => "Sign out",
{%- endif %}
{%- endfor %}
    }
}

{% endif %}/// Label for the theme toggle, which names the mode it switches *to*.
pub fn theme_toggle(lang: Lang, is_dark: bool) -> &'static str {
    match (lang, is_dark) {
{%- for loc in locales %}
{%- if loc.tag == "es" %}
        (Lang::{{ loc.variant }}, true) => "Modo claro",
        (Lang::{{ loc.variant }}, false) => "Modo oscuro",
{%- else %}
        (Lang::{{ loc.variant }}, true) => "Light mode",
        (Lang::{{ loc.variant }}, false) => "Dark mode",
{%- endif %}
{%- endfor %}
    }
}

/// The active language. Call from any component.
pub fn use_lang() -> Lang {
    marea_ui::use_locale::<Lang>().get()
}
{% if pairing %}
/// Copy for marea's pairing screens.
///
/// Shared by two features — the phone's "pair a device" section and the
/// browser's login screen — so by this crate's own rule it lives here rather
/// than in either one.
///
/// `..Default::default()` is doing real work: `PairingStrings` has forty-odd
/// fields covering error states most apps never surface, and spreading the
/// English default means adding a language only requires translating the copy
/// that is actually on screen.
pub fn pairing_strings(lang: Lang) -> marea_ui::pairing::PairingStrings {
    use marea_ui::pairing::PairingStrings;
    match lang {
{%- for loc in locales %}
{%- if loc.tag == "es" %}
        Lang::{{ loc.variant }} => PairingStrings {
            title_line_1: "Inicia sesión",
            title_line_2: "con el móvil",
            lede: "Escanea este código con la app para entrar sin escribir la contraseña.",
            step_1: "Abre la app en el móvil",
            step_2: "Ve a Ajustes › Vincular dispositivo",
            step_3: "Escanea el código",
            shield_note: "La sesión viaja cifrada de extremo a extremo.",
            code_label: "Código",
            use_email: "Usar correo y contraseña",
            use_qr: "Usar código QR",
            status_creating: "Preparando…",
            status_waiting: "Esperando al móvil…",
            status_waiting_hint: "El código sigue siendo válido unos minutos.",
            status_expired_hint: "Recarga la página para generar uno nuevo.",
            status_failed: "No se pudo completar el emparejamiento.",
            status_decrypting: "Descifrando…",
            status_success: "¡Listo!",
            status_expired: "El código ha caducado",
            device_section_title: "Vincular dispositivo",
            device_section_hint: "Escanea el código que aparece en el ordenador para entrar allí sin escribir la contraseña.",
            open_scanner: "Escanear código",
            point_qr: "Apunta al código QR",
            confirm_title: "¿Vincular este dispositivo?",
            confirm_button: "Vincular",
            sending: "Enviando…",
            done: "Hecho",
            cancel: "Cancelar",
            close: "Cerrar",
            err_mailbox_create: "No se pudo iniciar el emparejamiento.",
            err_mailbox_lookup: "No se pudo comprobar el código.",
            err_decrypt: "No se pudo descifrar la sesión.",
            err_code_expired: "El código ha caducado.",
            err_invalid_qr: "Ese código QR no es válido.",
            err_upload: "No se pudo enviar la sesión.",
            err_no_session: "Inicia sesión en este dispositivo primero.",
            ..Default::default()
        },
{%- else %}
        Lang::{{ loc.variant }} => PairingStrings::default(),
{%- endif %}
{%- endfor %}
    }
}
{% endif %}
