//! Loading / empty feedback: [`EmptyState`], [`LoadingSkeleton`] and
//! [`GatePending`].

use dioxus::prelude::*;

use super::merge_class;

/// Centered "nothing here" placeholder (`div.empty-state`) with optional
/// icon, description and action slots.
#[component]
pub fn EmptyState(
    title: String,
    #[props(default)] icon: Option<Element>,
    #[props(default)] description: Option<String>,
    #[props(default)] action: Option<Element>,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class("empty-state".to_string(), class.as_ref());
    rsx! {
        div { class: "{cls}",
            if let Some(ic) = icon {
                div { class: "empty-state__icon", {ic} }
            }
            div { class: "empty-state__title", "{title}" }
            if let Some(d) = &description {
                div { class: "empty-state__desc", "{d}" }
            }
            {action}
        }
    }
}

/// Pulsing placeholder lines (`div.skeleton`). Line widths vary so the
/// block reads as text: the last line is short, the rest alternate — the
/// one place inline `style` is allowed, since the widths are dynamic.
#[component]
pub fn LoadingSkeleton(
    #[props(default = 3)] lines: usize,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class("skeleton".to_string(), class.as_ref());
    rsx! {
        div { class: "{cls}",
            for i in 0..lines {
                div {
                    key: "{i}",
                    class: "skeleton__line",
                    style: if i + 1 == lines {
                        "width: 60%;"
                    } else if i % 2 == 1 {
                        "width: 88%;"
                    } else {
                        "width: 100%;"
                    },
                }
            }
        }
    }
}

/// Full-height pending state for auth/data gates (`div.gate-pending`).
/// Defaults to a [`LoadingSkeleton`] when no children are given.
#[component]
pub fn GatePending(
    #[props(default)] children: Option<Element>,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class("gate-pending".to_string(), class.as_ref());
    rsx! {
        div { class: "{cls}",
            if let Some(c) = children {
                {c}
            } else {
                LoadingSkeleton {}
            }
        }
    }
}
