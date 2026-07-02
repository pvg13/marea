//! Stock login screen: email/password against the app's PocketBase, with an
//! optional inline "create account" secondary action (the single-screen
//! login+register shape Ascend ships — no separate register route).
//!
//! Apps brand it through the `logo` / `title` / `subtitle` props; apps that
//! outgrow it pass their own `login:` element to
//! [`AppShell`](crate::shell::AppShell) and build on the same `auth-screen`
//! CSS classes and [`marea_auth::AuthState`].

use dioxus::prelude::*;
use marea_auth::use_auth;

#[component]
pub fn LoginScreen(
    /// Brand mark rendered above the title.
    #[props(default)]
    logo: Option<Element>,
    #[props(default = String::from("Welcome"))] title: String,
    /// One-liner under the title (e.g. "Log in to sync your training
    /// across devices.").
    #[props(default)]
    subtitle: Option<String>,
    /// Show the "Create account" secondary button.
    #[props(default = true)]
    allow_register: bool,
) -> Element {
    let auth = use_auth();
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let error = use_signal(|| Option::<String>::None);
    let submitting = use_signal(|| false);

    // register == true → create account, else log in. Rebinding the (Copy)
    // signals where they're mutated keeps this closure `Fn` (both buttons
    // call it).
    let submit = move |register: bool| {
        let em = email().trim().to_string();
        let pw = password();
        if em.is_empty() || pw.is_empty() {
            let mut error = error;
            error.set(Some("Enter your email and password.".to_string()));
            return;
        }
        spawn(async move {
            let mut error = error;
            let mut submitting = submitting;
            submitting.set(true);
            error.set(None);
            let res = if register {
                auth.register(&em, &pw, None).await
            } else {
                auth.login(&em, &pw).await
            };
            if let Err(e) = res {
                error.set(Some(e.user_message()));
            }
            submitting.set(false);
        });
    };

    let busy = submitting();

    rsx! {
        div { class: "auth-screen",
            if let Some(l) = logo {
                div { class: "auth-screen__logo", {l} }
            }
            h1 { class: "auth-screen__title", "{title}" }
            if let Some(sub) = subtitle {
                p { class: "auth-screen__subtitle", "{sub}" }
            }
            div { class: "auth-screen__form",
                input {
                    id: "login-email",
                    class: "field-input",
                    r#type: "email",
                    value: "{email}",
                    placeholder: "Email",
                    autocapitalize: "none",
                    oninput: move |e| email.set(e.value()),
                }
                input {
                    id: "login-password",
                    class: "field-input",
                    r#type: "password",
                    value: "{password}",
                    placeholder: "Password",
                    oninput: move |e| password.set(e.value()),
                }
                if let Some(err) = error() {
                    div { class: "error-banner", "{err}" }
                }
                button {
                    class: "btn-primary btn-block",
                    disabled: busy,
                    onclick: move |_| submit(false),
                    if busy { "Please wait…" } else { "Log in" }
                }
                if allow_register {
                    button {
                        class: "btn-outline btn-block",
                        disabled: busy,
                        onclick: move |_| submit(true),
                        "Create account"
                    }
                }
            }
        }
    }
}
