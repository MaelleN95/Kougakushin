// Accès SQL aux réglages.

use sqlx::{SqliteConnection, SqlitePool};

// Toutes les lignes (clé, valeur).
pub(super) async fn find_all(pool: &SqlitePool) -> Result<Vec<(String, String)>, sqlx::Error> {
    sqlx::query_as("SELECT key, value FROM setting")
        .fetch_all(pool)
        .await
}

// Crée ou remplace une valeur. ON CONFLICT ... DO UPDATE : "insert or update" (upsert).
pub(super) async fn upsert(
    conn: &mut SqliteConnection,
    key: &str,
    value: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO setting (key, value) VALUES (?, ?)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(conn)
    .await?;
    Ok(())
}
