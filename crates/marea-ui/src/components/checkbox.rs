//! [`Checkbox`] — a labelled checkbox (`label.checkbox`).

use dioxus::prelude::*;

use super::merge_class;

/// A checkbox with its text label; the whole label toggles the control.
#[component]
pub fn Checkbox(
    label: String,
    checked: bool,
    onchange: EventHandler<FormEvent>,
    #[props(default)] class: Option<String>,
) -> Element {
    let cls = merge_class("checkbox".to_string(), class.as_ref());
    rsx! {
        label { class: "{cls}",
            input {
                r#type: "checkbox",
                checked,
                onchange: move |e| onchange.call(e),
            }
            span { "{label}" }
        }
    }
}
