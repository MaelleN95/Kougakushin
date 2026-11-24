// Accès SQL aux manuels. Renvoie les erreurs sqlx telles quelles.

use sqlx::SqlitePool;

use super::model::Book;

// Tous les manuels, par ordre alphabétique.
pub(super) async fn find_all(pool: &SqlitePool) -> Result<Vec<Book>, sqlx::Error> {
    sqlx::query_as::<_, Book>("SELECT id, title, created_at FROM book ORDER BY title")
        .fetch_all(pool)
        .await
}

pub(super) async fn insert(pool: &SqlitePool, title: &str) -> Result<Book, sqlx::Error> {
    sqlx::query_as::<_, Book>("INSERT INTO book (title) VALUES (?) RETURNING id, title, created_at")
        .bind(title)
        .fetch_one(pool)
        .await
}

// Renomme un manuel ; None si l'id n'existe pas.
pub(super) async fn update(
    pool: &SqlitePool,
    id: i64,
    title: &str,
) -> Result<Option<Book>, sqlx::Error> {
    sqlx::query_as::<_, Book>(
        "UPDATE book SET title = ? WHERE id = ? RETURNING id, title, created_at",
    )
    .bind(title)
    .bind(id)
    .fetch_optional(pool)
    .await
}

// Supprime un manuel ; false si l'id n'existe pas.
pub(super) async fn delete(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    let result = sqlx::query("DELETE FROM book WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

#[cfg(test)]
mod tests {
    use crate::error::{is_foreign_key_violation, is_unique_violation};

    async fn pool() -> sqlx::SqlitePool {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[test]
    fn un_titre_deja_pris_viole_l_unicite_sans_tenir_compte_de_la_casse() {
        tauri::async_runtime::block_on(async {
            let pool = pool().await;
            super::insert(&pool, "Marugoto A1").await.unwrap();
            let error = super::insert(&pool, "marugoto a1").await.unwrap_err();
            assert!(is_unique_violation(&error), "erreur inattendue : {error:?}");
        });
    }

    #[test]
    fn supprimer_un_manuel_non_vide_viole_la_cle_etrangere() {
        tauri::async_runtime::block_on(async {
            let pool = pool().await;
            super::insert(&pool, "Marugoto A1").await.unwrap(); // id 1
            sqlx::query("INSERT INTO lesson (title, date, book_id) VALUES ('L1', '2026-11-18', 1)")
                .execute(&pool)
                .await
                .unwrap();
            let error = super::delete(&pool, 1).await.unwrap_err();
            assert!(
                is_foreign_key_violation(&error),
                "erreur inattendue : {error:?}"
            );
        });
    }
}
