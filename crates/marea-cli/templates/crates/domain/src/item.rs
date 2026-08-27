//! DELETE ME — sample slice showing the marea layering. Copy the shape, then
//! remove it (this file, `data/src/entities/item.rs`, `data/src/repo/items.rs`
//! and `ui/src/features/items/`).
//!
//! What belongs at this layer: the type, what makes it valid, and what you can
//! compute from it. Notice what is absent — no `id` generation strategy tied to
//! a database, no serde attributes chosen to please a wire format, no
//! `Signal`. Those decisions live one layer out, which is why this file's
//! tests need nothing but `cargo test -p domain`.

use serde::{Deserialize, Serialize};

/// A thing the user keeps a list of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub done: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ItemError {
    #[error("an item needs a title")]
    EmptyTitle,
    #[error("titles are limited to {max} characters")]
    TitleTooLong { max: usize },
}

impl Item {
    pub const MAX_TITLE: usize = 200;

    /// The only way to mint a valid item. Validation lives with the type, so
    /// no caller — screen, repository or test — can construct an invalid one.
    pub fn new(id: impl Into<String>, title: &str) -> Result<Self, ItemError> {
        let title = title.trim();
        if title.is_empty() {
            return Err(ItemError::EmptyTitle);
        }
        if title.chars().count() > Self::MAX_TITLE {
            return Err(ItemError::TitleTooLong {
                max: Self::MAX_TITLE,
            });
        }
        Ok(Self {
            id: id.into(),
            title: title.to_string(),
            done: false,
            created_at: chrono::Utc::now(),
        })
    }

    pub fn toggled(&self) -> Self {
        Self {
            done: !self.done,
            ..self.clone()
        }
    }
}

/// How many of a list remain open. A pure function over the domain type —
/// the screen renders this, it does not compute it.
pub fn remaining(items: &[Item]) -> usize {
    items.iter().filter(|i| !i.done).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_title_is_required() {
        assert_eq!(Item::new("1", "   "), Err(ItemError::EmptyTitle));
    }

    #[test]
    fn titles_are_trimmed() {
        assert_eq!(Item::new("1", "  milk ").unwrap().title, "milk");
    }

    #[test]
    fn overlong_titles_are_rejected() {
        let long = "x".repeat(Item::MAX_TITLE + 1);
        assert_eq!(
            Item::new("1", &long),
            Err(ItemError::TitleTooLong {
                max: Item::MAX_TITLE
            })
        );
    }

    #[test]
    fn toggling_flips_done_and_keeps_everything_else() {
        let a = Item::new("1", "milk").unwrap();
        let b = a.toggled();
        assert!(b.done);
        assert_eq!(b.id, a.id);
        assert_eq!(b.title, a.title);
        assert_eq!(b.created_at, a.created_at);
    }

    #[test]
    fn remaining_counts_only_open_items() {
        let a = Item::new("1", "milk").unwrap();
        let b = Item::new("2", "bread").unwrap().toggled();
        assert_eq!(remaining(&[a, b]), 1);
    }
}
