//! The navigation layout every route renders inside.
//!
//! `NavShell` is responsive on its own: a sidebar at ≥768px, bottom tabs
//! below. That is why one route enum covers phone, desktop and browser.

use dioxus::prelude::*;
use marea_ui::{NavItem, NavShell};

use crate::components::{icon, ThemeToggle};
{%- if auth %}
use crate::components::SignOut;
{%- endif %}
use crate::routes::Route;

fn nav_items() -> Vec<NavItem<Route>> {
    vec![
        NavItem::new(
            "Home",
            icon("M3 11l9-8 9 8M5 10v10a1 1 0 001 1h4v-6h4v6h4a1 1 0 001-1V10"),
            Route::Home {},
        ),
        {%- if with_example and wavesync %}
        NavItem::new(
            "Items",
            icon("M4 6h16M4 12h16M4 18h16"),
            Route::Items {},
        ),
        {%- endif %}
        NavItem::new(
            "Settings",
            icon("M12 2v3m0 14v3M4.9 4.9l2.1 2.1m10 10l2.1 2.1M2 12h3m14 0h3M4.9 19.1l2.1-2.1m10-10l2.1-2.1M12 9a3 3 0 100 6 3 3 0 000-6z"),
            Route::Settings {},
        ),
    ]
}

#[component]
pub fn Shell() -> Element {
    rsx! {
        {%- if android_back %}
        // Android's system back gesture pops the router instead of killing
        // the app. Must sit inside `Router` (it reads the navigator) and
        // inside the authed subtree so it resets on logout. Renders nothing,
        // and does nothing off Android — no cfg needed at the call site.
        marea_ui::back_gesture::GlobalBackHandler {}
        {%- endif %}

        NavShell {
            items: nav_items(),
            brand: rsx! { "{{ title }}" },
            footer: rsx! {
                ThemeToggle {}
                {%- if auth %}
                SignOut {}
                {%- endif %}
            },
            Outlet::<Route> {}
        }
    }
}
