//! Adaptive navigation shell: desktop sidebar ≥768px, bottom tab bar below —
//! or bottom-bar-everywhere in `mobile_only` mode (Ascend's phone-frame
//! design). All breakpoint logic lives in `marea.css`
//! (`.app-shell`/`.side-nav`/`.bottom-nav`); this component only renders the
//! structure.
//!
//! Apps use it as the body of their `#[layout(..)]` component:
//!
//! ```rust,ignore
//! #[component]
//! fn Shell() -> Element {
//!     rsx! {
//!         NavShell {
//!             items: nav_items(),          // Vec<NavItem<Route>>
//!             mobile_only: true,
//!             Outlet::<Route> {}
//!         }
//!     }
//! }
//! ```
//!
//! Icons are `Element` slots and must draw with `currentColor` — active /
//! inactive tinting is pure CSS on the tab class.

use dioxus::prelude::*;

/// One navigation destination. `route` doubles as the active-state identity
/// (`use_route::<R>() == route`).
#[derive(Clone, PartialEq)]
pub struct NavItem<R: Clone + PartialEq + 'static> {
    pub label: String,
    pub icon: Element,
    pub route: R,
}

impl<R: Clone + PartialEq + 'static> NavItem<R> {
    pub fn new(label: impl Into<String>, icon: Element, route: R) -> Self {
        Self {
            label: label.into(),
            icon,
            route,
        }
    }
}

#[component]
pub fn NavShell<R: Routable + Clone + PartialEq>(
    items: Vec<NavItem<R>>,
    /// Sidebar brand block (logo / app name). Desktop only.
    #[props(default)]
    brand: Option<Element>,
    /// Sidebar footer (theme toggle, sign-out, user chip). Desktop only.
    #[props(default)]
    footer: Option<Element>,
    /// Keep the bottom tab bar at every width (phone-frame apps).
    #[props(default)]
    mobile_only: bool,
    /// Page content — normally `Outlet::<R> {}`.
    children: Element,
) -> Element {
    let current: R = use_route::<R>();
    let shell_class = if mobile_only {
        "app-shell app-shell--mobile-only"
    } else {
        "app-shell"
    };

    rsx! {
        div { class: "{shell_class}",
            nav { class: "side-nav",
                if let Some(b) = brand {
                    div { class: "side-nav__brand", {b} }
                }
                div { class: "side-nav__links",
                    for item in items.iter() {
                        Link {
                            key: "{item.label}",
                            to: item.route.clone(),
                            class: if current == item.route { "side-nav__link side-nav__link--active" } else { "side-nav__link" },
                            span { class: "bottom-nav__icon", {item.icon.clone()} }
                            "{item.label}"
                        }
                    }
                }
                if let Some(f) = footer {
                    div { class: "side-nav__footer", {f} }
                }
            }
            main { class: "app-main", {children} }
            nav { class: "bottom-nav",
                for item in items.iter() {
                    Link {
                        key: "{item.label}",
                        to: item.route.clone(),
                        class: if current == item.route { "bottom-nav__tab bottom-nav__tab--active" } else { "bottom-nav__tab" },
                        span { class: "bottom-nav__icon", {item.icon.clone()} }
                        span { class: "bottom-nav__label", "{item.label}" }
                    }
                }
            }
        }
    }
}
