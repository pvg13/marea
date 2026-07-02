//! [`Card`] — the surface container, with padding / interactive / inset
//! modifiers.

use dioxus::prelude::*;

use super::merge_class;

/// Padding preset for a [`Card`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CardPad {
    /// No padding class — the caller lays out the interior.
    None,
    /// `card-pad` (16px).
    #[default]
    Md,
    /// `card-pad-lg` (24px).
    Lg,
}

impl CardPad {
    fn class(self) -> &'static str {
        match self {
            CardPad::None => "",
            CardPad::Md => " card-pad",
            CardPad::Lg => " card-pad-lg",
        }
    }
}

/// A `div.card` surface. `interactive` adds the hover/cursor treatment,
/// `inset` the flat recessed look.
#[component]
pub fn Card(
    #[props(default)] pad: CardPad,
    #[props(default)] interactive: bool,
    #[props(default)] inset: bool,
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(default)] class: Option<String>,
    children: Element,
) -> Element {
    let mut cls = String::from("card");
    cls.push_str(pad.class());
    if interactive {
        cls.push_str(" card--interactive");
    }
    if inset {
        cls.push_str(" card--inset");
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
            {children}
        }
    }
}
