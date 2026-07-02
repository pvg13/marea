//! The marea-ui component library.
//!
//! Every component emits only the semantic classes defined in
//! `assets/marea.css` — no utility classes, no inline styles (the sole
//! exception is [`LoadingSkeleton`]'s per-line widths, which are inherently
//! dynamic). Each component accepts an optional `class` prop appended after
//! its base classes so callers can attach app-specific hooks.

mod avatar;
mod button;
mod card;
mod checkbox;
mod display;
mod feedback;
mod header;
mod input;
mod pill;
mod row;

pub use avatar::{Avatar, AvatarSize};
pub use button::{Button, ButtonVariant, IconButton};
pub use card::{Card, CardPad};
pub use checkbox::Checkbox;
pub use display::{Divider, Eyebrow};
pub use feedback::{EmptyState, GatePending, LoadingSkeleton};
pub use header::{ScreenHeader, SectionHeader, TopBar};
pub use input::Input;
pub use pill::{Pill, PillTone};
pub use row::{InfoRow, Row};

/// Append the caller-supplied `class` prop (if any) after the base classes.
pub(crate) fn merge_class(base: String, extra: Option<&String>) -> String {
    match extra {
        Some(c) if !c.is_empty() => format!("{base} {c}"),
        _ => base,
    }
}
