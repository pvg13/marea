//! Buttons: [`Button`] (variant + size/layout modifiers, full attribute
//! passthrough) and [`IconButton`] (ghost icon-only button).

use dioxus::prelude::*;

use super::merge_class;

/// Visual variant of a [`Button`]. Each maps to a self-sufficient `btn-*`
/// class in `marea.css`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
    Soft,
    Danger,
    Outline,
}

impl ButtonVariant {
    fn class(self) -> &'static str {
        match self {
            ButtonVariant::Primary => "btn-primary",
            ButtonVariant::Secondary => "btn-secondary",
            ButtonVariant::Ghost => "btn-ghost",
            ButtonVariant::Soft => "btn-soft",
            ButtonVariant::Danger => "btn-danger",
            ButtonVariant::Outline => "btn-outline",
        }
    }
}

/// A `<button>` styled by its [`ButtonVariant`]. `small` adds `btn-sm`,
/// `block` adds `btn-block`. All global and `<button>` attributes pass
/// through (`r#type`, `id`, `aria_*`, …).
#[component]
pub fn Button(
    #[props(default)] variant: ButtonVariant,
    #[props(default)] small: bool,
    #[props(default)] block: bool,
    #[props(default)] disabled: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] class: Option<String>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let mut cls = String::from(variant.class());
    if small {
        cls.push_str(" btn-sm");
    }
    if block {
        cls.push_str(" btn-block");
    }
    let cls = merge_class(cls, class.as_ref());

    rsx! {
        button {
            class: "{cls}",
            disabled: if disabled { true },
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e);
                }
            },
            ..attributes,
            {children}
        }
    }
}

/// Icon-only ghost button (`btn-ghost btn-icon`). The accessible name is
/// mandatory since the icon child carries no text.
#[component]
pub fn IconButton(
    aria_label: String,
    #[props(default)] disabled: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let cls = merge_class("btn-ghost btn-icon".to_string(), class.as_ref());
    rsx! {
        button {
            class: "{cls}",
            r#type: "button",
            aria_label: "{aria_label}",
            disabled: if disabled { true },
            onclick: move |e| {
                if let Some(h) = &onclick {
                    h.call(e);
                }
            },
            {children}
        }
    }
}
