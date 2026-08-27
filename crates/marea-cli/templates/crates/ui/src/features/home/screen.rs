//! The landing screen.

use dioxus::prelude::*;
use marea_ui::{Card, CardPad, ScreenHeader, TopBar};

use crate::i18n::use_lang;

use super::i18n::strings;

#[component]
pub fn Home() -> Element {
    let t = strings(use_lang());

    rsx! {
        div { class: "page",
            // The theme control lives in Settings; a second copy in the top
            // bar of one screen is clutter, not convenience.
            TopBar { title: "{{ title }}" }
            div { class: "page__body",
                ScreenHeader {
                    title: "{t.title}",
                    subtitle: "{t.subtitle}",
                }
                Card { pad: CardPad::Lg, "{t.body}" }
            }
        }
    }
}
