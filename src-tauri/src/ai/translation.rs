// Exercice de traduction (SPEC.md 3.4) : l'IA crée une phrase avec mes connaissances,
// le code vérifie qu'elle n'utilise rien d'inconnu (garde-fou), puis l'IA corrige ma traduction.

use serde_json::json;
use sqlx::SqlitePool;
use tauri::State;

use super::client::{ChatRequest, ChatTurn, Ollama};
use super::knowledge_prompt::{
    NO_PLURAL_IN_JAPANESE, is_kanji, kanji_reminder, knowledge_rules, normalize_japanese,
    pick_index, unknown_words, writing_rule,
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
         Qualité attendue, par ordre d'importance :\n\
         - La phrase a un sens clair et dit quelque chose de vrai ou de plausible dans la vie \
         réelle, qu'un Japonais pourrait vraiment dire. Interdit : les associations absurdes \
         (ex. にほん は アラビアご の くに です : le Japon n'est pas le pays de l'arabe).\n\
         - Elle sonne naturelle pour un Japonais. Omets le sujet quand il est évident : \
         n'écris わたし は que si la phrase en a vraiment besoin (contraste, présentation).\n\
         - Elle est grammaticalement correcte, avec seulement les mots et la grammaire autorisés. \
         Une phrase simple et juste vaut mieux qu'une phrase longue, fausse ou absurde : \
         3 à 8 mots, sans forcer des combinaisons que ses connaissances ne permettent pas.\n\
         - Écris chaque mot de la liste exactement comme elle (katakana compris : ホテル, \
         jamais ほてる ; mot composé d'un seul bloc : にほんご, jamais にほん ご).\n\
         Réponds :\n\
         - idea : D'ABORD, en français, l'idée de ta phrase : une chose vraie ou plausible \
         qu'on dit dans la vie (ex. 'Mon pays, c'est la France.', 'C'est le chat de Maëlle ?').\n\
         - japanese : cette idée en japonais.\n{}",
        knowledge_rules(knowledge)
    )
}

// Ce qu'exige une forme de phrase pour être possible avec mes connaissances.
#[derive(Clone, Copy, PartialEq)]
enum Needs {
    Nothing,
    Verb,     // au moins un verbe dans mon vocabulaire
    Negation, // un point de grammaire sur la négation
}

// Forme de phrase imposée à chaque demande : sans elle, le modèle produit toujours
// la même affirmation simple. Mesure du 2026-11-25 : imposer une forme impossible
// (lieu, moment, négation sans verbe ni négation connus) fait inventer des mots
// (みます, べんきょう, ない) ou des phrases absurdes. On ne tire donc que des formes possibles.
const SENTENCE_STYLES: [(&str, Needs); 7] = [
    ("une question (avec か)", Needs::Nothing),
    (
        "une réponse naturelle à une question qu'on t'aurait posée",
        Needs::Nothing,
    ),
    (
        "une phrase qui présente ou décrit une personne, un objet ou un lieu",
        Needs::Nothing,
    ),
    ("une phrase négative", Needs::Negation),
    (
        "une phrase qui précise un moment (matin, soir...)",
        Needs::Verb,
    ),
    (
        "une phrase qui précise un lieu ou une destination",
        Needs::Verb,
    ),
    ("une phrase qui relie deux idées", Needs::Verb),
];

// Vrai si un mot ressemble à un verbe : kana finissant par un son en -u et sens à
// l'infinitif (manger, boire, lire...). Heuristique : le type du mot n'est pas transmis à l'IA.
fn looks_like_verb(kana: &str, meaning: &str) -> bool {
    let kana = kana.trim();
    if matches!(kana, "です" | "だ") {
        return false;
    }
    let ends_in_u = kana
        .chars()
        .last()
        .is_some_and(|c| "うくぐすつぬぶむる".contains(c));
    let first_meaning = meaning
        .split([';', ','])
        .next()
        .unwrap_or("")
        .trim()
        .to_lowercase();
    ends_in_u
        && ["er", "ir", "re", "oir"]
            .iter()
            .any(|ending| first_meaning.ends_with(ending))
}

fn can_use(needs: Needs, knowledge: &Knowledge) -> bool {
    match needs {
        Needs::Nothing => true,
        // Verbe typé (conjugaison connue) ou, à défaut, mot qui ressemble à un verbe.
        Needs::Verb => {
            !knowledge.conjugations.is_empty()
                || knowledge
                    .words
                    .iter()
                    .any(|w| looks_like_verb(&w.kana, &w.meanings.join(";")))
        }
        Needs::Negation => knowledge.grammar.iter().any(|g| {
            let text = format!("{} {}", g.title, g.explanation).to_lowercase();
            ["ません", "ない", "じゃ", "négati"]
                .iter()
                .any(|m| text.contains(m))
        }),
    }
}

fn pick_style(knowledge: &Knowledge) -> &'static str {
    let possible: Vec<&str> = SENTENCE_STYLES
        .iter()
        .filter(|(_, needs)| can_use(*needs, knowledge))
        .map(|(style, _)| *style)
        .collect();
    possible[pick_index(possible.len())]
}

fn generation_user_prompt(
    knowledge: &Knowledge,
    previous: &[String],
    rejected: &[String],
    flaw: Option<(&str, Flaw)>,
) -> String {
    let mut prompt = format!(
        "Crée une nouvelle phrase : {}, si sa grammaire est connue (sinon, une autre forme).",
        pick_style(knowledge)
    );
    if let Some(reminder) = kanji_reminder(knowledge) {
        prompt.push(' ');
        prompt.push_str(&reminder);
    }
    if !knowledge.focus.is_empty() {
        let focus: Vec<&str> = knowledge.focus.iter().map(|w| w.kana.as_str()).collect();
        prompt.push_str(&format!(
            " Utilise de préférence, si la phrase garde un sens naturel : {}.",
            focus.join(", ")
        ));
    }
    if !previous.is_empty() {
        prompt.push_str(&format!(
            " Ne reprends pas ces phrases déjà proposées, et change de sujet et de mots : {}.",
            previous.join(" / ")
        ));
    }
    if !rejected.is_empty() {
        prompt.push_str(&format!(
            " Ta phrase précédente utilisait des mots interdits ({}) : n'utilise que la liste.",
            rejected.join(", ")
        ));
    }
    match flaw {
        Some((sentence, Flaw::Absurd)) => prompt.push_str(&format!(
            " Ta phrase précédente ({sentence}) était absurde ou fautive pour un Japonais : \
             choisis une idée simple et vraie."
        )),
        Some((sentence, Flaw::Repeated)) => prompt.push_str(&format!(
            " Ta phrase précédente ({sentence}) avait déjà été proposée : invente une phrase \
             différente."
        )),
        None => {}
    }
    prompt
}

// Défaut d'une phrase générée, repéré après coup (en plus des mots inconnus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Flaw {
    Repeated, // identique à une phrase déjà proposée (moins grave : la phrase reste juste)
    Absurd,   // jugée absurde ou fautive par le contrôle de sens
}

fn generation_schema() -> serde_json::Value {
    json!({
        "type": "object",
        // `idea` en premier : le modèle choisit d'abord une idée sensée, puis l'écrit en
        // japonais (sinon il aligne des mots permis sans souci du sens). Non lu par le code.
        "properties": {
            "idea": { "type": "string" },
            "japanese": { "type": "string" },
            "words": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["idea", "japanese", "words"]
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
    // Mesure du 2026-11-25 (vraie base) : vers le français, "Ce sont les chats de Jonathan"
    // pour ジョナタン の ねこ です était jugé faux ("です est au singulier", ce qui est inexact),
    // et "Mon pays, c'est la France." jugé presque juste.
    let direction_rule = match exercise.direction {
        TranslationDirection::FrJp => format!("- {NO_PLURAL_IN_JAPANESE}\n"),
        TranslationDirection::JpFr => "- Le japonais ne marque ni le nombre ni l'article. Si \
             la phrase japonaise ne précise pas le nombre (pas de chiffre, たち, たくさん...), le \
             singulier ET le pluriel sont justes (ねこ = le chat, un chat, les chats, des chats), \
             avec un article défini ou indéfini. です ne dit rien du nombre. Une différence de \
             nombre ou d'article n'est alors jamais une erreur.\n\
             - Juge le sens, pas la tournure : une phrase française naturelle qui dit la même \
             chose est correcte même si sa construction diffère de la référence (ex. 'Mon pays, \
             c'est la France.' = 'Mon pays est la France.').\n"
            .to_string(),
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
         {direction_rule}\
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
        // Rappel du nombre ici, près des phrases : placé seulement dans les consignes, il était
        // ignoré (mesure du 2026-11-25, 3 pluriels justes sur 3 jugés faux).
        TranslationDirection::JpFr => format!(
            "Phrase à traduire (japonais) : {}\nTraduction de référence : {}\n\
             Traduction de l'élève : {answer}\n\
             Rappel : la référence a choisi un nombre (singulier ou pluriel) et un article au \
             hasard. Sauf si le japonais précise le nombre, l'autre choix est tout aussi juste : \
             ne le corrige pas, et garde le nombre choisi par l'élève dans ta correction.",
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
        knowledge,
    )
    .await?;
    Ok(TranslationExercise {
        direction,
        japanese: japanese_sentence.0,
        french,
        unknown_words: japanese_sentence.1,
    })
}

// Consigne du contrôle de sens. Exemples tirés des phrases mesurées le 2026-11-25 : sans
// eux, le modèle jugeait absurdes des phrases courtes mais justes (とうきょう の ホテル です).
const SENSE_CHECK_PROMPT: &str = "Tu es un professeur de japonais natif. On te donne une phrase \
    japonaise d'exercice pour débutante, au vocabulaire très limité.\n\
    Réponds :\n\
    - french : ce que dit la phrase, en français naturel.\n\
    - verdict : \"ok\" si un Japonais pourrait dire cette phrase dans une situation réelle (les \
    phrases courtes, sans sujet, ou qui répondent à une question implicite sont \"ok\" : ex. \
    わたし の くに は フランス です, とうきょう の ホテル です, ジョナタン の ねこ です) ; \
    \"absurd\" si elle associe des choses sans rapport ou affirme une chose évidemment fausse \
    (ex. マエル は フランスご です : une personne n'\"est\" pas une langue ; \
    にほん は ドイツご です か) ; \"incorrect\" si elle contient une faute de grammaire ou \
    d'écriture (ex. です です, とうきょう に です, ほてる au lieu de ホテル).";

#[derive(serde::Deserialize)]
struct SenseCheck {
    verdict: String,
}

// Contrôle de sens par le modèle principal (mesure du 2026-11-25 sur 20 phrases réelles :
// 10 justes sur 10 acceptées, 6 absurdes ou fautives sur 10 écartées ; environ 0,4 s).
// Imparfait : il filtre, sans garantir.
// Réponse illisible : la phrase est acceptée (le contrôle ne doit pas bloquer l'exercice).
async fn is_sensible(ollama: &Ollama, model: &str, japanese: &str) -> Result<bool, AppError> {
    let result: Result<SenseCheck, AppError> = ollama
        .chat_json(ChatRequest {
            model,
            system: SENSE_CHECK_PROMPT,
            turns: &[ChatTurn::user(japanese)],
            schema: json!({
                "type": "object",
                "properties": {
                    "french": { "type": "string" },
                    "verdict": { "type": "string", "enum": ["ok", "absurd", "incorrect"] }
                },
                "required": ["french", "verdict"]
            }),
            temperature: 0.0,
        })
        .await;
    match result {
        Ok(check) => Ok(check.verdict == "ok"),
        Err(AppError::InvalidAiResponse) => Ok(true),
        Err(e) => Err(e),
    }
}

// Phrase japonaise + ses mots inconnus : la première sensée et sans intrus, sinon la
// meilleure des essais (sensée d'abord, puis le moins d'intrus).
async fn generate_japanese(
    ollama: &Ollama,
    model: &str,
    knowledge: &Knowledge,
    previous: &[String],
) -> Result<(String, Vec<String>), AppError> {
    let system = generation_system_prompt(knowledge);
    let mut rejected: Vec<String> = Vec::new();
    let mut flaw: Option<(String, Flaw)> = None;
    // (phrase, mots inconnus, défaut)
    let mut best: Option<(String, Vec<String>, Option<Flaw>)> = None;

    for _ in 0..MAX_ATTEMPTS {
        let user = generation_user_prompt(
            knowledge,
            previous,
            &rejected,
            flaw.as_ref().map(|(s, f)| (s.as_str(), *f)),
        );
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
        // Répétition exacte (espaces, ponctuation et kanji / kana mis à part) : la consigne
        // "ne reprends pas" ne suffit pas (mesuré). Une phrase déjà acceptée n'est pas revérifiée.
        let normalized = normalize_japanese(&japanese, knowledge);
        let found = if previous
            .iter()
            .any(|p| normalize_japanese(p, knowledge) == normalized)
        {
            Some(Flaw::Repeated)
        } else if !is_sensible(ollama, model, &japanese).await? {
            Some(Flaw::Absurd)
        } else {
            None
        };
        if unknown.is_empty() && found.is_none() {
            return Ok((japanese, unknown));
        }
        log::info!("Phrase rejetée (mots inconnus : {unknown:?}, défaut : {found:?}) : {japanese}");
        // Ordre de préférence : sans défaut < répétée < absurde, puis le moins d'intrus.
        let is_better = best
            .as_ref()
            .is_none_or(|b| (found, unknown.len()) < (b.2, b.1.len()));
        flaw = found.map(|f| (japanese.clone(), f));
        if is_better {
            best = Some((japanese, unknown.clone(), found));
        }
        rejected = unknown;
    }
    best.map(|(japanese, unknown, _)| (japanese, unknown))
        .ok_or(AppError::InvalidAiResponse)
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

// Vrai si la phrase japonaise précise un nombre (chiffre, たち, compteur courant) : le
// singulier et le pluriel français ne sont alors plus interchangeables.
fn japanese_states_number(japanese: &str) -> bool {
    const MARKERS: [&str; 7] = [
        "たち",
        "達",
        "ひとつ",
        "ふたつ",
        "ひとり",
        "ふたり",
        "みっつ",
    ];
    japanese.chars().any(|c| {
        c.is_ascii_digit() || ('０'..='９').contains(&c) || "一二三四五六七八九十百千".contains(c)
    }) || MARKERS.iter().any(|m| japanese.contains(m))
}

// Forme d'une phrase française sans le nombre ni l'article, pour comparer deux traductions
// qui ne diffèrent que par là ("Ce sont les chats." / "C'est le chat."). Volontairement
// grossière : elle ne sert qu'à transformer un verdict en "correct", jamais l'inverse ;
// une comparaison ratée laisse simplement le verdict de l'IA.
fn without_number(french: &str) -> String {
    // \u{2019} : apostrophe typographique, que le clavier peut produire.
    let lower = french.to_lowercase().replace(['\u{2019}', '\''], "' ");
    let mut words: Vec<String> = Vec::new();
    for raw in lower.split(|c: char| c.is_whitespace() || ".,;:!?\"()".contains(c)) {
        let word = match raw {
            "" => continue,
            "le" | "la" | "les" | "l'" | "un" | "une" | "des" => "(article)".to_string(),
            "sont" => "est".to_string(),
            // Pluriel régulier : -s / -x final retiré (mots de plus de 3 lettres).
            w if w.chars().count() > 3 && (w.ends_with('s') || w.ends_with('x')) => {
                w[..w.len() - 1].to_string()
            }
            w => w.to_string(),
        };
        words.push(word);
    }
    // "ce sont" et "est-ce" (question) => "c' est" (forme de "c'est", apostrophe séparée).
    words
        .join(" ")
        .replace("ce est", "c' est")
        .replace("est-ce", "c' est")
}

// Correction de l'IA, puis garde-fous du code (testés ensemble par les essais réels).
async fn correct_with(
    ollama: &Ollama,
    model: &str,
    exercise: &TranslationExercise,
    answer: &str,
    knowledge: &Knowledge,
) -> Result<Correction, AppError> {
    let mut correction = ask_correction(ollama, model, exercise, answer, knowledge).await?;

    // Garde-fou : l'IA juge parfois fausse une réponse identique à sa propre correction
    // (espaces, ponctuation ou kanji / kana mis à part). Le code la remet à "correct".
    if exercise.direction == TranslationDirection::FrJp
        && correction.verdict != Verdict::Correct
        && normalize_japanese(answer, knowledge)
            == normalize_japanese(&correction.correction, knowledge)
    {
        log::info!("Verdict corrigé en 'correct' (réponse identique) : {answer}");
        correction.verdict = Verdict::Correct;
        correction.explanation =
            "Ta traduction est correcte (elle correspond à la correction proposée).".into();
    }

    // Garde-fou du pluriel (vers le français) : réponse identique à la référence au nombre et
    // à l'article près, alors que le japonais ne précise pas le nombre.
    // Mesure du 2026-11-25 : malgré les consignes, 2 pluriels justes sur 5 restaient refusés.
    // Comparaison avec la référence SEULEMENT : la correction de l'IA recopie parfois une
    // réponse fausse ("les voitures de Jonathan" pour マエル の くるま), qui serait validée.
    if exercise.direction == TranslationDirection::JpFr
        && correction.verdict != Verdict::Correct
        && !japanese_states_number(&exercise.japanese)
        && without_number(answer) == without_number(&exercise.french)
    {
        log::info!("Verdict corrigé en 'correct' (nombre ou article seulement) : {answer}");
        correction.verdict = Verdict::Correct;
        correction.correction = answer.to_string();
        correction.explanation = "Ta traduction est correcte : le japonais ne précise pas \
            le nombre, le singulier et le pluriel (avec leur article) sont tous deux justes."
            .into();
    }
    Ok(correction)
}

async fn ask_correction(
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
    correct_with(ollama.inner(), &model, &exercise, answer.trim(), &knowledge).await
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
            ..Knowledge::default()
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
    fn pluriel_et_article_ignores_mais_pas_le_reste() {
        let same = |a: &str, b: &str| without_number(a) == without_number(b);
        assert!(same(
            "Ce sont les voitures de Maëlle.",
            "C'est la voiture de Maëlle."
        ));
        assert!(same(
            "Ce sont des hôtels de Tokyo ?",
            "C'est un hôtel de Tokyo ?"
        ));
        assert!(same("Les chats de Jonathan", "le chat de Jonathan"));
        assert!(same(
            "Ce sont des hôtels de Tokyo ?",
            "Est-ce un hôtel de Tokyo ?"
        ));
        // Autre mot, article manquant, autre personne : différents.
        assert!(!same(
            "Ce sont les chats de Maëlle.",
            "C'est la voiture de Maëlle."
        ));
        assert!(!same("Mon pays est France.", "Mon pays est la France."));
        assert!(!same(
            "C'est la voiture de Jonathan.",
            "C'est la voiture de Maëlle."
        ));
    }

    #[test]
    fn nombre_precise_en_japonais() {
        assert!(!japanese_states_number("マエル の くるま です"));
        assert!(japanese_states_number("ともだち たち です"));
        assert!(japanese_states_number("ねこ が 2 ひき います"));
        assert!(japanese_states_number("ねこ が 三 びき います"));
    }

    #[test]
    fn consignes_de_correction_sur_le_pluriel() {
        let exercise = |direction| TranslationExercise {
            direction,
            japanese: "マエル の くるま です".into(),
            french: "C'est la voiture de Maëlle.".into(),
            unknown_words: vec![],
        };
        let to_french = exercise(TranslationDirection::JpFr);
        assert!(correction_system_prompt(&to_french).contains("le singulier ET le pluriel"));
        assert!(
            correction_user_prompt(&to_french, "x").contains("l'autre choix est tout aussi juste")
        );
        let to_japanese = exercise(TranslationDirection::FrJp);
        assert!(correction_system_prompt(&to_japanese).contains(NO_PLURAL_IN_JAPANESE));
        assert!(!correction_system_prompt(&to_japanese).contains("le singulier ET le pluriel"));
    }

    #[test]
    fn formes_de_phrase_selon_les_connaissances() {
        // Que des noms : ni lieu, ni moment, ni négation (ils faisaient inventer des mots).
        let nouns = Knowledge {
            words: vec![word("ねこ", None, "chat"), word("です", None, "être")],
            ..Knowledge::default()
        };
        assert!(!can_use(Needs::Verb, &nouns));
        assert!(!can_use(Needs::Negation, &nouns));
        assert!(can_use(Needs::Verb, &knowledge())); // たべる : manger
        assert!(looks_like_verb("のむ", "boire; avaler"));
        assert!(!looks_like_verb("とうきょう", "Tokyo"));
        assert!(!looks_like_verb("いす", "chaise"));
    }

    #[test]
    fn consigne_de_generation_et_retour_sur_un_defaut() {
        assert!(generation_system_prompt(&knowledge()).contains("idea : D'ABORD"));
        let user = generation_user_prompt(
            &knowledge(),
            &[],
            &[],
            Some(("にほん は ドイツご です", Flaw::Absurd)),
        );
        assert!(user.contains("(にほん は ドイツご です) était absurde"));
        let user = generation_user_prompt(&knowledge(), &[], &[], Some(("x", Flaw::Repeated)));
        assert!(user.contains("avait déjà été proposée"));
        // Ordre de préférence des essais : sans défaut, puis répétée, puis absurde.
        assert!(None < Some(Flaw::Repeated) && Some(Flaw::Repeated) < Some(Flaw::Absurd));
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
            ..Knowledge::default()
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

    // Diagnostic sur une COPIE de la vraie base (ouverte en lecture seule) :
    // KOUGAKUSHIN_DB=chemin/copie.db KOUGAKUSHIN_N=10 cargo test -- --ignored essai_reel_base --nocapture
    #[test]
    #[ignore]
    fn essai_reel_base() {
        tauri::async_runtime::block_on(async {
            let path = std::env::var("KOUGAKUSHIN_DB").unwrap();
            let count: usize = std::env::var("KOUGAKUSHIN_N")
                .ok()
                .and_then(|n| n.parse().ok())
                .unwrap_or(10);
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&format!("sqlite://{path}?mode=ro"))
                .await
                .unwrap();
            let knowledge = items::find_knowledge(&pool, &[]).await.unwrap();
            println!(
                "{} mots, {} points de grammaire",
                knowledge.words.len(),
                knowledge.grammar.len()
            );
            if std::env::var("KOUGAKUSHIN_SHOW").is_ok() {
                println!("{}", knowledge_rules(&knowledge));
            }
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            let mut previous: Vec<String> = Vec::new();
            for i in 0..count {
                // Relue à chaque phrase, comme l'application : les mots prioritaires changent.
                let knowledge = items::find_knowledge(&pool, &[]).await.unwrap();
                let (japanese, unknown) = generate_japanese(&ollama, model, &knowledge, &previous)
                    .await
                    .unwrap();
                let written = super::super::to_french::with_kanji(&ollama, model, &japanese).await;
                let french = ollama
                    .translate_ja_to_fr(
                        "translategemma:4b",
                        &written,
                        &super::super::to_french::glossary(&japanese, &knowledge),
                    )
                    .await
                    .unwrap();
                println!(
                    "#{i} JP  {japanese}\n    KAN {written}\n    FR  {french}\n    INC {unknown:?}"
                );
                previous.push(japanese);
                if previous.len() > MAX_PREVIOUS {
                    previous.remove(0);
                }
            }
        });
    }

    // Corrections de réponses justes, presque justes et fausses, dans les deux sens, avec la
    // vraie base (copie, lecture seule). Verdict attendu entre crochets.
    // KOUGAKUSHIN_DB=chemin/copie.db cargo test -- --ignored essai_reel_corrections --nocapture
    #[test]
    #[ignore]
    fn essai_reel_corrections() {
        tauri::async_runtime::block_on(async {
            let path = std::env::var("KOUGAKUSHIN_DB").unwrap();
            let pool = sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(1)
                .connect(&format!("sqlite://{path}?mode=ro"))
                .await
                .unwrap();
            let knowledge = items::find_knowledge(&pool, &[]).await.unwrap();
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            // (sens, japonais, français, [(réponse, verdict attendu)])
            type Case<'a> = (
                TranslationDirection,
                &'a str,
                &'a str,
                &'a [(&'a str, &'a str)],
            );
            let cases: [Case; 8] = [
                (
                    TranslationDirection::JpFr,
                    "わたし の くに は フランス です",
                    "Mon pays est la France.",
                    &[
                        ("Mon pays, c'est la France.", "correct"),
                        ("Mon pays est France.", "almost"),
                        ("Je suis en France.", "wrong"),
                    ],
                ),
                (
                    TranslationDirection::JpFr,
                    "ジョナタン の ねこ です",
                    "C'est le chat de Jonathan.",
                    &[
                        ("C'est le chat de Jonathan", "correct"),
                        ("Ce sont les chats de Jonathan.", "correct"),
                        ("C'est le chien de Jonathan.", "wrong"),
                    ],
                ),
                (
                    TranslationDirection::JpFr,
                    "マエル の くるま です",
                    "C'est la voiture de Maëlle.",
                    &[
                        ("Ce sont les voitures de Maëlle.", "correct"),
                        ("Ce sont les voitures de Jonathan.", "wrong"),
                        ("Ce sont les chats de Maëlle.", "wrong"),
                    ],
                ),
                (
                    TranslationDirection::JpFr,
                    "とうきょう の ホテル です か",
                    "Est-ce un hôtel de Tokyo ?",
                    &[
                        ("C'est un hôtel à Tokyo ?", "correct"),
                        ("Ce sont des hôtels de Tokyo ?", "correct"),
                        ("C'est l'hôtel de Kyoto ?", "wrong"),
                    ],
                ),
                (
                    TranslationDirection::FrJp,
                    "わたし の くに は フランス です",
                    "Mon pays est la France.",
                    &[
                        ("わたしのくにはフランスです", "correct"),
                        ("わたし は くに の フランス です", "wrong"),
                    ],
                ),
                (
                    TranslationDirection::FrJp,
                    "ジョナタン の ねこ です",
                    "Ce sont les chats de Jonathan.",
                    &[
                        ("ジョナタン の ねこ です", "correct"),
                        ("ねこ の ジョナタン です", "wrong"),
                    ],
                ),
                (
                    TranslationDirection::FrJp,
                    "あさ の コーヒー です",
                    "C'est le café du matin.",
                    &[
                        ("あさ の コーヒー です", "correct"),
                        ("あさ の コーヒー", "almost"),
                        ("コーヒー の あさ です", "wrong"),
                    ],
                ),
                (
                    TranslationDirection::FrJp,
                    "いぬ と ねこ です",
                    "Ce sont des chiens et des chats.",
                    &[
                        ("いぬ と ねこ です", "correct"),
                        ("いぬ と ねこ と です", "almost"),
                    ],
                ),
            ];
            let (mut good, mut total) = (0, 0);
            for (direction, japanese, french, answers) in cases {
                let exercise = TranslationExercise {
                    direction,
                    japanese: japanese.into(),
                    french: french.into(),
                    unknown_words: vec![],
                };
                for (answer, expected) in answers {
                    let c = correct_with(&ollama, model, &exercise, answer, &knowledge)
                        .await
                        .unwrap();
                    let got = serde_json::to_value(c.verdict).unwrap();
                    let ok = got == *expected;
                    total += 1;
                    good += usize::from(ok);
                    println!(
                        "{} {japanese} / {french} | {answer} [{expected}] => {got} | {} | {}",
                        if ok { "OK" } else { "XX" },
                        c.correction,
                        c.explanation
                    );
                }
            }
            println!("{good}/{total} verdicts attendus");
        });
    }

    // Pluriel (sans base réelle) : "les voitures" pour くるま est juste dans les deux sens.
    // cargo test -- --ignored essai_reel_pluriel --nocapture
    #[test]
    #[ignore]
    fn essai_reel_pluriel() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            let mut k = knowledge();
            k.words.push(word("くるま", None, "voiture"));
            k.words.push(word("マエル", None, "Maëlle"));
            let to_french = TranslationExercise {
                direction: TranslationDirection::JpFr,
                japanese: "マエル の くるま です".into(),
                french: "C'est la voiture de Maëlle.".into(),
                unknown_words: vec![],
            };
            let c = correct_with(
                &ollama,
                model,
                &to_french,
                "Ce sont les voitures de Maëlle.",
                &k,
            )
            .await
            .unwrap();
            println!("vers le français : {c:?}");
            assert_eq!(c.verdict, Verdict::Correct, "{c:?}");
            let to_japanese = TranslationExercise {
                direction: TranslationDirection::FrJp,
                french: "Ce sont les voitures de Maëlle.".into(),
                ..to_french
            };
            let c = correct_with(&ollama, model, &to_japanese, "マエル の くるま です", &k)
                .await
                .unwrap();
            println!("vers le japonais : {c:?}");
            assert_eq!(c.verdict, Verdict::Correct, "{c:?}");
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
