//! Headers: [`TopBar`], [`ScreenHeader`] and [`SectionHeader`].

use dioxus::prelude::*;

use super::merge_class;

/// Sticky app bar (`header.top-bar`) with a title and an optional trailing
/// actions slot.
#[component]
pub fn TopBar(
    title: String,
    #[props(default)] actions: Option<Element>,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class("top-bar".to_string(), class.as_ref());
    rsx! {
        header { class: "{cls}",
            div { class: "top-bar__title", "{title}" }
            if let Some(a) = actions {
                div { class: "top-bar__actions", {a} }
            }
        }
    }
}

/// Page heading block (`div.screen-header`): optional eyebrow, `h1` title,
/// optional subtitle, and an optional `right` slot rendered as a sibling of
/// the text body.
#[component]
pub fn ScreenHeader(
    title: String,
    #[props(default)] eyebrow: Option<String>,
    #[props(default)] subtitle: Option<String>,
    #[props(default)] right: Option<Element>,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class("screen-header".to_string(), class.as_ref());
    rsx! {
        div { class: "{cls}",
            div { class: "screen-header__body",
                if let Some(e) = &eyebrow {
                    span { class: "eyebrow", "{e}" }
                }
                h1 { class: "screen-header__title", "{title}" }
                if let Some(s) = &subtitle {
                    div { class: "screen-header__subtitle", "{s}" }
                }
            }
            {right}
        }
    }
}

/// In-page section heading (`div.section-header`) with an optional trailing
/// text action button.
#[component]
pub fn SectionHeader(
    title: String,
    #[props(default)] action_label: Option<String>,
    #[props(default)] on_action: Option<EventHandler<MouseEvent>>,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class("section-header".to_string(), class.as_ref());
    rsx! {
        div { class: "{cls}",
            h2 { class: "section-header__title", "{title}" }
            if let Some(label) = &action_label {
                button {
                    class: "section-header__action",
                    r#type: "button",
                    onclick: move |e| {
                        if let Some(h) = &on_action {
                            h.call(e);
                        }
                    },
                    "{label}"
                }
            }
        }
    }
}
