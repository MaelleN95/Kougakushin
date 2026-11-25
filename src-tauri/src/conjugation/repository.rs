// Accès SQL de la conjugaison : formes connues (table conjugation_form) et mots à vérifier.

use sqlx::{Executor, Sqlite, SqliteConnection, SqlitePool};

use super::model::VerbForm;

// Formes que je connais, dans l'ordre de VerbForm::ALL. Une valeur inconnue en base est ignorée.
pub(crate) async fn known_forms<'c, E>(executor: E) -> Result<Vec<VerbForm>, sqlx::Error>
where
    E: Executor<'c, Database = Sqlite>,
{
    let rows: Vec<String> = sqlx::query_scalar("SELECT form FROM conjugation_form WHERE known = 1")
        .fetch_all(executor)
        .await?;
    Ok(VerbForm::ALL
        .into_iter()
        .filter(|form| rows.iter().any(|r| r == form.as_str()))
        .collect())
}

// Marque une forme comme connue ou non. Upsert : la ligne peut manquer (import d'un fichier
// qui ne la contient pas).
pub(super) async fn set_known(
    conn: &mut SqliteConnection,
    form: VerbForm,
    known: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO conjugation_form (form, known) VALUES (?1, ?2)
         ON CONFLICT (form) DO UPDATE SET known = ?2",
    )
    .bind(form.as_str())
    .bind(known)
    .execute(conn)
    .await?;
    Ok(())
}

// Mot de vocabulaire, pour repérer les verbes au type manquant ou incohérent.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct VocabRow {
    pub(super) id: i64,
    pub(super) kana: String,
    pub(super) meaning: String,
    pub(super) word_type: Option<String>,
}

pub(super) async fn find_vocab(pool: &SqlitePool) -> Result<Vec<VocabRow>, sqlx::Error> {
    sqlx::query_as::<_, VocabRow>(
        "SELECT id, kana, meaning, word_type FROM item WHERE kind = 'vocab' ORDER BY id",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use sqlx::sqlite::SqlitePoolOptions;

    use super::super::model::VerbForm;
    use super::{known_forms, set_known};

    #[test]
    fn formes_connues_par_defaut_puis_modifiees() {
        tauri::async_runtime::block_on(async {
            let pool = SqlitePoolOptions::new()
                .max_connections(1)
                .connect("sqlite::memory:")
                .await
                .unwrap();
            sqlx::migrate!("./migrations").run(&pool).await.unwrap();
            assert_eq!(known_forms(&pool).await.unwrap(), [VerbForm::Masu]);

            let mut conn = pool.acquire().await.unwrap();
            set_known(&mut conn, VerbForm::Te, true).await.unwrap();
            set_known(&mut conn, VerbForm::Masu, false).await.unwrap();
            // Ligne absente (fichier importé incomplet) : créée par l'upsert.
            sqlx::query("DELETE FROM conjugation_form WHERE form = 'nai'")
                .execute(&mut *conn)
                .await
                .unwrap();
            set_known(&mut conn, VerbForm::Nai, true).await.unwrap();
            drop(conn);
            assert_eq!(
                known_forms(&pool).await.unwrap(),
                [VerbForm::Te, VerbForm::Nai]
            );
        });
    }
}
