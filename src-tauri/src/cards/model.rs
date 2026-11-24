// Structures de données des cartes, règle de génération et conversions vers rs_fsrs.

use chrono::{DateTime, Local, NaiveDateTime, Utc};

use crate::error::AppError;
use crate::items::ItemContent;

// Format des dates en base : celui de datetime('now') de SQLite, en UTC.
// Toujours le même, pour que la comparaison de textes `next_review_at <= ?` reste chronologique.
const DB_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub(super) fn to_db_date(date: DateTime<Utc>) -> String {
    date.format(DB_DATE_FORMAT).to_string()
}

fn from_db_date(text: &str, field: &'static str) -> Result<DateTime<Utc>, AppError> {
    NaiveDateTime::parse_from_str(text, DB_DATE_FORMAT)
        .map(|naive| naive.and_utc())
        .map_err(|_| AppError::InvalidData(field))
}

// Minuit (heure locale) du jour de `now`, exprimé en UTC : début du "aujourd'hui" de l'utilisatrice.
pub(super) fn start_of_local_day(now: DateTime<Utc>) -> DateTime<Utc> {
    now.with_timezone(&Local)
        .date_naive()
        .and_hms_opt(0, 0, 0)
        // earliest : en cas de changement d'heure, minuit peut être ambigu ou inexistant.
        .and_then(|midnight| midnight.and_local_timezone(Local).earliest())
        .map(|midnight| midnight.with_timezone(&Utc))
        .unwrap_or(now) // repli improbable : on compte alors à partir de maintenant
}

// Sens d'une carte (SPEC.md 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Direction {
    FrJp,      // français => japonais
    JpFr,      // japonais (kana) => français
    KanjiRead, // kanji => lecture + sens
}

impl Direction {
    // Tous les sens possibles (pour supprimer ceux qui ne sont plus voulus).
    pub(super) const ALL: [Direction; 3] = [Direction::FrJp, Direction::JpFr, Direction::KanjiRead];

    // Valeur de la colonne `direction` (doit correspondre au CHECK de la migration).
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Direction::FrJp => "fr_jp",
            Direction::JpFr => "jp_fr",
            Direction::KanjiRead => "kanji_read",
        }
    }

    // Inverse de as_str ; texte inconnu => None.
    fn from_db(value: &str) -> Option<Direction> {
        Direction::ALL.into_iter().find(|d| d.as_str() == value)
    }
}

// Cartes qu'un élément doit avoir : 2 par mot, 3 s'il a un kanji, aucune pour la grammaire.
// Même règle que le rattrapage SQL de la migration 20261122100000.
pub(super) fn directions_for(content: &ItemContent) -> Vec<Direction> {
    match content {
        ItemContent::Vocab { kanji, .. } => {
            let mut directions = vec![Direction::FrJp, Direction::JpFr];
            // is_some_and : Some avec une condition vraie ; un kanji vide ne compte pas.
            if kanji.as_deref().is_some_and(|k| !k.trim().is_empty()) {
                directions.push(Direction::KanjiRead);
            }
            directions
        }
        ItemContent::Grammar { .. } => Vec::new(),
    }
}

// --- État FSRS ---

// Colonnes FSRS d'une carte, telles que stockées.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct FsrsRow {
    next_review_at: String,
    stability: f64,
    difficulty: f64,
    elapsed_days: i64,
    scheduled_days: i64,
    reps: i64,
    lapses: i64,
    state: i64,
    last_review: Option<String>,
}

fn state_from_db(value: i64) -> Result<rs_fsrs::State, AppError> {
    match value {
        0 => Ok(rs_fsrs::State::New),
        1 => Ok(rs_fsrs::State::Learning),
        2 => Ok(rs_fsrs::State::Review),
        3 => Ok(rs_fsrs::State::Relearning),
        _ => Err(AppError::InvalidData("state")),
    }
}

// Valeur stockée pour un état (0 à 3, comme les discriminants de rs_fsrs::State).
pub(super) fn state_to_db(state: rs_fsrs::State) -> i64 {
    state as i64
}

impl FsrsRow {
    // Ligne => carte rs_fsrs prête à être planifiée.
    pub(super) fn to_fsrs(&self, now: DateTime<Utc>) -> Result<rs_fsrs::Card, AppError> {
        Ok(rs_fsrs::Card {
            due: from_db_date(&self.next_review_at, "next_review_at")?,
            stability: self.stability,
            difficulty: self.difficulty,
            elapsed_days: self.elapsed_days,
            scheduled_days: self.scheduled_days,
            reps: i32::try_from(self.reps).map_err(|_| AppError::InvalidData("reps"))?,
            lapses: i32::try_from(self.lapses).map_err(|_| AppError::InvalidData("lapses"))?,
            state: state_from_db(self.state)?,
            // Jamais révisée : rs_fsrs ignore cette date pour une carte nouvelle.
            last_review: match &self.last_review {
                Some(text) => from_db_date(text, "last_review")?,
                None => now,
            },
        })
    }
}

// Note donnée par l'utilisatrice. En JSON : "again", "hard", "good", "easy".
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewRating {
    Again, // À revoir
    Hard,  // Difficile
    Good,  // Bien
    Easy,  // Facile
}

impl ReviewRating {
    pub(super) fn to_fsrs(self) -> rs_fsrs::Rating {
        match self {
            ReviewRating::Again => rs_fsrs::Rating::Again,
            ReviewRating::Hard => rs_fsrs::Rating::Hard,
            ReviewRating::Good => rs_fsrs::Rating::Good,
            ReviewRating::Easy => rs_fsrs::Rating::Easy,
        }
    }
}

// --- Session de révision ---

// Cartes concernées par une session : une leçon, un manuel entier (D30), ou tout.
// Copy : deux Option<i64>, copiés sans coût à chaque requête.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SessionScope {
    pub(super) lesson_id: Option<i64>,
    pub(super) book_id: Option<i64>,
}

// Ligne brute : carte + champs utiles de son élément (toujours du vocabulaire).
#[derive(Debug, sqlx::FromRow)]
pub(super) struct SessionRow {
    card_id: i64,
    direction: String,
    state: i64,
    kana: Option<String>,
    romaji: Option<String>,
    kanji: Option<String>,
    meaning: Option<String>,
    example_jp: Option<String>,
    example_fr: Option<String>,
}

// Carte à réviser, envoyée au front : de quoi poser la question et montrer la réponse.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ReviewCard {
    card_id: i64,
    direction: Direction,
    is_new: bool,
    kana: String,
    romaji: Option<String>,
    kanji: Option<String>,
    meaning: String,
    example_jp: Option<String>,
    example_fr: Option<String>,
}

impl SessionRow {
    pub(super) fn into_review_card(self) -> Result<ReviewCard, AppError> {
        Ok(ReviewCard {
            card_id: self.card_id,
            direction: Direction::from_db(&self.direction)
                .ok_or(AppError::InvalidData("direction"))?,
            is_new: self.state == 0,
            kana: self.kana.ok_or(AppError::InvalidData("kana"))?,
            romaji: self.romaji,
            kanji: self.kanji,
            meaning: self.meaning.ok_or(AppError::InvalidData("meaning"))?,
            example_jp: self.example_jp,
            example_fr: self.example_fr,
        })
    }
}

// Session de révision envoyée au front.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ReviewSession {
    pub(super) cards: Vec<ReviewCard>,
    // Vrai s'il existe d'autres nouvelles cartes que celles proposées (limite du jour atteinte).
    pub(super) more_new_available: bool,
}

// Résultat d'une révision : date de prochaine révision (UTC, format de la base).
#[derive(Debug, serde::Serialize)]
pub(crate) struct ReviewResult {
    pub(super) next_review_at: String,
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};

    use super::{
        Direction, directions_for, from_db_date, start_of_local_day, state_from_db, state_to_db,
        to_db_date,
    };
    use crate::items::ItemContent;

    fn vocab(kanji: Option<&str>) -> ItemContent {
        ItemContent::Vocab {
            kana: "たべる".into(),
            romaji: None,
            kanji: kanji.map(String::from),
            meaning: "manger".into(),
            word_type: None,
        }
    }

    #[test]
    fn regle_de_generation() {
        use Direction::*;
        assert_eq!(directions_for(&vocab(None)), vec![FrJp, JpFr]);
        assert_eq!(directions_for(&vocab(Some("  "))), vec![FrJp, JpFr]);
        assert_eq!(
            directions_for(&vocab(Some("食べる"))),
            vec![FrJp, JpFr, KanjiRead]
        );
        let grammar = ItemContent::Grammar {
            title: "t".into(),
            explanation: "e".into(),
        };
        assert!(directions_for(&grammar).is_empty());
    }

    #[test]
    fn direction_as_str_et_from_db_sont_inverses() {
        for direction in Direction::ALL {
            assert_eq!(Direction::from_db(direction.as_str()), Some(direction));
        }
        assert_eq!(Direction::from_db("kanji"), None);
    }

    #[test]
    fn etat_aller_retour() {
        for value in 0..=3 {
            assert_eq!(state_from_db(value).map(state_to_db).ok(), Some(value));
        }
        assert!(state_from_db(4).is_err());
    }

    #[test]
    fn date_aller_retour_a_la_seconde() {
        let text = "2026-11-22 08:30:05";
        let date = from_db_date(text, "next_review_at");
        assert_eq!(date.map(to_db_date).ok().as_deref(), Some(text));
        assert!(from_db_date("2026-11-22T08:30:05Z", "next_review_at").is_err());
    }

    #[test]
    fn debut_de_journee_locale_dans_les_24_dernieres_heures() {
        let now = Utc::now();
        let start = start_of_local_day(now);
        assert!(start <= now);
        assert!(now - start < Duration::hours(24));
    }
}
