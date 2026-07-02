//! Guards the load-bearing write workarounds: both upsert facades must
//! persist the INSERT (new row) and UPDATE (existing row) paths. The upstream
//! `submit` no-ops the UPDATE and `RecordNotFound`s the INSERT — these tests
//! fail loudly if either facade ever regresses to that behaviour.
#![cfg(not(target_arch = "wasm32"))]

use marea_sync::handle_ext_test_support::upsert_by_pk;
use marea_sync::{delete_by_pk, upsert_all_columns};
use sea_orm::{ConnectionTrait, Database, DatabaseConnection, EntityTrait, Schema};

mod widget {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};
    use wavesyncdb::SyncEntity;

    #[derive(
        Clone, Debug, PartialEq, Eq, DeriveEntityModel, SyncEntity, Serialize, Deserialize,
    )]
    #[sea_orm(table_name = "widget")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub count: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

async fn mem_db() -> DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    let schema = Schema::new(db.get_database_backend());
    db.execute(&schema.create_table_from_entity(widget::Entity))
        .await
        .unwrap();
    db
}

fn w(id: &str, name: &str, count: i32) -> widget::Model {
    widget::Model {
        id: id.into(),
        name: name.into(),
        count,
    }
}

#[tokio::test]
async fn on_conflict_upsert_inserts_updates_and_never_duplicates() {
    let db = mem_db().await;

    upsert_all_columns(&db, &w("w1", "first", 1)).await.unwrap();
    let got = widget::Entity::find_by_id("w1".to_string())
        .one(&db)
        .await
        .unwrap()
        .expect("insert path must create the row");
    assert_eq!((got.name.as_str(), got.count), ("first", 1));

    upsert_all_columns(&db, &w("w1", "second", 2))
        .await
        .unwrap();
    let got = widget::Entity::find_by_id("w1".to_string())
        .one(&db)
        .await
        .unwrap()
        .expect("row must still exist after update");
    assert_eq!(
        (got.name.as_str(), got.count),
        ("second", 2),
        "update must persist every changed column (regression guard)"
    );

    let all = widget::Entity::find().all(&db).await.unwrap();
    assert_eq!(all.len(), 1, "upsert must keep exactly one row per PK");
}

#[tokio::test]
async fn find_then_write_upsert_inserts_updates_and_never_duplicates() {
    let db = mem_db().await;

    upsert_by_pk(&db, &w("w1", "first", 1)).await.unwrap();
    upsert_by_pk(&db, &w("w1", "second", 2)).await.unwrap();

    let all = widget::Entity::find().all(&db).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].name, "second");
    assert_eq!(all[0].count, 2);
}

#[tokio::test]
async fn delete_by_pk_removes_only_that_row() {
    let db = mem_db().await;
    upsert_all_columns(&db, &w("w1", "keep", 1)).await.unwrap();
    upsert_all_columns(&db, &w("w2", "drop", 2)).await.unwrap();

    delete_by_pk::<_, widget::Model>(&db, "w2").await.unwrap();

    let all = widget::Entity::find().all(&db).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "w1");
}
