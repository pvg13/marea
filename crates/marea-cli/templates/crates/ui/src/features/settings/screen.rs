//! Settings screen.
//!
//! Note there is no `ScreenHeader` here: `TopBar` already names the screen,
//! and rendering the title twice is the most common way a generated app looks
//! unfinished. `ScreenHeader` earns its place when it says something the top
//! bar does not.

use dioxus::prelude::*;
use marea_ui::{Button, ButtonVariant, Card, CardPad, Row, SectionHeader, TopBar, use_locale};
{%- if auth %}
use marea_ui::InfoRow;
{%- endif %}

{% if auth %}use crate::components::SignOut;
{% endif %}use crate::components::ThemeToggle;
use crate::i18n::{Lang, use_lang};

use super::i18n::strings;

#[component]
pub fn Settings() -> Element {
    let lang = use_lang();
    let t = strings(lang);

    rsx! {
        div { class: "page",
            TopBar { title: "{t.title}" }
            div { class: "page__body",

                SectionHeader { title: "{t.appearance}" }
                Card { pad: CardPad::Md,
                    // A row, not a bare button: a setting needs a label saying
                    // what it controls, with the control on the trailing edge.
                    Row {
                        title: "{t.theme}",
                        trailing: rsx! { ThemeToggle {} },
                    }
                }

                SectionHeader { title: "{t.language}" }
                Card { pad: CardPad::Md, LanguagePicker {} }
                {%- if auth %}

                SectionHeader { title: "{t.account}" }
                Card { pad: CardPad::Md, AccountRows {} }
                Card { pad: CardPad::Md, SignOut { block: true } }
                {%- endif %}
                {%- if pairing %}

                SectionHeader { title: "{t.devices}" }
                // Phone side of the QR handoff: scans the code the browser
                // shows and seals this session to it. Cross-target — it
                // renders as an inert placeholder anywhere without a camera,
                // so no cfg is needed here.
                marea_ui::pairing::PairDeviceSection {
                    config: &crate::app::PAIRING,
                    strings: crate::i18n::pairing_strings(lang),
                }
                {%- endif %}
            }
        }
    }
}

/// Language switcher over the app's own `Lang` enum.
///
/// Laid out as a horizontal group, and labelled with each language's own name
/// — `{lang:?}` would render the Rust variant ("Es"), which is a developer
/// detail rather than a word anyone is looking for.
#[component]
fn LanguagePicker() -> Element {
    let mut locale = use_locale::<Lang>();
    let current = locale.get();

    rsx! {
        div { class: "lang-picker",
            for lang in Lang::ALL.iter().copied() {
                Button {
                    key: "{lang.label()}",
                    variant: if lang == current { ButtonVariant::Primary } else { ButtonVariant::Ghost },
                    small: true,
                    onclick: move |_| locale.set(lang),
                    "{lang.label()}"
                }
            }
        }
    }
}
{% if auth %}
/// Read-only account facts, straight off the session marea persists.
#[component]
fn AccountRows() -> Element {
    let auth = marea_auth::use_auth();
    let t = strings(use_lang());
    let session = (auth.session)();

    rsx! {
        if let Some(s) = session {
            InfoRow { label: "{t.email}", value: "{s.email}" }
            InfoRow { label: "{t.user_id}", value: "{s.user_id}" }
        }
    }
}
{% endif %}
