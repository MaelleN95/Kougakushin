// Contenu d'un fichier de sauvegarde : une liste par table, colonne pour colonne.
// deny_unknown_fields : un fichier modifié à la main ou d'une autre application est refusé
// au lieu d'être importé à moitié.

use crate::error::AppError;

// Identifie nos fichiers, et leur format (à augmenter si la forme change).
pub(super) const FORMAT: &str = "kougakushin";
// 2 : manuels (book) ajoutés ; 3 : sens des mots en liste (item_meaning, D43).
pub(super) const VERSION: u32 = 3;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct BookRecord {
    pub(super) id: i64,
    pub(super) title: String,
    pub(super) created_at: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct LessonRecord {
    pub(super) id: i64,
    pub(super) book_id: Option<i64>,
    pub(super) title: String,
    pub(super) date: String,
    pub(super) notes: Option<String>,
    pub(super) created_at: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct ItemRecord {
    pub(super) id: i64,
    pub(super) lesson_id: i64,
    pub(super) kind: String,
    pub(super) active: bool,
    pub(super) kana: Option<String>,
    pub(super) romaji: Option<String>,
    pub(super) kanji: Option<String>,
    pub(super) meaning: Option<String>,
    pub(super) word_type: Option<String>,
    pub(super) title: Option<String>,
    pub(super) explanation: Option<String>,
    pub(super) example_jp: Option<String>,
    pub(super) example_fr: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
}

// Un sens d'un mot. ItemRecord garde la colonne meaning : texte dérivé de ces sens ("je; moi").
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct ItemMeaningRecord {
    pub(super) item_id: i64,
    pub(super) position: i64,
    pub(super) meaning: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct CardRecord {
    pub(super) id: i64,
    pub(super) item_id: i64,
    pub(super) direction: String,
    pub(super) next_review_at: String,
    pub(super) stability: f64,
    pub(super) difficulty: f64,
    pub(super) elapsed_days: i64,
    pub(super) scheduled_days: i64,
    pub(super) reps: i64,
    pub(super) lapses: i64,
    pub(super) state: i64,
    pub(super) last_review: Option<String>,
    pub(super) created_at: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct ReviewLogRecord {
    pub(super) id: i64,
    pub(super) card_id: i64,
    pub(super) rating: i64,
    pub(super) state: i64,
    pub(super) elapsed_days: i64,
    pub(super) scheduled_days: i64,
    pub(super) reviewed_at: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
#[serde(deny_unknown_fields)]
pub(super) struct SettingRecord {
    pub(super) key: String,
    pub(super) value: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Backup {
    pub(super) format: String,
    pub(super) version: u32,
    pub(super) exported_at: String, // UTC
    pub(super) books: Vec<BookRecord>,
    pub(super) lessons: Vec<LessonRecord>,
    pub(super) items: Vec<ItemRecord>,
    pub(super) item_meanings: Vec<ItemMeaningRecord>,
    pub(super) cards: Vec<CardRecord>,
    pub(super) review_logs: Vec<ReviewLogRecord>,
    pub(super) settings: Vec<SettingRecord>,
}

impl Backup {
    // Texte JSON => sauvegarde, refusée si ce n'est pas un fichier Kōgakushin de version connue.
    pub(super) fn from_json(text: &str) -> Result<Backup, AppError> {
        let backup: Backup =
            serde_json::from_str(text).map_err(|e| AppError::InvalidBackup(e.to_string()))?;
        if backup.format != FORMAT {
            return Err(AppError::InvalidBackup(
                "ce n'est pas un fichier de sauvegarde Kōgakushin".into(),
            ));
        }
        if backup.version != VERSION {
            return Err(AppError::InvalidBackup(format!(
                "version {} non prise en charge (attendue : {VERSION})",
                backup.version
            )));
        }
        Ok(backup)
    }
}

// Nombre d'éléments importés, affiché après l'import.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ImportReport {
    pub(super) books: usize,
    pub(super) lessons: usize,
    pub(super) items: usize,
    pub(super) cards: usize,
    pub(super) review_logs: usize,
    // Sauvegarde automatique de l'état d'avant l'import (D23).
    pub(super) safety_backup: String,
}
