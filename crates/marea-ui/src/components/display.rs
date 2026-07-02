//! Small typographic helpers: [`Eyebrow`] and [`Divider`].

use dioxus::prelude::*;

use super::merge_class;

/// Uppercase kicker text (`span.eyebrow`).
#[component]
pub fn Eyebrow(#[props(default)] class: Option<String>, children: Element) -> Element {
    let cls = merge_class("eyebrow".to_string(), class.as_ref());
    rsx! {
        span { class: "{cls}", {children} }
    }
}

/// Horizontal rule (`hr.divider`).
#[component]
pub fn Divider(#[props(default)] class: Option<String>) -> Element {
    let cls = merge_class("divider".to_string(), class.as_ref());
    rsx! {
        hr { class: "{cls}" }
    }
}
