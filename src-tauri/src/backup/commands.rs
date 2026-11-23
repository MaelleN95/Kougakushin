// Commandes Tauri de sauvegarde : fenêtres Windows "Enregistrer sous" / "Ouvrir", puis lecture
// ou écriture du fichier par Rust.

use std::path::{Path, PathBuf};

use chrono::Utc;
use sqlx::SqlitePool;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use super::model::{Backup, ImportReport};
use super::repository;
use crate::error::AppError;

// Instantané de toutes les données, en JSON lisible (indenté).
async fn export_json(pool: &SqlitePool) -> Result<String, AppError> {
    let backup =
        repository::snapshot(pool, Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()).await?;
    serde_json::to_string_pretty(&backup).map_err(|e| AppError::InvalidBackup(e.to_string()))
}

// Sauvegarde automatique avant un import (D23), dans le dossier de l'application :
// %APPDATA%\com.kougakushin.app\backups\avant-import-AAAAMMJJ-HHMMSS.json
async fn write_safety_backup(app: &AppHandle, pool: &SqlitePool) -> Result<PathBuf, AppError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::InvalidBackup(e.to_string()))?
        .join("backups");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!(
        "avant-import-{}.json",
        Utc::now().format("%Y%m%d-%H%M%S")
    ));
    std::fs::write(&path, export_json(pool).await?)?;
    Ok(path)
}

// Fichier lu => import complet. Le fichier est entièrement vérifié avant toute modification.
async fn import_file(
    pool: &SqlitePool,
    path: &Path,
    safety_backup: PathBuf,
) -> Result<ImportReport, AppError> {
    let backup = Backup::from_json(&std::fs::read_to_string(path)?)?;
    repository::replace_all(pool, &backup).await.map_err(|e| {
        // Contrainte violée (clé étrangère, CHECK...) : la transaction est annulée.
        log::warn!("Import refusé : {e}");
        AppError::InvalidBackup("données incohérentes, rien n'a été modifié".into())
    })?;
    Ok(ImportReport {
        lessons: backup.lessons.len(),
        items: backup.items.len(),
        cards: backup.cards.len(),
        review_logs: backup.review_logs.len(),
        safety_backup: safety_backup.display().to_string(),
    })
}

// Ouvre "Enregistrer sous", écrit le fichier. None si la fenêtre est fermée sans choisir.
#[tauri::command]
pub(crate) async fn export_data(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<Option<String>, AppError> {
    let file_name = format!("kougakushin-{}.json", Utc::now().format("%Y-%m-%d"));
    // Fenêtre bloquante : autorisée ici, la commande ne tourne pas sur le fil principal.
    let Some(choice) = app
        .dialog()
        .file()
        .set_title("Exporter mes données")
        .set_file_name(file_name)
        .add_filter("Sauvegarde Kōgakushin", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path = choice
        .into_path()
        .map_err(|e| AppError::InvalidBackup(e.to_string()))?;
    std::fs::write(&path, export_json(pool.inner()).await?)?;
    Ok(Some(path.display().to_string()))
}

// Ouvre "Ouvrir", sauvegarde l'état actuel, puis remplace tout par le fichier (D23).
// La confirmation est demandée par l'écran avant l'appel.
#[tauri::command]
pub(crate) async fn import_data(
    app: AppHandle,
    pool: State<'_, SqlitePool>,
) -> Result<Option<ImportReport>, AppError> {
    let Some(choice) = app
        .dialog()
        .file()
        .set_title("Importer une sauvegarde (remplace toutes les données)")
        .add_filter("Sauvegarde Kōgakushin", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = choice
        .into_path()
        .map_err(|e| AppError::InvalidBackup(e.to_string()))?;
    let safety_backup = write_safety_backup(&app, pool.inner()).await?;
    import_file(pool.inner(), &path, safety_backup)
        .await
        .map(Some)
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::super::model::Backup;
    use super::super::repository;
    use super::{export_json, import_file};

    async fn empty_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    // Une leçon, un mot (2 cartes), une révision, un réglage.
    async fn seeded_pool() -> SqlitePool {
        let pool = empty_pool().await;
        sqlx::raw_sql(
            "INSERT INTO lesson (title, date) VALUES ('L1', '2026-11-18');
             INSERT INTO item (lesson_id, kind, kana, kanji, meaning) VALUES (1, 'vocab', 'たべる', '食べる', 'manger');
             INSERT INTO card (item_id, direction) VALUES (1, 'fr_jp'), (1, 'jp_fr');
             UPDATE card SET state = 2, reps = 3, stability = 4.5 WHERE id = 1;
             INSERT INTO review_log (card_id, rating, state, elapsed_days, scheduled_days, reviewed_at)
                 VALUES (1, 3, 0, 0, 1, '2026-11-20 10:00:00');
             INSERT INTO setting (key, value) VALUES ('new_cards_per_day', '15');",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    fn temp_file(name: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("kougakushin-test-{name}.json"));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn export_puis_import_restitue_exactement_les_donnees() {
        tauri::async_runtime::block_on(async {
            let source = seeded_pool().await;
            let json = export_json(&source).await.unwrap();
            let path = temp_file("aller-retour", &json);

            // Base cible avec d'autres données, qui doivent disparaître (l'import remplace).
            let target = empty_pool().await;
            sqlx::query("INSERT INTO lesson (title, date) VALUES ('Autre', '2026-01-01')")
                .execute(&target)
                .await
                .unwrap();
            let report = import_file(&target, &path, "x".into()).await.unwrap();
            assert_eq!(
                (
                    report.lessons,
                    report.items,
                    report.cards,
                    report.review_logs
                ),
                (1, 1, 2, 1)
            );

            let before = repository::snapshot(&source, "t".into()).await.unwrap();
            let after = repository::snapshot(&target, "t".into()).await.unwrap();
            assert_eq!(before, after); // progression FSRS et historique compris (D24)
        });
    }

    #[test]
    fn fichier_incoherent_refuse_sans_rien_modifier() {
        tauri::async_runtime::block_on(async {
            let pool = seeded_pool().await;
            let before = repository::snapshot(&pool, "t".into()).await.unwrap();

            // Un élément qui pointe vers une leçon absente du fichier : clé étrangère violée.
            let mut broken: Backup = before.clone();
            broken.lessons.clear();
            let path = temp_file("incoherent", &serde_json::to_string(&broken).unwrap());
            assert!(import_file(&pool, &path, "x".into()).await.is_err());

            assert_eq!(
                repository::snapshot(&pool, "t".into()).await.unwrap(),
                before
            );
        });
    }

    #[test]
    fn fichier_etranger_ou_modifie_refuse() {
        assert!(Backup::from_json("{}").is_err());
        assert!(Backup::from_json("pas du json").is_err());
        let wrong_format = r#"{"format":"autre","version":1,"exported_at":"","lessons":[],
            "items":[],"cards":[],"review_logs":[],"settings":[]}"#;
        assert!(Backup::from_json(wrong_format).is_err());
        let extra_field = r#"{"format":"kougakushin","version":1,"exported_at":"","lessons":[],
            "items":[],"cards":[],"review_logs":[],"settings":[],"pirate":true}"#;
        assert!(Backup::from_json(extra_field).is_err());
    }
}
