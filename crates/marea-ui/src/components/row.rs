//! List rows: [`Row`] (icon / title / subtitle / trailing) and [`InfoRow`]
//! (label / value pair).

use dioxus::prelude::*;

use super::merge_class;

/// A list row (`div.m-row`). Becomes `m-row--interactive` when an `onclick`
/// handler is supplied.
#[component]
pub fn Row(
    title: String,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] icon: Option<Element>,
    #[props(default)] trailing: Option<Element>,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let mut cls = String::from("m-row");
    if onclick.is_some() {
        cls.push_str(" m-row--interactive");
    }
    let cls = merge_class(cls, class.as_ref());

    rsx! {
        div {
            class: "{cls}",
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e);
                }
            },
            if let Some(ic) = icon {
                div { class: "m-row__icon", {ic} }
            }
            div { class: "m-row__body",
                div { class: "m-row__title", "{title}" }
                if let Some(s) = &subtitle {
                    div { class: "m-row__sub", "{s}" }
                }
            }
            if let Some(t) = trailing {
                div { class: "m-row__trailing", {t} }
            }
        }
    }
}

/// A label/value line (`div.info-row`) for detail screens.
#[component]
pub fn InfoRow(label: String, value: String, #[props(default)] class: Option<String>) -> Element {
    let cls = merge_class("info-row".to_string(), class.as_ref());
    rsx! {
        div { class: "{cls}",
            span { class: "info-row__label", "{label}" }
            span { class: "info-row__value", "{value}" }
        }
    }
}
