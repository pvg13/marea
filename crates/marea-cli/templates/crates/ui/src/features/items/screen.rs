//! DELETE ME — sample slice showing the marea layering.
//!
//! Read this screen for what it *doesn't* contain: no `wavesyncdb`, no
//! `SyncHandle`, no `#[cfg(target_arch)]`, no SQL, no manual refresh. It asks
//! `data` for a repository and renders it. The same file runs on a phone, a
//! desktop window and a browser tab, and a row added on one appears on the
//! others without this code knowing that sync exists.

use dioxus::prelude::*;
use data::repo::use_items;
use marea_ui::{
    Button, ButtonVariant, Card, CardPad, EmptyState, Input, Row, SectionHeader, TopBar, use_toasts,
};

use crate::i18n::use_lang;

use super::i18n::strings;

#[component]
pub fn Items() -> Element {
    let t = strings(use_lang());
    let items = use_items();
    let all = items.all();
    // Computed here rather than inline: rsx interpolation takes field access,
    // not method calls with arguments.
    let remaining = t.remaining(items.remaining());

    rsx! {
        div { class: "page",
            // No `ScreenHeader` repeating the title the top bar already shows.
            TopBar { title: "{t.title}" }
            div { class: "page__body",

                AddItem {}

                if all.is_empty() {
                    Card { pad: CardPad::Lg,
                        EmptyState {
                            title: "{t.empty_title}",
                            description: "{t.empty_body}",
                        }
                    }
                } else {
                    SectionHeader { title: "{remaining}" }
                    Card { pad: CardPad::Md,
                        for item in all {
                            ItemRow { key: "{item.id}", item: item.clone() }
                        }
                    }
                }
            }
        }
    }
}

/// One row. A component only this feature uses, so it lives in this feature —
/// it moves to `ui/src/components/` the day a second feature needs it.
#[component]
fn ItemRow(item: domain::Item) -> Element {
    let items = use_items();
    let toasts = use_toasts();
    let t = strings(use_lang());

    let toggle = {
        let items = items.clone();
        let item = item.clone();
        move |_| {
            let items = items.clone();
            let toggled = item.toggled();
            let toasts = toasts;
            spawn(async move {
                if let Err(e) = items.save(&toggled).await {
                    toasts.error(e.to_string());
                }
            });
        }
    };

    let remove = {
        let items = items.clone();
        let id = item.id.clone();
        move |_| {
            let items = items.clone();
            let id = id.clone();
            let toasts = toasts;
            spawn(async move {
                if let Err(e) = items.delete(&id).await {
                    toasts.error(e.to_string());
                }
            });
        }
    };

    rsx! {
        Row {
            title: "{item.title}",
            // `Option`, not an empty string: `Some("")` renders an empty
            // subtitle element and leaves a gap under every open item.
            subtitle: item.done.then(|| t.done.to_string()),
            onclick: toggle,
            trailing: rsx! {
                Button {
                    variant: ButtonVariant::Ghost,
                    small: true,
                    onclick: remove,
                    "{t.delete}"
                }
            },
        }
    }
}

/// The add form. Validation lives in `domain::Item::new`, so an invalid item
/// cannot be constructed here even by mistake.
#[component]
fn AddItem() -> Element {
    let t = strings(use_lang());
    let items = use_items();
    let toasts = use_toasts();
    let mut title = use_signal(String::new);
    let mut error: Signal<Option<String>> = use_signal(|| None);

    let submit = move |_| {
        let items = items.clone();
        let text = title();
        // A user-minted UUID: WaveSyncDB merges per row and per column, so
        // primary keys must be unique across devices without a server.
        let id = uuid::Uuid::new_v4().to_string();
        match domain::Item::new(id, &text) {
            Err(e) => error.set(Some(e.to_string())),
            Ok(item) => {
                error.set(None);
                title.set(String::new());
                spawn(async move {
                    if let Err(e) = items.save(&item).await {
                        toasts.error(e.to_string());
                    }
                });
            }
        }
    };

    rsx! {
        Card { pad: CardPad::Md,
            Input {
                label: "{t.add_label}",
                value: "{title}",
                // `Option`, not a string: marea's `Input` paints the error state
                // for any `Some`, so an empty `String` would show a red border
                // permanently.
                error: error(),
                oninput: move |e: FormEvent| title.set(e.value()),
            }
            div { class: "form-actions",
                Button { onclick: submit, "{t.add}" }
            }
        }
    }
}
