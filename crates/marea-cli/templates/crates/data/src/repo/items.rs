//! DELETE ME — sample slice showing the marea layering.
//!
//! A repository: the whole vocabulary a screen needs for one aggregate, and
//! the only place that knows how a `domain::Item` becomes a synced row.
//!
//! Every method here works identically on desktop, phone and browser.
//! `use_synced_table` and `SyncHandleExt` are both cross-target — one reads,
//! one writes — which is why `ui` can stay free of target checks.

use dioxus::prelude::*;
use marea_sync::SyncHandleExt;
use marea_sync::wavesyncdb::dioxus::{SyncHandle, use_synced_table};

use crate::entities::item;
use crate::repo::RepoError;

/// Typed access to the item table for the signed-in account.
#[derive(Clone)]
pub struct Items {
    handle: SyncHandle,
    rows: Signal<Vec<item::Model>>,
}

/// Call from any component below `SyncProvider`.
///
/// The returned `rows` signal is live: when another device changes a row, the
/// screen re-renders. No refresh button, no manual invalidation.
pub fn use_items() -> Items {
    let handle = use_context::<SyncHandle>();
    let rows = use_synced_table::<item::Model>(handle.clone());
    Items { handle, rows }
}

impl Items {
    /// Every item, newest first, as domain types.
    ///
    /// Rows that fail to convert are skipped rather than panicking: they come
    /// from other devices, possibly running an older build, and one bad row
    /// must not take down the screen.
    pub fn all(&self) -> Vec<domain::Item> {
        let mut items: Vec<domain::Item> =
            self.rows.read().iter().filter_map(to_domain).collect();
        items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        items
    }

    pub fn remaining(&self) -> usize {
        domain::item::remaining(&self.all())
    }

    /// Insert or update by primary key.
    pub async fn save(&self, item: &domain::Item) -> Result<(), RepoError> {
        self.handle.submit_upsert(&from_domain(item)).await?;
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<(), RepoError> {
        self.handle.delete_row::<item::Model>(id).await?;
        Ok(())
    }
}

fn to_domain(row: &item::Model) -> Option<domain::Item> {
    Some(domain::Item {
        id: row.id.clone(),
        title: row.title.clone(),
        done: row.done,
        created_at: chrono::DateTime::parse_from_rfc3339(&row.created_at)
            .ok()?
            .with_timezone(&chrono::Utc),
    })
}

fn from_domain(item: &domain::Item) -> item::Model {
    item::Model {
        id: item.id.clone(),
        title: item.title.clone(),
        done: item.done,
        created_at: item.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_domain_item_survives_a_round_trip_through_the_row() {
        let item = domain::Item::new("abc", "milk").unwrap();
        let back = to_domain(&from_domain(&item)).expect("round trip");
        assert_eq!(back, item);
    }

    #[test]
    fn an_unparseable_timestamp_is_skipped_rather_than_panicking() {
        let row = item::Model {
            id: "x".into(),
            title: "from a future build".into(),
            done: false,
            created_at: "not a date".into(),
        };
        assert!(to_domain(&row).is_none());
    }
}
