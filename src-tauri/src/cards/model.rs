// Structures de données des cartes, règle de génération et conversions vers rs_fsrs.

use chrono::{DateTime, Local, NaiveDateTime, Utc};

use crate::conjugation::{self, VerbForm, VerbType};
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
    // forme du dictionnaire => forme conjuguée (D41) ; la forme est dans la colonne `form`
    Conjugation,
}

impl Direction {
    // Tous les sens possibles.
    pub(super) const ALL: [Direction; 4] = [
        Direction::FrJp,
        Direction::JpFr,
        Direction::KanjiRead,
        Direction::Conjugation,
    ];

    // Valeur de la colonne `direction` (plus de CHECK en base depuis la migration 20261126000200 :
    // cet enum est la seule liste des valeurs permises).
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Direction::FrJp => "fr_jp",
            Direction::JpFr => "jp_fr",
            Direction::KanjiRead => "kanji_read",
            Direction::Conjugation => "conjugation",
        }
    }

    // Inverse de as_str ; texte inconnu => None.
    fn from_db(value: &str) -> Option<Direction> {
        Direction::ALL.into_iter().find(|d| d.as_str() == value)
    }
}

// Identité d'une carte pour un élément : son sens, et sa forme pour une carte de conjugaison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CardKey {
    pub(super) direction: Direction,
    pub(super) form: Option<VerbForm>,
}

impl CardKey {
    fn plain(direction: Direction) -> CardKey {
        CardKey {
            direction,
            form: None,
        }
    }

    pub(super) fn conjugation(form: VerbForm) -> CardKey {
        CardKey {
            direction: Direction::Conjugation,
            form: Some(form),
        }
    }

    // Colonnes direction + form => clé ; None si une valeur est inconnue ou incohérente.
    pub(super) fn from_db(direction: &str, form: Option<&str>) -> Option<CardKey> {
        let direction = Direction::from_db(direction)?;
        match (direction, form) {
            (Direction::Conjugation, Some(form)) => {
                VerbForm::from_db(form).map(CardKey::conjugation)
            }
            (Direction::Conjugation, None) | (_, Some(_)) => None,
            (direction, None) => Some(CardKey::plain(direction)),
        }
    }
}

// Cartes qu'un élément doit avoir : 2 par mot, 3 s'il a un kanji, aucune pour la grammaire,
// plus une carte de conjugaison par forme connue pour un verbe conjugable (D41).
// Même règle que les rattrapages SQL des migrations 20261122100000 et 20261126000200.
pub(super) fn wanted_cards(content: &ItemContent, known_forms: &[VerbForm]) -> Vec<CardKey> {
    match content {
        ItemContent::Vocab {
            kana,
            kanji,
            word_type,
            ..
        } => {
            let mut cards = vec![
                CardKey::plain(Direction::FrJp),
                CardKey::plain(Direction::JpFr),
            ];
            // is_some_and : Some avec une condition vraie ; un kanji vide ne compte pas.
            if kanji.as_deref().is_some_and(|k| !k.trim().is_empty()) {
                cards.push(CardKey::plain(Direction::KanjiRead));
            }
            // and_then : None si ce n'est pas un verbe. Un verbe dont la terminaison ne
            // correspond pas au type n'a pas de cartes de conjugaison (signalé dans Éléments).
            let verb_type = word_type.and_then(VerbType::from_word_type);
            if verb_type.is_some_and(|t| conjugation::conjugate(kana, None, t).is_ok()) {
                cards.extend(known_forms.iter().copied().map(CardKey::conjugation));
            }
            cards
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
    form: Option<String>,
    state: i64,
    word_type: Option<String>,
    kana: Option<String>,
    romaji: Option<String>,
    kanji: Option<String>,
    example_jp: Option<String>,
    example_fr: Option<String>,
    // Sens de l'élément (table item_meaning), en tableau JSON décodé par sqlx.
    #[sqlx(json)]
    meanings: Vec<String>,
}

// Question d'une carte de conjugaison, calculée par le code : "たべる - forme en て ?" => たべて.
// Pour la forme du dictionnaire, la question part de la forme en ます (たべます => たべる).
#[derive(Debug, serde::Serialize)]
pub(crate) struct ConjugationQuiz {
    form: VerbForm,
    question_kana: String,
    question_kanji: Option<String>,
    answer_kana: String,
    answer_kanji: Option<String>,
}

impl ConjugationQuiz {
    // None si le verbe ne se conjugue pas (type absent ou terminaison incompatible).
    fn build(
        kana: &str,
        kanji: Option<&str>,
        word_type: Option<&str>,
        form: VerbForm,
    ) -> Option<ConjugationQuiz> {
        let verb_type = word_type.and_then(VerbType::from_db)?;
        let forms = conjugation::conjugate(kana, kanji, verb_type).ok()?;
        let find = |wanted: VerbForm| forms.iter().find(|f| f.form == wanted);
        let question_form = if form == VerbForm::Dictionary {
            VerbForm::Masu
        } else {
            VerbForm::Dictionary
        };
        let question = find(question_form)?;
        let answer = find(form)?;
        Some(ConjugationQuiz {
            form,
            question_kana: question.kana.clone(),
            question_kanji: question.kanji.clone(),
            answer_kana: answer.kana.clone(),
            answer_kanji: answer.kanji.clone(),
        })
    }
}

// Carte à réviser, envoyée au front : de quoi poser la question et montrer la réponse.
#[derive(Debug, serde::Serialize)]
pub(crate) struct ReviewCard {
    card_id: i64,
    direction: Direction,
    // Rempli seulement pour direction = conjugation.
    conjugation: Option<ConjugationQuiz>,
    is_new: bool,
    kana: String,
    romaji: Option<String>,
    kanji: Option<String>,
    meanings: Vec<String>, // au moins un, dans l'ordre de saisie
    example_jp: Option<String>,
    example_fr: Option<String>,
}

impl SessionRow {
    // Ok(None) : carte de conjugaison d'un verbe qui ne se conjugue pas (ex. données importées
    // incohérentes) ; écartée de la session plutôt que de bloquer toutes les révisions.
    pub(super) fn into_review_card(self) -> Result<Option<ReviewCard>, AppError> {
        let key = CardKey::from_db(&self.direction, self.form.as_deref())
            .ok_or(AppError::InvalidData("direction"))?;
        let kana = self.kana.ok_or(AppError::InvalidData("kana"))?;
        // Une carte sans sens ne pourrait être ni posée (fr_jp) ni corrigée (jp_fr).
        if self.meanings.is_empty() {
            return Err(AppError::InvalidData("meanings"));
        }
        let conjugation = match key.form {
            Some(form) => {
                let quiz = ConjugationQuiz::build(
                    &kana,
                    self.kanji.as_deref(),
                    self.word_type.as_deref(),
                    form,
                );
                if quiz.is_none() {
                    log::warn!(
                        "Carte de conjugaison {} écartée : verbe non conjugable",
                        self.card_id
                    );
                    return Ok(None);
                }
                quiz
            }
            None => None,
        };
        Ok(Some(ReviewCard {
            card_id: self.card_id,
            direction: key.direction,
            conjugation,
            is_new: self.state == 0,
            kana,
            romaji: self.romaji,
            kanji: self.kanji,
            meanings: self.meanings,
            example_jp: self.example_jp,
            example_fr: self.example_fr,
        }))
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
        CardKey, ConjugationQuiz, Direction, from_db_date, start_of_local_day, state_from_db,
        state_to_db, to_db_date, wanted_cards,
    };
    use crate::conjugation::VerbForm;
    use crate::items::{ItemContent, WordType};

    fn vocab(kanji: Option<&str>) -> ItemContent {
        verb("たべる", kanji, None)
    }

    fn verb(kana: &str, kanji: Option<&str>, word_type: Option<WordType>) -> ItemContent {
        ItemContent::Vocab {
            kana: kana.into(),
            romaji: None,
            kanji: kanji.map(String::from),
            meanings: vec!["manger".into()],
            word_type,
        }
    }

    // Sens seuls (sans forme), pour comparer simplement.
    fn directions(content: &ItemContent, known: &[VerbForm]) -> Vec<Direction> {
        wanted_cards(content, known)
            .into_iter()
            .map(|k| k.direction)
            .collect()
    }

    #[test]
    fn regle_de_generation() {
        use Direction::*;
        let known = [VerbForm::Masu];
        assert_eq!(directions(&vocab(None), &known), vec![FrJp, JpFr]);
        assert_eq!(directions(&vocab(Some("  ")), &known), vec![FrJp, JpFr]);
        assert_eq!(
            directions(&vocab(Some("食べる")), &known),
            vec![FrJp, JpFr, KanjiRead]
        );
        let grammar = ItemContent::Grammar {
            title: "t".into(),
            explanation: "e".into(),
        };
        assert!(wanted_cards(&grammar, &known).is_empty());
    }

    #[test]
    fn une_carte_de_conjugaison_par_forme_connue() {
        let known = [VerbForm::Masu, VerbForm::Te];
        let ru = verb("たべる", None, Some(WordType::RuVerb));
        assert_eq!(
            wanted_cards(&ru, &known)[2..],
            [
                CardKey::conjugation(VerbForm::Masu),
                CardKey::conjugation(VerbForm::Te)
            ]
        );
        // Aucune forme connue, ou terminaison incompatible avec le type : pas de carte.
        assert_eq!(wanted_cards(&ru, &[]).len(), 2);
        let broken = verb("たべ", None, Some(WordType::RuVerb));
        assert_eq!(wanted_cards(&broken, &known).len(), 2);
        // Un nom n'a pas de conjugaison.
        let noun = verb("ほん", None, Some(WordType::Noun));
        assert_eq!(wanted_cards(&noun, &known).len(), 2);
    }

    #[test]
    fn cle_de_carte_depuis_la_base() {
        assert_eq!(
            CardKey::from_db("conjugation", Some("te")),
            Some(CardKey::conjugation(VerbForm::Te))
        );
        assert_eq!(
            CardKey::from_db("fr_jp", None).map(|k| k.direction),
            Some(Direction::FrJp)
        );
        // Incohérences : forme manquante, forme sur une carte ordinaire, valeurs inconnues.
        assert_eq!(CardKey::from_db("conjugation", None), None);
        assert_eq!(CardKey::from_db("fr_jp", Some("te")), None);
        assert_eq!(CardKey::from_db("conjugation", Some("volitive")), None);
        assert_eq!(CardKey::from_db("kanji", None), None);
    }

    #[test]
    fn question_de_conjugaison() {
        let te = ConjugationQuiz::build("たべる", Some("食べる"), Some("ru_verb"), VerbForm::Te)
            .unwrap();
        assert_eq!(
            (te.question_kana.as_str(), te.answer_kana.as_str()),
            ("たべる", "たべて")
        );
        assert_eq!(te.answer_kanji.as_deref(), Some("食べて"));
        // Forme du dictionnaire : la question part de la forme en ます.
        let dico =
            ConjugationQuiz::build("いく", None, Some("u_verb"), VerbForm::Dictionary).unwrap();
        assert_eq!(
            (dico.question_kana.as_str(), dico.answer_kana.as_str()),
            ("いきます", "いく")
        );
        assert!(ConjugationQuiz::build("たべる", None, Some("noun"), VerbForm::Te).is_none());
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
