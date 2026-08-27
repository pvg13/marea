//! The app's one `Route` enum.
//!
//! There is deliberately no second router for phones. When a screen needs to
//! look different on a narrow viewport, branch *inside* that screen; a
//! parallel route enum is how an app ends up with two view layers that drift.
//!
//! Each variant's name must match a component in scope, so features export
//! their screen under the route's name (`features::home::Home`).

use dioxus::prelude::*;

use crate::features::home::Home;
use crate::features::settings::Settings;
{%- if with_example and wavesync %}
use crate::features::items::Items;
{%- endif %}
use crate::shell::Shell;

#[derive(Clone, Routable, PartialEq, Debug)]
pub enum Route {
    #[layout(Shell)]
    #[route("/")]
    Home {},
    {%- if with_example and wavesync %}
    #[route("/items")]
    Items {},
    {%- endif %}
    #[route("/settings")]
    Settings {},
}
