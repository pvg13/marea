//! App-wide toast surface.
//!
//! A single queue is provided once at the app root (AppShell does it) via
//! [`provide_toasts`] and rendered by [`ToastHost`]. Any component grabs a
//! [`Toasts`] handle with [`use_toasts`] and calls `.error(..)` /
//! `.success(..)` / `.info(..)` to surface a transient message.
//!
//! Toasts are for *transient* feedback (write failures in particular —
//! never swallow a sync error with `let _ =`). Inline `.error-banner`s still
//! make sense for *persistent*, in-context validation messages (e.g. a login
//! form).

use dioxus::prelude::*;

/// How long a toast stays up before auto-dismissing.
const AUTO_DISMISS_MS: u32 = 5_000;

/// Cross-target async sleep (tokio on native, gloo-timers on wasm).
async fn sleep_ms(ms: u32) {
    #[cfg(not(target_arch = "wasm32"))]
    tokio::time::sleep(std::time::Duration::from_millis(ms as u64)).await;
    #[cfg(target_arch = "wasm32")]
    gloo_timers::future::TimeoutFuture::new(ms).await;
}

/// Severity of a toast — drives its colour.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Error,
    Success,
    Info,
}

/// A single transient message.
#[derive(Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
}

/// `Copy` handle to the app-wide toast queue. Obtain via [`use_toasts`].
#[derive(Clone, Copy)]
pub struct Toasts {
    items: Signal<Vec<Toast>>,
    next_id: Signal<u64>,
}

impl Toasts {
    /// Enqueue a toast and schedule its auto-dismissal.
    pub fn push(mut self, kind: ToastKind, message: impl Into<String>) {
        let id = {
            let mut n = self.next_id.write();
            let id = *n;
            *n = n.wrapping_add(1);
            id
        };
        self.items.write().push(Toast {
            id,
            kind,
            message: message.into(),
        });

        let mut items = self.items;
        spawn(async move {
            sleep_ms(AUTO_DISMISS_MS).await;
            items.write().retain(|t| t.id != id);
        });
    }

    pub fn error(self, message: impl Into<String>) {
        self.push(ToastKind::Error, message);
    }

    pub fn success(self, message: impl Into<String>) {
        self.push(ToastKind::Success, message);
    }

    pub fn info(self, message: impl Into<String>) {
        self.push(ToastKind::Info, message);
    }

    /// Dismiss a specific toast (the close button).
    pub fn dismiss(mut self, id: u64) {
        self.items.write().retain(|t| t.id != id);
    }
}

/// Provide the queue. Call once, at the app root.
pub fn provide_toasts() {
    let items = use_signal(Vec::<Toast>::new);
    let next_id = use_signal(|| 0u64);
    use_context_provider(|| Toasts { items, next_id });
}

/// Grab the toast handle from context.
pub fn use_toasts() -> Toasts {
    use_context::<Toasts>()
}

/// Renders the stacked toasts. Mount once near the app root (AppShell does).
#[component]
pub fn ToastHost() -> Element {
    let toasts = use_toasts();
    rsx! {
        div { class: "toast-host",
            for toast in (toasts.items)() {
                ToastItem { key: "{toast.id}", toast: toast.clone() }
            }
        }
    }
}

#[component]
fn ToastItem(toast: Toast) -> Element {
    let toasts = use_toasts();
    let id = toast.id;
    let kind_class = match toast.kind {
        ToastKind::Error => "toast toast--error",
        ToastKind::Success => "toast toast--success",
        ToastKind::Info => "toast toast--info",
    };
    rsx! {
        div { class: "{kind_class}", role: "status",
            span { class: "toast__msg", "{toast.message}" }
            button {
                class: "toast__close",
                r#type: "button",
                "aria-label": "Dismiss",
                onclick: move |_| { toasts.dismiss(id); },
                "×"
            }
        }
    }
}
