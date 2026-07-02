//! [`Input`] — the `field-input` text control, optionally wrapped in a
//! `.field` block with a label and an inline error.

use dioxus::prelude::*;

use super::merge_class;

/// A styled `<input>`. When `label` or `error` is set the input is wrapped
/// in `div.field` with a `span.field-label` above and/or `span.field-error`
/// below; otherwise the bare input is rendered. `error` also switches the
/// input to the `field-input--error` treatment.
///
/// All global and `<input>` attributes pass through (`placeholder`,
/// `r#type`, `value`, `initial_value`, `autofocus`, `name`, …).
#[component]
pub fn Input(
    #[props(default)] label: Option<String>,
    #[props(default)] error: Option<String>,
    #[props(default)] oninput: Option<EventHandler<FormEvent>>,
    #[props(default)] onchange: Option<EventHandler<FormEvent>>,
    #[props(default)] onfocus: Option<EventHandler<FocusEvent>>,
    #[props(default)] onblur: Option<EventHandler<FocusEvent>>,
    #[props(default)] onkeydown: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] onkeyup: Option<EventHandler<KeyboardEvent>>,
    #[props(default)] class: Option<String>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = input)]
    attributes: Vec<Attribute>,
) -> Element {
    let mut cls = String::from("field-input");
    if error.is_some() {
        cls.push_str(" field-input--error");
    }
    let cls = merge_class(cls, class.as_ref());

    let input_el = rsx! {
        input {
            class: "{cls}",
            oninput: move |e| {
                if let Some(h) = &oninput {
                    h.call(e);
                }
            },
            onchange: move |e| {
                if let Some(h) = &onchange {
                    h.call(e);
                }
            },
            onfocus: move |e| {
                if let Some(h) = &onfocus {
                    h.call(e);
                }
            },
            onblur: move |e| {
                if let Some(h) = &onblur {
                    h.call(e);
                }
            },
            onkeydown: move |e| {
                if let Some(h) = &onkeydown {
                    h.call(e);
                }
            },
            onkeyup: move |e| {
                if let Some(h) = &onkeyup {
                    h.call(e);
                }
            },
            ..attributes,
        }
    };

    if label.is_none() && error.is_none() {
        return input_el;
    }

    rsx! {
        div { class: "field",
            if let Some(l) = &label {
                span { class: "field-label", "{l}" }
            }
            {input_el}
            if let Some(err) = &error {
                span { class: "field-error", "{err}" }
            }
        }
    }
}
