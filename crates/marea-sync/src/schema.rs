//! Idempotent schema reconciliation primitives.
//!
//! WaveSyncDB's `get_schema_registry(scope).sync()` creates tables fresh
//! (`IF NOT EXISTS`); column additions to **existing** installs go through
//! these helpers, PRAGMA-guarded so re-running on every launch is safe.
//! Forward-only and additive — never destructive, never a wipe. New NOT NULL
//! columns need a SQL `DEFAULT`. Renames (ADD-new + COPY + DROP-old) stay
//! app-local: they're one-off schema history, not a reusable primitive.

use sea_orm::{ConnectionTrait, DbBackend, DbErr, FromQueryResult, Statement};

#[derive(Debug, FromQueryResult)]
struct PragmaColumnRow {
    name: String,
}

/// Add a column to an existing table only if it is missing. Safe to call on
/// every launch. No-ops when the table itself doesn't exist yet (the registry
/// `sync()` will create it with the column already in the entity).
pub async fn add_column_if_missing<C: ConnectionTrait>(
    db: &C,
    table: &str,
    column: &str,
    sql_type: &str,
) -> Result<(), DbErr> {
    let rows = PragmaColumnRow::find_by_statement(Statement::from_string(
        DbBackend::Sqlite,
        format!("PRAGMA table_info(\"{table}\")"),
    ))
    .all(db)
    .await?;
    if rows.is_empty() {
        return Ok(()); // table not created yet
    }
    if rows.iter().any(|r| r.name == column) {
        return Ok(()); // already present
    }
    db.execute_unprepared(&format!(
        "ALTER TABLE \"{table}\" ADD COLUMN \"{column}\" {sql_type}"
    ))
    .await?;
    Ok(())
}

/// [`add_column_if_missing`] over a `(column, sql_type)` list — the shape of
/// an app's `reconcile()` body.
pub async fn ensure_columns<C: ConnectionTrait>(
    db: &C,
    table: &str,
    columns: &[(&str, &str)],
) -> Result<(), DbErr> {
    for (column, sql_type) in columns {
        add_column_if_missing(db, table, column, sql_type).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    async fn column_names<C: ConnectionTrait>(db: &C, table: &str) -> Vec<String> {
        PragmaColumnRow::find_by_statement(Statement::from_string(
            DbBackend::Sqlite,
            format!("PRAGMA table_info(\"{table}\")"),
        ))
        .all(db)
        .await
        .unwrap()
        .into_iter()
        .map(|c| c.name)
        .collect()
    }

    #[tokio::test]
    async fn adds_column_once_and_is_idempotent() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        db.execute_unprepared("CREATE TABLE t (id TEXT PRIMARY KEY)")
            .await
            .unwrap();

        ensure_columns(
            &db,
            "t",
            &[("flag", "INTEGER NOT NULL DEFAULT 0"), ("note", "TEXT")],
        )
        .await
        .unwrap();
        // Running again must not error (PRAGMA guard).
        ensure_columns(
            &db,
            "t",
            &[("flag", "INTEGER NOT NULL DEFAULT 0"), ("note", "TEXT")],
        )
        .await
        .unwrap();

        let cols = column_names(&db, "t").await;
        assert!(cols.contains(&"flag".to_string()));
        assert!(cols.contains(&"note".to_string()));
        assert_eq!(cols.iter().filter(|c| *c == "flag").count(), 1);
    }

    #[tokio::test]
    async fn missing_table_is_a_no_op() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        add_column_if_missing(&db, "nonexistent", "c", "TEXT")
            .await
            .unwrap();
    }
}
