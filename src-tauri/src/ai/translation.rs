// Exercice de traduction (SPEC.md 3.4) : l'IA crée une phrase avec mes connaissances,
// le code vérifie qu'elle n'utilise rien d'inconnu (garde-fou), puis l'IA corrige ma traduction.

use std::collections::HashSet;

use serde_json::json;
use sqlx::SqlitePool;
use tauri::State;

use super::client::{ChatRequest, Ollama};
use crate::error::AppError;
use crate::items::{self, Knowledge};
use crate::settings;

// Essais de génération avant d'accepter une phrase avec des mots inconnus (signalés).
const MAX_ATTEMPTS: usize = 3;
// Phrases récentes transmises à l'IA pour qu'elle ne se répète pas.
const MAX_PREVIOUS: usize = 5;
// Longueur maximale d'une explication de grammaire dans les consignes (prompt plus court).
const MAX_EXPLANATION_CHARS: usize = 200;

// Mots grammaticaux jamais signalés comme inconnus (particules, copule, terminaisons polies).
const FUNCTION_WORDS: [&str; 18] = [
    "は",
    "が",
    "を",
    "に",
    "へ",
    "で",
    "と",
    "も",
    "の",
    "か",
    "よ",
    "ね",
    "から",
    "まで",
    "です",
    "だ",
    "ます",
    "ません",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TranslationDirection {
    FrJp, // je traduis du français vers le japonais
    JpFr, // je traduis du japonais vers le français
}

// Réponse attendue de l'IA à la génération.
#[derive(Debug, serde::Deserialize)]
struct GeneratedSentence {
    japanese: String,
    french: String,
    // Mots porteurs de sens à leur forme du dictionnaire : sert au garde-fou.
    words: Vec<String>,
}

// Exercice envoyé au front, puis renvoyé tel quel pour la correction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TranslationExercise {
    direction: TranslationDirection,
    japanese: String,
    french: String,
    // Mots ou kanji hors de ma base, repérés par le code (vide si tout est connu).
    unknown_words: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Verdict {
    Correct,
    Almost, // presque : petite erreur
    Wrong,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Correction {
    verdict: Verdict,
    correction: String,
    explanation: String,
}

// --- Garde-fou ---

// Idéogramme CJK (kanji) : blocs Unicode principal et extension A.
fn is_kanji(c: char) -> bool {
    ('\u{4E00}'..='\u{9FFF}').contains(&c) || ('\u{3400}'..='\u{4DBF}').contains(&c)
}

// Mots et kanji de la phrase absents de ma base.
// - mots : ceux que l'IA déclare avoir utilisés (forme du dictionnaire) ;
// - kanji : vérifiés caractère par caractère dans la phrase elle-même (fiable, sans l'IA).
fn unknown_words(sentence: &GeneratedSentence, knowledge: &Knowledge) -> Vec<String> {
    let mut known: HashSet<&str> = FUNCTION_WORDS.into_iter().collect();
    let mut known_kanji: HashSet<char> = HashSet::new();
    for word in &knowledge.words {
        known.insert(word.kana.trim());
        if let Some(kanji) = &word.kanji {
            known.insert(kanji.trim());
            known_kanji.extend(kanji.chars().filter(|&c| is_kanji(c)));
        }
    }

    let mut unknown: Vec<String> = Vec::new();
    for word in &sentence.words {
        let word = word.trim();
        if !word.is_empty() && !known.contains(word) && !unknown.iter().any(|u| u == word) {
            unknown.push(word.to_string());
        }
    }
    for c in sentence.japanese.chars().filter(|&c| is_kanji(c)) {
        let kanji = c.to_string();
        if !known_kanji.contains(&c) && !unknown.contains(&kanji) {
            unknown.push(kanji);
        }
    }
    unknown
}

// --- Consignes envoyées à l'IA ---

fn truncate(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((byte_index, _)) => format!("{}...", &text[..byte_index]),
        None => text.to_string(),
    }
}

// Règle d'écriture commune à la génération et à la correction (décisions D9 et D10).
fn writing_rule(allowed_kanji: &str) -> String {
    let kanji = if allowed_kanji.is_empty() {
        "N'utilise AUCUN kanji.".to_string()
    } else {
        format!("Kanji autorisés UNIQUEMENT parmi : {allowed_kanji} (aucun autre kanji).")
    };
    format!(
        "Écriture : hiragana et katakana. {kanji} \
         Mets une espace entre chaque mot, et entre un mot et sa particule \
         (exemple : わたし は がっこう に いきます)."
    )
}

fn allowed_kanji(knowledge: &Knowledge) -> String {
    let mut seen: Vec<char> = Vec::new();
    for word in &knowledge.words {
        for c in word
            .kanji
            .iter()
            .flat_map(|k| k.chars())
            .filter(|&c| is_kanji(c))
        {
            if !seen.contains(&c) {
                seen.push(c);
            }
        }
    }
    seen.into_iter().collect()
}

fn generation_system_prompt(knowledge: &Knowledge) -> String {
    let vocabulary: Vec<String> = knowledge
        .words
        .iter()
        .map(|w| match &w.kanji {
            Some(kanji) => format!("- {} ({kanji}) : {}", w.kana, w.meaning),
            None => format!("- {} : {}", w.kana, w.meaning),
        })
        .collect();
    let grammar: Vec<String> = knowledge
        .grammar
        .iter()
        .map(|g| {
            format!(
                "- {} : {}",
                g.title,
                truncate(&g.explanation, MAX_EXPLANATION_CHARS)
            )
        })
        .collect();
    let grammar = if grammar.is_empty() {
        "(aucun point saisi)".to_string()
    } else {
        grammar.join("\n")
    };

    format!(
        "Tu es professeur de japonais pour une débutante francophone. \
         Tu crées UNE phrase japonaise courte (3 à 8 mots), naturelle, et sa traduction française.\n\
         Règles strictes :\n\
         1. Vocabulaire : UNIQUEMENT les mots de la liste ci-dessous (conjugués si besoin), \
         plus les particules.\n\
         2. Grammaire : UNIQUEMENT les points de la liste ci-dessous, plus les particules de base \
         et la forme polie です / ます.\n\
         3. {} Écris avec leur kanji les mots de la liste qui en ont un.\n\
         4. Dans `words`, liste les noms, verbes, adjectifs et adverbes de ta phrase, à leur forme \
         du dictionnaire, écrits exactement comme dans la liste de vocabulaire \
         (sans les particules ni です / ます).\n\n\
         Vocabulaire :\n{}\n\nGrammaire :\n{grammar}",
        writing_rule(&allowed_kanji(knowledge)),
        vocabulary.join("\n"),
    )
}

fn generation_user_prompt(
    knowledge: &Knowledge,
    previous: &[String],
    rejected: &[String],
) -> String {
    let mut prompt = String::from("Crée une nouvelle phrase.");
    // Rappel explicite, mot par mot : une consigne générale sur les kanji est ignorée par le modèle.
    let with_kanji: Vec<String> = knowledge
        .words
        .iter()
        .filter_map(|w| w.kanji.as_ref().map(|k| format!("{k} (pas {})", w.kana)))
        .collect();
    if !with_kanji.is_empty() {
        prompt.push_str(&format!(
            " Écris obligatoirement : {}.",
            with_kanji.join(", ")
        ));
    }
    if !knowledge.focus.is_empty() {
        let focus: Vec<&str> = knowledge.focus.iter().map(|w| w.kana.as_str()).collect();
        prompt.push_str(&format!(" Utilise de préférence : {}.", focus.join(", ")));
    }
    if !previous.is_empty() {
        prompt.push_str(&format!(
            " Ne reprends pas ces phrases déjà proposées : {}.",
            previous.join(" / ")
        ));
    }
    if !rejected.is_empty() {
        prompt.push_str(&format!(
            " Ta phrase précédente utilisait des mots interdits ({}) : n'utilise que la liste.",
            rejected.join(", ")
        ));
    }
    prompt
}

fn generation_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "japanese": { "type": "string" },
            "french": { "type": "string" },
            "words": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["japanese", "french", "words"]
    })
}

fn correction_system_prompt(exercise: &TranslationExercise) -> String {
    let (target, rule) = match exercise.direction {
        TranslationDirection::FrJp => (
            "en japonais",
            // Kanji de la phrase de référence seulement : ce sont des kanji que je connais.
            writing_rule(
                &exercise
                    .japanese
                    .chars()
                    .filter(|&c| is_kanji(c))
                    .collect::<String>(),
            ),
        ),
        TranslationDirection::JpFr => ("en français", String::new()),
    };
    format!(
        "Tu es professeur de japonais, bienveillant et précis, pour une débutante francophone. \
         Tu corriges sa traduction.\n\
         - verdict : \"correct\" si le sens et la grammaire sont justes, même si la formulation \
         diffère de la référence ; \"almost\" pour une petite erreur (particule, conjugaison, \
         orthographe) ; \"wrong\" sinon.\n\
         - correction : la meilleure traduction {target}, proche de celle de l'élève si elle est \
         juste. {rule}\n\
         - explanation : 1 à 3 phrases courtes en français ; si c'est correct, dis brièvement \
         pourquoi."
    )
}

fn correction_user_prompt(exercise: &TranslationExercise, answer: &str) -> String {
    match exercise.direction {
        TranslationDirection::FrJp => format!(
            "Phrase à traduire (français) : {}\nTraduction de référence : {}\n\
             Traduction de l'élève : {answer}",
            exercise.french, exercise.japanese
        ),
        TranslationDirection::JpFr => format!(
            "Phrase à traduire (japonais) : {}\nTraduction de référence : {}\n\
             Traduction de l'élève : {answer}",
            exercise.japanese, exercise.french
        ),
    }
}

fn correction_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "verdict": { "type": "string", "enum": ["correct", "almost", "wrong"] },
            "correction": { "type": "string" },
            "explanation": { "type": "string" }
        },
        "required": ["verdict", "correction", "explanation"]
    })
}

// --- Logique (sans Tauri, testable) ---

async fn generate_with(
    ollama: &Ollama,
    model: &str,
    knowledge: &Knowledge,
    direction: TranslationDirection,
    previous: &[String],
) -> Result<TranslationExercise, AppError> {
    let system = generation_system_prompt(knowledge);
    let mut rejected: Vec<String> = Vec::new();
    // Meilleure phrase obtenue (le moins de mots inconnus), gardée si aucun essai n'est parfait.
    let mut best: Option<TranslationExercise> = None;

    for _ in 0..MAX_ATTEMPTS {
        let user = generation_user_prompt(knowledge, previous, &rejected);
        let sentence: GeneratedSentence = ollama
            .chat_json(ChatRequest {
                model,
                system: &system,
                user: &user,
                schema: generation_schema(),
                temperature: 0.8, // de la variété d'une phrase à l'autre
            })
            .await?;
        let unknown = unknown_words(&sentence, knowledge);
        let exercise = TranslationExercise {
            direction,
            japanese: sentence.japanese.trim().to_string(),
            french: sentence.french.trim().to_string(),
            unknown_words: unknown.clone(),
        };
        if unknown.is_empty() {
            return Ok(exercise);
        }
        log::info!(
            "Phrase rejetée (mots inconnus : {unknown:?}) : {}",
            exercise.japanese
        );
        let is_better = best
            .as_ref()
            .is_none_or(|b| exercise.unknown_words.len() < b.unknown_words.len());
        if is_better {
            best = Some(exercise);
        }
        rejected = unknown;
    }
    best.ok_or(AppError::InvalidAiResponse)
}

async fn correct_with(
    ollama: &Ollama,
    model: &str,
    exercise: &TranslationExercise,
    answer: &str,
) -> Result<Correction, AppError> {
    ollama
        .chat_json(ChatRequest {
            model,
            system: &correction_system_prompt(exercise),
            user: &correction_user_prompt(exercise, answer),
            schema: correction_schema(),
            temperature: 0.2, // correction : on veut de la constance
        })
        .await
}

// --- Commandes Tauri ---

// JS : invoke("generate_translation", { lessonIds: [1, 2], direction: "fr_jp", previous: [...] }).
// lessonIds vide ou absent = toutes les leçons.
#[tauri::command]
pub(crate) async fn generate_translation(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    lesson_ids: Option<Vec<i64>>,
    direction: TranslationDirection,
    previous: Option<Vec<String>>,
) -> Result<TranslationExercise, AppError> {
    let knowledge = items::find_knowledge(pool.inner(), &lesson_ids.unwrap_or_default()).await?;
    if knowledge.words.is_empty() {
        return Err(AppError::Validation(
            "Aucun mot de vocabulaire actif dans ces leçons.",
        ));
    }
    let model = settings::load(pool.inner()).await?.ollama_model;

    // Seules les phrases les plus récentes sont transmises (consignes plus courtes).
    let mut previous = previous.unwrap_or_default();
    let skip = previous.len().saturating_sub(MAX_PREVIOUS);
    previous.drain(..skip);

    generate_with(ollama.inner(), &model, &knowledge, direction, &previous).await
}

#[tauri::command]
pub(crate) async fn correct_translation(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    exercise: TranslationExercise,
    answer: String,
) -> Result<Correction, AppError> {
    if answer.trim().is_empty() {
        return Err(AppError::Validation(
            "Écris ta traduction avant de la faire corriger.",
        ));
    }
    let model = settings::load(pool.inner()).await?.ollama_model;
    correct_with(ollama.inner(), &model, &exercise, answer.trim()).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::{KnownGrammar, KnownWord};

    fn word(kana: &str, kanji: Option<&str>, meaning: &str) -> KnownWord {
        KnownWord {
            kana: kana.into(),
            kanji: kanji.map(String::from),
            meaning: meaning.into(),
        }
    }

    fn knowledge() -> Knowledge {
        Knowledge {
            words: vec![
                word("わたし", Some("私"), "je"),
                word("たべる", Some("食べる"), "manger"),
                word("りんご", None, "pomme"),
                word("がっこう", None, "école"),
                word("いく", None, "aller"),
            ],
            grammar: vec![KnownGrammar {
                title: "は".into(),
                explanation: "particule du thème".into(),
            }],
            focus: vec![word("りんご", None, "pomme")],
        }
    }

    fn sentence(japanese: &str, words: &[&str]) -> GeneratedSentence {
        GeneratedSentence {
            japanese: japanese.into(),
            french: "x".into(),
            words: words.iter().map(|w| w.to_string()).collect(),
        }
    }

    #[test]
    fn phrase_entierement_connue() {
        let s = sentence(
            "私 は りんご を 食べます",
            &["わたし", "りんご", "たべる", "です"],
        );
        assert!(unknown_words(&s, &knowledge()).is_empty());
    }

    #[test]
    fn mot_et_kanji_inconnus_signales_une_seule_fois() {
        let s = sentence(
            "私 は 水 を のみます 水",
            &["わたし", "みず", "のむ", "のむ"],
        );
        assert_eq!(unknown_words(&s, &knowledge()), ["みず", "のむ", "水"]);
    }

    #[test]
    fn consignes_contiennent_vocabulaire_kanji_et_regle_d_espaces() {
        let prompt = generation_system_prompt(&knowledge());
        assert!(prompt.contains("- たべる (食べる) : manger"));
        assert!(prompt.contains("UNIQUEMENT parmi : 私食"));
        assert!(prompt.contains("わたし は がっこう に いきます"));
        let no_kanji = Knowledge {
            words: vec![word("いく", None, "aller")],
            ..Knowledge::default()
        };
        assert!(generation_system_prompt(&no_kanji).contains("N'utilise AUCUN kanji."));
    }

    #[test]
    fn troncature_par_caracteres_et_non_par_octets() {
        assert_eq!(truncate("たべる", 2), "たべ...");
        assert_eq!(truncate("たべる", 3), "たべる");
    }

    // Nécessite Ollama lancé avec le modèle par défaut :
    // cargo test -- --ignored essai_reel_traduction --nocapture
    #[test]
    #[ignore]
    fn essai_reel_traduction() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = "gemma4:e4b-it-q4_K_M";
            let exercise = generate_with(
                &ollama,
                model,
                &knowledge(),
                TranslationDirection::FrJp,
                &[],
            )
            .await
            .unwrap();
            println!("exercice : {exercise:?}");
            let correction =
                correct_with(&ollama, model, &exercise, "わたし は りんご を たべます")
                    .await
                    .unwrap();
            println!("correction : {correction:?}");
        });
    }
}
