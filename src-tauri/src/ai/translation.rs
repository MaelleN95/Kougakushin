// Exercice de traduction (SPEC.md 3.4) : l'IA crée une phrase avec mes connaissances,
// le code vérifie qu'elle n'utilise rien d'inconnu (garde-fou), puis l'IA corrige ma traduction.

use serde_json::json;
use sqlx::SqlitePool;
use tauri::State;

use super::client::{ChatRequest, ChatTurn, Ollama};
use super::knowledge_prompt::{
    is_kanji, kanji_reminder, knowledge_rules, normalize_japanese, pick_index, unknown_words,
    writing_rule,
};
use super::to_french::to_french;
use crate::error::AppError;
use crate::items::{self, Knowledge};
use crate::settings;

// Essais de génération avant d'accepter une phrase avec des mots inconnus (signalés).
const MAX_ATTEMPTS: usize = 3;
// Phrases récentes transmises à l'IA pour qu'elle ne se répète pas.
const MAX_PREVIOUS: usize = 5;

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

// --- Consignes envoyées à l'IA ---

fn generation_system_prompt(knowledge: &Knowledge) -> String {
    format!(
        "Tu es un professeur de japonais expérimenté, de langue maternelle japonaise, exigeant \
         sur le naturel. Ton élève est une francophone qui apprend en cours. \
         Tu crées UNE phrase japonaise (sa traduction française est faite à part).\n\
         Qualité attendue :\n\
         - La phrase doit sonner naturelle pour un Japonais, comme dans une vraie conversation. \
         Omets le sujet quand il est évident, comme le font les Japonais : n'écris わたし は que \
         si la phrase en a vraiment besoin (contraste, présentation).\n\
         - Exploite au mieux ses connaissances : combine plusieurs mots et points de grammaire, \
         4 à 10 mots, pas seulement 'A は B を V ます'.\n{}",
        knowledge_rules(knowledge)
    )
}

// Forme de phrase imposée à chaque demande : sans elle, le modèle produit toujours
// la même affirmation simple.
const SENTENCE_STYLES: [&str; 6] = [
    "une question (avec か)",
    "une phrase négative",
    "une phrase qui précise un moment (aujourd'hui, demain...) si le vocabulaire le permet",
    "une phrase qui précise un lieu ou une destination",
    "une réponse naturelle à une question qu'on t'aurait posée",
    "une phrase qui relie deux idées",
];

fn pick_style() -> &'static str {
    SENTENCE_STYLES[pick_index(SENTENCE_STYLES.len())]
}

fn generation_user_prompt(
    knowledge: &Knowledge,
    previous: &[String],
    rejected: &[String],
) -> String {
    let mut prompt = format!(
        "Crée une nouvelle phrase : {}, si sa grammaire est connue (sinon, une autre forme).",
        pick_style()
    );
    if let Some(reminder) = kanji_reminder(knowledge) {
        prompt.push(' ');
        prompt.push_str(&reminder);
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
            "words": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["japanese", "words"]
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
    // Vers le japonais : la phrase française est une traduction automatique de la référence,
    // parfois inexacte. La référence fait donc foi, et ses mots sont ceux que l'élève connaît.
    let reference_rule = match exercise.direction {
        TranslationDirection::FrJp => {
            " La traduction de référence est la réponse attendue et fait foi si la phrase \
             française semble dire autre chose. N'utilise QUE des mots présents dans la \
             référence ou dans la traduction de l'élève : n'en ajoute aucun autre."
        }
        TranslationDirection::JpFr => "",
    };
    format!(
        "Tu es un professeur de japonais expérimenté, de langue maternelle japonaise. \
         Tu corriges la traduction d'une élève francophone, avec l'exigence d'un bon professeur.\n\
         Juge comme un Japonais :\n\
         - Une traduction qui omet un sujet évident est correcte : c'est même plus naturel. \
         Plusieurs formulations justes existent : ne pénalise pas un écart avec la référence \
         s'il est correct et naturel.\n\
         - Vérifie précisément : particules, conjugaison (temps, négation, forme polie), \
         ordre des mots, choix du mot, sens exact (rien d'ajouté, rien d'oublié).\n\
         Réponds :\n\
         - analysis : D'ABORD, compare mot par mot la traduction de l'élève à la phrase à \
         traduire. Recopie chaque mot de l'élève et dis s'il est juste. Ne signale un oubli que \
         si le mot est réellement absent de la traduction de l'élève.\n\
         - verdict : \"correct\" si c'est juste et naturel ; \"almost\" si une petite erreur \
         ou une formulation juste mais peu naturelle ; \"wrong\" si le sens ou la grammaire \
         est faux.\n\
         - correction : la meilleure traduction {target}, au plus près de celle de l'élève \
         quand c'est possible. {rule}{reference_rule}\n\
         - words : les mots porteurs de sens de ta correction, à leur forme du dictionnaire.\n\
         - explanation : en français, 1 à 3 phrases précises qui nomment la notion en jeu \
         (ex. 'particule に de destination', 'négation polie 〜ません'). Pas de compliments \
         vagues. Si c'est correct, signale une nuance ou une autre formulation naturelle utile."
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
        // `analysis` en premier : le modèle écrit dans l'ordre du schéma, il analyse donc
        // avant de décider. Ce brouillon n'est pas montré (serde l'ignore à la lecture).
        "properties": {
            "analysis": { "type": "string" },
            "verdict": { "type": "string", "enum": ["correct", "almost", "wrong"] },
            "correction": { "type": "string" },
            // Mots porteurs de sens de la correction, forme du dictionnaire : contrôlés par le code.
            "words": { "type": "array", "items": { "type": "string" } },
            "explanation": { "type": "string" }
        },
        "required": ["analysis", "verdict", "correction", "words", "explanation"]
    })
}

// --- Logique (sans Tauri, testable) ---

// Modèles utilisés : `model` invente la phrase, `translator` la traduit en français (D22).
struct Models<'a> {
    model: &'a str,
    translator: &'a str,
}

async fn generate_with(
    ollama: &Ollama,
    models: &Models<'_>,
    knowledge: &Knowledge,
    direction: TranslationDirection,
    previous: &[String],
) -> Result<TranslationExercise, AppError> {
    let japanese_sentence = generate_japanese(ollama, models.model, knowledge, previous).await?;
    // Traduction par le modèle dédié : la version française correspond à la phrase réellement
    // générée (le modèle principal se trompait parfois de phrase à traduire).
    // La phrase est d'abord réécrite en kanji : voir to_french.rs.
    let french = to_french(
        ollama,
        models.model,
        models.translator,
        &japanese_sentence.0,
    )
    .await?;
    Ok(TranslationExercise {
        direction,
        japanese: japanese_sentence.0,
        french,
        unknown_words: japanese_sentence.1,
    })
}

// Phrase japonaise + ses mots inconnus : la première sans intrus, sinon la meilleure des essais.
async fn generate_japanese(
    ollama: &Ollama,
    model: &str,
    knowledge: &Knowledge,
    previous: &[String],
) -> Result<(String, Vec<String>), AppError> {
    let system = generation_system_prompt(knowledge);
    let mut rejected: Vec<String> = Vec::new();
    let mut best: Option<(String, Vec<String>)> = None;

    for _ in 0..MAX_ATTEMPTS {
        let user = generation_user_prompt(knowledge, previous, &rejected);
        let result: Result<GeneratedSentence, AppError> = ollama
            .chat_json(ChatRequest {
                model,
                system: &system,
                turns: &[ChatTurn::user(user)],
                schema: generation_schema(),
                temperature: 0.8, // de la variété d'une phrase à l'autre
            })
            .await;
        // JSON illisible : on retente (essai suivant) au lieu d'échouer tout de suite.
        let sentence = match result {
            Err(AppError::InvalidAiResponse) => continue,
            other => other?,
        };
        let unknown = unknown_words(&sentence.japanese, &sentence.words, knowledge);
        let japanese = sentence.japanese.trim().to_string();
        if unknown.is_empty() {
            return Ok((japanese, unknown));
        }
        log::info!("Phrase rejetée (mots inconnus : {unknown:?}) : {japanese}");
        let is_better = best.as_ref().is_none_or(|b| unknown.len() < b.1.len());
        if is_better {
            best = Some((japanese, unknown.clone()));
        }
        rejected = unknown;
    }
    best.ok_or(AppError::InvalidAiResponse)
}

// Réponse brute de l'IA : `words` (mots de sa correction) sert seulement au contrôle.
#[derive(serde::Deserialize)]
struct RawCorrection {
    verdict: Verdict,
    correction: String,
    explanation: String,
    words: Vec<String>,
}

// Mots de la correction japonaise absents de ma base, hors mots déjà présents dans la
// référence ou dans ma réponse (ceux-là ne sont pas inventés par le correcteur).
fn invented_words(
    raw: &RawCorrection,
    exercise: &TranslationExercise,
    answer: &str,
    knowledge: &Knowledge,
) -> Vec<String> {
    unknown_words(&raw.correction, &raw.words, knowledge)
        .into_iter()
        .filter(|w| !exercise.japanese.contains(w.as_str()) && !answer.contains(w.as_str()))
        .collect()
}

async fn correct_with(
    ollama: &Ollama,
    model: &str,
    exercise: &TranslationExercise,
    answer: &str,
    knowledge: &Knowledge,
) -> Result<Correction, AppError> {
    let mut user = correction_user_prompt(exercise, answer);
    let mut last: Option<RawCorrection> = None;
    // Deux essais : le second rappelle les mots interdits utilisés par le premier.
    for _ in 0..2 {
        let raw: RawCorrection = ollama
            .chat_json(ChatRequest {
                model,
                system: &correction_system_prompt(exercise),
                turns: &[ChatTurn::user(user.clone())],
                schema: correction_schema(),
                temperature: 0.2, // correction : on veut de la constance
            })
            .await?;
        // Vers le français : pas de mots japonais à contrôler.
        let invented = match exercise.direction {
            TranslationDirection::FrJp => invented_words(&raw, exercise, answer, knowledge),
            TranslationDirection::JpFr => Vec::new(),
        };
        if invented.is_empty() {
            return Ok(Correction {
                verdict: raw.verdict,
                correction: raw.correction,
                explanation: raw.explanation,
            });
        }
        log::info!(
            "Correction rejetée (mots hors base : {invented:?}) : {}",
            raw.correction
        );
        user.push_str(&format!(
            "\nTa correction précédente utilisait des mots hors de ses connaissances ({}) : \
             corrige uniquement avec les mots de la référence et de sa traduction.",
            invented.join(", ")
        ));
        last = Some(raw);
    }
    // Toujours des mots inventés : la phrase de référence sert de correction.
    let raw = last.ok_or(AppError::InvalidAiResponse)?;
    Ok(Correction {
        verdict: raw.verdict,
        correction: exercise.japanese.clone(),
        explanation: format!(
            "{} (Correction remplacée par la phrase de référence : la correction de l'IA \
             utilisait des mots hors de ta base.)",
            raw.explanation
        ),
    })
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
    let settings = settings::load(pool.inner()).await?;
    let models = Models {
        model: &settings.ollama_model,
        translator: &settings.translation_model,
    };

    // Seules les phrases les plus récentes sont transmises (consignes plus courtes).
    let mut previous = previous.unwrap_or_default();
    let skip = previous.len().saturating_sub(MAX_PREVIOUS);
    previous.drain(..skip);

    generate_with(ollama.inner(), &models, &knowledge, direction, &previous).await
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
    let knowledge = items::find_knowledge(pool.inner(), &[]).await?; // toute ma base
    let mut correction =
        correct_with(ollama.inner(), &model, &exercise, answer.trim(), &knowledge).await?;

    // Garde-fou : l'IA juge parfois fausse une réponse identique à sa propre correction
    // (espaces, ponctuation ou kanji / kana mis à part). Le code la remet à "correct".
    if exercise.direction == TranslationDirection::FrJp
        && correction.verdict != Verdict::Correct
        && normalize_japanese(answer.trim(), &knowledge)
            == normalize_japanese(&correction.correction, &knowledge)
    {
        log::info!("Verdict corrigé en 'correct' (réponse identique) : {answer}");
        correction.verdict = Verdict::Correct;
        correction.explanation =
            "Ta traduction est correcte (elle correspond à la correction proposée).".into();
    }
    Ok(correction)
}

#[cfg(test)]
mod tests {
    use super::super::knowledge_prompt::truncate;
    use super::*;
    use crate::items::{KnownGrammar, KnownWord};

    fn word(kana: &str, kanji: Option<&str>, meaning: &str) -> KnownWord {
        KnownWord {
            kana: kana.into(),
            kanji: kanji.map(String::from),
            meanings: vec![meaning.into()],
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
            words: words.iter().map(|w| w.to_string()).collect(),
        }
    }

    #[test]
    fn phrase_entierement_connue() {
        let s = sentence(
            "私 は りんご を 食べます",
            &["わたし", "りんご", "たべる", "です"],
        );
        assert!(unknown_words(&s.japanese, &s.words, &knowledge()).is_empty());
    }

    #[test]
    fn mot_et_kanji_inconnus_signales_une_seule_fois() {
        let s = sentence(
            "私 は 水 を のみます 水",
            &["わたし", "みず", "のむ", "のむ"],
        );
        assert_eq!(
            unknown_words(&s.japanese, &s.words, &knowledge()),
            ["みず", "のむ", "水"]
        );
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

    // Modèles des essais réels : le modèle testé + le traducteur par défaut.
    fn test_models(model: &str) -> Models<'_> {
        Models {
            model,
            translator: "translategemma:4b",
        }
    }

    // Vocabulaire réaliste de débutante, pour juger le niveau des phrases.
    fn realistic_knowledge() -> Knowledge {
        let words = [
            ("わたし", Some("私"), "je"),
            ("ともだち", None, "ami"),
            ("せんせい", Some("先生"), "professeur"),
            ("がっこう", Some("学校"), "école"),
            ("うち", None, "maison"),
            ("えき", Some("駅"), "gare"),
            ("みず", Some("水"), "eau"),
            ("おちゃ", None, "thé"),
            ("パン", None, "pain"),
            ("すし", None, "sushi"),
            ("ほん", Some("本"), "livre"),
            ("えいが", None, "film"),
            ("たべる", Some("食べる"), "manger"),
            ("のむ", Some("飲む"), "boire"),
            ("いく", Some("行く"), "aller"),
            ("くる", Some("来る"), "venir"),
            ("みる", Some("見る"), "regarder"),
            ("よむ", Some("読む"), "lire"),
            ("おいしい", None, "bon, délicieux"),
            ("おおきい", None, "grand"),
            ("きょう", None, "aujourd'hui"),
            ("あした", None, "demain"),
            ("いっしょに", None, "ensemble"),
            ("よく", None, "souvent"),
        ];
        let grammar = [
            ("は", "thème de la phrase"),
            ("を", "complément d'objet direct"),
            ("に / へ", "destination (avec いく, くる)"),
            ("で", "lieu de l'action"),
            ("か", "particule interrogative en fin de phrase"),
            ("〜ません", "négation polie du verbe"),
            ("〜ませんか", "invitation : voulez-vous ... ?"),
        ];
        Knowledge {
            words: words.iter().map(|(k, j, m)| word(k, *j, m)).collect(),
            grammar: grammar
                .iter()
                .map(|(t, e)| KnownGrammar {
                    title: t.to_string(),
                    explanation: e.to_string(),
                })
                .collect(),
            focus: vec![
                word("えいが", None, "film"),
                word("いっしょに", None, "ensemble"),
            ],
        }
    }

    // cargo test -- --ignored essai_reel_niveau --nocapture
    #[test]
    #[ignore]
    fn essai_reel_niveau() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            let mut previous = Vec::new();
            for _ in 0..5 {
                let e = generate_with(
                    &ollama,
                    &test_models(model),
                    &realistic_knowledge(),
                    TranslationDirection::FrJp,
                    &previous,
                )
                .await
                .unwrap();
                println!(
                    "PHRASE {} | {} | inconnus {:?}",
                    e.japanese, e.french, e.unknown_words
                );
                previous.push(e.japanese);
            }
            let exercise = TranslationExercise {
                direction: TranslationDirection::FrJp,
                japanese: "あした ともだち と えいが を 見ます".into(),
                french: "Demain, je regarde un film avec un ami.".into(),
                unknown_words: vec![],
            };
            for answer in [
                "あした ともだち と えいが を みます",
                "あした ともだち と えいが に みます",
                "わたし は きょう えいが を みます",
            ] {
                let c = correct_with(&ollama, model, &exercise, answer, &knowledge())
                    .await
                    .unwrap();
                println!("CORRECTION {answer} => {c:?}");
            }
        });
    }

    // Nécessite Ollama lancé avec le modèle par défaut :
    // cargo test -- --ignored essai_reel_traduction --nocapture
    #[test]
    #[ignore]
    fn essai_reel_traduction() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            let exercise = generate_with(
                &ollama,
                &test_models(model),
                &knowledge(),
                TranslationDirection::FrJp,
                &[],
            )
            .await
            .unwrap();
            println!("exercice : {exercise:?}");
            let correction = correct_with(
                &ollama,
                model,
                &exercise,
                "わたし は りんご を たべます",
                &knowledge(),
            )
            .await
            .unwrap();
            println!("correction : {correction:?}");
        });
    }
}
