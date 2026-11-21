// Structures de données des leçons (équivalent des entités, sans ORM).

use crate::error::AppError;

// Structs en pub(crate) : le code généré par #[tauri::command] les manipule hors de `lessons`.

// Leçon lue en base et renvoyée au front.
// Champs privés : les derive génèrent leur code ici, ils y ont donc accès.
#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct Lesson {
    id: i64,
    title: String,
    date: String,
    notes: Option<String>, // NULL en base => None
    created_at: String,
}

// Données envoyées par le front pour créer ou modifier une leçon.
#[derive(serde::Deserialize)]
pub(crate) struct CreateLessonInput {
    // pub(super) : lus par commands.rs, invisibles hors de `lessons`.
    pub(super) title: String,
    pub(super) date: String,
    pub(super) notes: Option<String>,
}

impl CreateLessonInput {
    // Refuse un titre ou une date vides (espaces seuls compris).
    pub(super) fn validate(&self) -> Result<(), AppError> {
        if self.title.trim().is_empty() {
            return Err(AppError::Validation(
                "Le titre de la leçon ne peut pas être vide",
            ));
        }
        if !is_iso_date(&self.date) {
            return Err(AppError::Validation(
                "La date de la leçon doit être une date réelle au format AAAA-MM-JJ",
            ));
        }
        Ok(())
    }
}

// Vrai si `date` est une date existante écrite exactement en AAAA-MM-JJ.
// Ce format strict garantit que le tri texte `ORDER BY date` est chronologique.
fn is_iso_date(date: &str) -> bool {
    match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        // chrono accepte "2026-1-5" : on exige que la date réécrite soit identique.
        Ok(parsed) => parsed.format("%Y-%m-%d").to_string() == date,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_iso_date;

    #[test]
    fn accepte_une_date_iso_reelle() {
        assert!(is_iso_date("2026-11-18"));
        assert!(is_iso_date("2028-02-29")); // année bissextile
    }

    #[test]
    fn refuse_les_dates_invalides() {
        for date in [
            "",
            "   ",
            "abc",
            "2026-1-5",
            "2026-02-30",
            "2026-13-01",
            "18/11/2026",
            " 2026-11-18",
            "2026-11-18 ",
            "2027-02-29",
        ] {
            assert!(!is_iso_date(date), "acceptée à tort : {date:?}");
        }
    }
}
