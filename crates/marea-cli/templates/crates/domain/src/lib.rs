//! {{ title }} domain model.
//!
//! Pure Rust: the types, invariants and calculations that define what this
//! app *is*. No dioxus, no sea-orm, no wavesyncdb — nothing here knows that a
//! screen or a database exists.
//!
//! That boundary is the point. Domain rules tested here need no runtime, no
//! fixtures and no async, and they survive a rewrite of everything above
//! them. CI asserts the boundary with `cargo tree -p domain`.
//!
//! Put here: value types, state machines, validation, calculations.
//! Put in `data`: anything that persists or syncs.
//! Put in `ui`: anything that renders.
{% if with_example and wavesync %}
pub mod item;

pub use item::{Item, ItemError};
{% else %}
// Your types go here.
{% endif %}
