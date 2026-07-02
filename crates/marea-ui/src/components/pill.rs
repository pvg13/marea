//! [`Pill`] — inline status chip with tone modifiers.

use dioxus::prelude::*;

use super::merge_class;

/// Color tone of a [`Pill`]. `Neutral` is the bare `.pill`; the rest add a
/// `pill--*` modifier.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PillTone {
    #[default]
    Neutral,
    Primary,
    Accent,
    Success,
    Warning,
    Danger,
    Ghost,
}

impl PillTone {
    fn class(self) -> &'static str {
        match self {
            PillTone::Neutral => "",
            PillTone::Primary => " pill--primary",
            PillTone::Accent => " pill--accent",
            PillTone::Success => " pill--success",
            PillTone::Warning => " pill--warning",
            PillTone::Danger => " pill--danger",
            PillTone::Ghost => " pill--ghost",
        }
    }
}

/// A `span.pill` badge.
#[component]
pub fn Pill(
    #[props(default)] tone: PillTone,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let cls = merge_class(format!("pill{}", tone.class()), class.as_ref());
    rsx! {
        span { class: "{cls}", {children} }
    }
}
