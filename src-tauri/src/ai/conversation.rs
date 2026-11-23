// Conversation avec l'IA (SPEC.md 3.5) : l'IA joue un personnage et ne parle que japonais,
// avec mes connaissances ; à la fin, elle fait le bilan de mes messages en français.

use serde_json::json;
use sqlx::SqlitePool;
use tauri::State;

use super::client::{ChatRequest, ChatTurn, Ollama, Role};
use super::knowledge_prompt::{
    allowed_kanji, kanji_reminder, knowledge_rules, pick_index, unknown_words, writing_rule,
};
use crate::error::AppError;
use crate::items::{self, Knowledge};
use crate::settings;

// Essais (JSON illisible ou mots inconnus) avant d'accepter le meilleur message obtenu.
const MAX_ATTEMPTS: usize = 3;
// Messages transmis à l'IA : les plus récents seulement (consignes plus courtes, réponses rapides).
const MAX_TURNS: usize = 20;

// Premier message (invisible) qui lance la conversation.
const OPENING: &str = "Commence la conversation : salue-moi et lance la situation.";

// Situations du quotidien pour débutante (décision D14).
struct Theme {
    id: &'static str,
    label: &'static str,
    role: &'static str,     // qui joue l'IA (consigne)
    scenario: &'static str, // présentation de la situation, montrée à l'élève
}

const THEMES: [Theme; 6] = [
    Theme {
        id: "cafe",
        label: "Au café",
        role: "un serveur sympathique dans un petit café de Tokyo ; l'élève est cliente",
        scenario: "Tu entres dans un café à Tokyo. Le serveur vient prendre ta commande.",
    },
    Theme {
        id: "introduction",
        label: "Se présenter",
        role: "un étudiant japonais qui rencontre l'élève pour la première fois",
        scenario: "Tu rencontres un étudiant japonais pour la première fois.",
    },
    Theme {
        id: "invitation",
        label: "Inviter un ami",
        role: "un ami japonais de l'élève ; vous organisez une sortie ensemble",
        scenario: "Tu discutes avec un ami japonais pour organiser une sortie.",
    },
    Theme {
        id: "shopping",
        label: "Faire des courses",
        role: "le vendeur d'une petite épicerie de quartier ; l'élève vient acheter quelque chose",
        scenario: "Tu fais des courses dans une petite épicerie. Le vendeur t'accueille.",
    },
    Theme {
        id: "directions",
        label: "Demander son chemin",
        role: "un passant aimable dans une rue de Tokyo ; l'élève cherche un lieu",
        scenario: "Tu es perdue dans Tokyo et tu demandes ton chemin à un passant.",
    },
    Theme {
        id: "weekend",
        label: "Parler du week-end",
        role: "un collègue japonais ; vous parlez de ce que vous faites le week-end",
        scenario: "Tu parles du week-end avec un collègue japonais.",
    },
];

// "surprise" ou absent : tirage au sort.
fn find_theme(theme_id: Option<&str>) -> Result<&'static Theme, AppError> {
    match theme_id {
        None | Some("surprise") => Ok(&THEMES[pick_index(THEMES.len())]),
        Some(id) => THEMES
            .iter()
            .find(|t| t.id == id)
            .ok_or(AppError::Validation("Thème de conversation inconnu")),
    }
}

// --- Types échangés ---

#[derive(Debug, serde::Serialize)]
pub(crate) struct ThemeInfo {
    id: &'static str,
    label: &'static str,
}

// Réponse attendue de l'IA à chaque tour.
#[derive(Debug, serde::Deserialize)]
struct AiMessage {
    japanese: String,
    french: String,
    hint: String,
    words: Vec<String>,
}

// Message de l'IA envoyé au front : traduction et indice affichés seulement à la demande.
#[derive(Debug, serde::Serialize)]
pub(crate) struct AssistantMessage {
    japanese: String,
    french: String,
    hint: String,
    unknown_words: Vec<String>, // garde-fou, comme pour la traduction
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct ConversationStart {
    theme_id: &'static str,
    theme_label: &'static str,
    scenario: &'static str,
    message: AssistantMessage,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct SummaryError {
    student: String,
    correction: String,
    explanation: String,
}

// Bilan de fin (le champ `analysis` du JSON, brouillon de l'IA, est ignoré à la lecture).
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct ConversationSummary {
    errors: Vec<SummaryError>,
    strengths: Vec<String>,
    to_review: Vec<String>,
}

// --- Consignes ---

fn conversation_system_prompt(theme: &Theme, knowledge: &Knowledge, rejected: &[String]) -> String {
    let mut prompt = format!(
        "Tu joues {}. Tu es aussi un professeur de japonais natif et expérimenté : tu adaptes \
         ton langage au niveau de ton interlocutrice, une francophone qui apprend en cours.\n\
         Déroulement :\n\
         - Dans `japanese`, tu parles UNIQUEMENT japonais : 1 ou 2 phrases courtes et naturelles, \
         comme un vrai Japonais (sujet omis quand il est évident), qui font avancer la situation.\n\
         - Termine le plus souvent par une question simple, pour la faire parler.\n\
         - Ne corrige pas ses erreurs pendant la conversation : le bilan s'en chargera. \
         Si tu ne comprends pas, demande-lui gentiment de reformuler, en japonais simple.\n\
         - `french` : traduction française naturelle de TON message japonais (jamais de celui \
         de l'élève).\n\
         - `hint` : en français, une piste de réponse courte, avec 1 à 3 mots japonais utiles \
         tirés de sa liste.\n{}",
        theme.role,
        knowledge_rules(knowledge)
    );
    if let Some(reminder) = kanji_reminder(knowledge) {
        prompt.push_str("\n\n");
        prompt.push_str(&reminder);
    }
    if !rejected.is_empty() {
        prompt.push_str(&format!(
            "\n\nAttention : n'utilise pas ces mots, absents de sa liste : {}.",
            rejected.join(", ")
        ));
    }
    prompt
}

fn message_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "japanese": { "type": "string" },
            "french": { "type": "string" },
            "hint": { "type": "string" },
            "words": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["japanese", "french", "hint", "words"]
    })
}

// Conversation lisible par l'IA pour le bilan : une ligne par message.
fn transcript(history: &[ChatTurn]) -> String {
    history
        .iter()
        .map(|turn| match turn.role {
            Role::Assistant => format!("Toi : {}", turn.content),
            Role::User => format!("Élève : {}", turn.content),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn summary_system_prompt(theme: &Theme, knowledge: &Knowledge) -> String {
    format!(
        "Tu es un professeur de japonais natif, expérimenté et exigeant. Voici une conversation \
         entre toi ({}) et ton élève francophone. Fais le bilan de SES messages (pas des tiens), \
         en français.\n\
         - analysis : D'ABORD, reprends chacun de ses messages et analyse-le précisément \
         (particules, conjugaison, vocabulaire, naturel).\n\
         - errors : une entrée par erreur réelle. student = sa phrase telle qu'écrite ; \
         correction = la phrase corrigée ; explanation = courte, nomme la notion en jeu. \
         N'invente pas d'erreur : un sujet omis n'en est pas une. Liste vide si tout est juste.\n\
         - strengths : 1 à 3 réussites précises, uniquement ce qui est réellement juste dans \
         ses phrases (une tournure fautive ne peut pas être citée en réussite).\n\
         - to_review : 2 ou 3 notions ou mots à revoir.\n\
         Pour les corrections en japonais : {}",
        theme.role,
        writing_rule(&allowed_kanji(knowledge))
    )
}

fn summary_schema() -> serde_json::Value {
    json!({
        "type": "object",
        // `analysis` en premier : le modèle analyse avant de lister les erreurs.
        "properties": {
            "analysis": { "type": "string" },
            "errors": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "student": { "type": "string" },
                        "correction": { "type": "string" },
                        "explanation": { "type": "string" }
                    },
                    "required": ["student", "correction", "explanation"]
                }
            },
            "strengths": { "type": "array", "items": { "type": "string" } },
            "to_review": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["analysis", "errors", "strengths", "to_review"]
    })
}

// --- Contrôles de l'historique envoyé par le front ---

// Le dernier message doit être le mien, non vide ; seuls les MAX_TURNS derniers sont gardés.
fn checked_history(mut history: Vec<ChatTurn>) -> Result<Vec<ChatTurn>, AppError> {
    match history.last() {
        Some(last) if last.role == Role::User && !last.content.trim().is_empty() => {}
        _ => {
            return Err(AppError::Validation("Écris ta réponse avant de l'envoyer."));
        }
    }
    let skip = history.len().saturating_sub(MAX_TURNS);
    history.drain(..skip);
    Ok(history)
}

// --- Logique (sans Tauri, testable) ---

async fn next_message(
    ollama: &Ollama,
    model: &str,
    theme: &Theme,
    knowledge: &Knowledge,
    history: &[ChatTurn],
) -> Result<AssistantMessage, AppError> {
    // Le message d'ouverture précède toujours l'historique : l'IA a ainsi un premier message
    // de l'élève auquel répondre, ce que certains modèles exigent.
    let mut turns = vec![ChatTurn::user(OPENING)];
    turns.extend_from_slice(history);

    let mut rejected: Vec<String> = Vec::new();
    let mut best: Option<AssistantMessage> = None;
    for _ in 0..MAX_ATTEMPTS {
        let result: Result<AiMessage, AppError> = ollama
            .chat_json(ChatRequest {
                model,
                system: &conversation_system_prompt(theme, knowledge, &rejected),
                turns: &turns,
                schema: message_schema(),
                temperature: 0.7,
            })
            .await;
        // JSON illisible : on retente (essai suivant) au lieu d'échouer tout de suite.
        let reply = match result {
            Err(AppError::InvalidAiResponse) => continue,
            other => other?,
        };
        let unknown = unknown_words(&reply.japanese, &reply.words, knowledge);
        let message = AssistantMessage {
            japanese: reply.japanese.trim().to_string(),
            french: reply.french.trim().to_string(),
            hint: reply.hint.trim().to_string(),
            unknown_words: unknown.clone(),
        };
        if unknown.is_empty() {
            return Ok(message);
        }
        log::info!(
            "Message rejeté (mots inconnus : {unknown:?}) : {}",
            message.japanese
        );
        let is_better = best
            .as_ref()
            .is_none_or(|b| message.unknown_words.len() < b.unknown_words.len());
        if is_better {
            best = Some(message);
        }
        rejected = unknown;
    }
    best.ok_or(AppError::InvalidAiResponse)
}

async fn summarize(
    ollama: &Ollama,
    model: &str,
    theme: &Theme,
    knowledge: &Knowledge,
    history: &[ChatTurn],
) -> Result<ConversationSummary, AppError> {
    ollama
        .chat_json(ChatRequest {
            model,
            system: &summary_system_prompt(theme, knowledge),
            turns: &[ChatTurn::user(format!(
                "Conversation :\n{}",
                transcript(history)
            ))],
            schema: summary_schema(),
            temperature: 0.2, // bilan : on veut de la constance
        })
        .await
}

// Connaissances des leçons choisies + modèle réglé ; erreur claire s'il n'y a aucun mot.
async fn context(
    pool: &SqlitePool,
    lesson_ids: Option<Vec<i64>>,
) -> Result<(Knowledge, String), AppError> {
    let knowledge = items::find_knowledge(pool, &lesson_ids.unwrap_or_default()).await?;
    if knowledge.words.is_empty() {
        return Err(AppError::Validation(
            "Aucun mot de vocabulaire actif dans ces leçons.",
        ));
    }
    let model = settings::load(pool).await?.ollama_model;
    Ok((knowledge, model))
}

// --- Commandes Tauri ---

#[tauri::command]
pub(crate) fn list_conversation_themes() -> Vec<ThemeInfo> {
    THEMES
        .iter()
        .map(|t| ThemeInfo {
            id: t.id,
            label: t.label,
        })
        .collect()
}

// JS : invoke("start_conversation", { lessonIds: [1], themeId: "cafe" }) ; themeId absent ou
// "surprise" = tirage au sort.
#[tauri::command]
pub(crate) async fn start_conversation(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    lesson_ids: Option<Vec<i64>>,
    theme_id: Option<String>,
) -> Result<ConversationStart, AppError> {
    let theme = find_theme(theme_id.as_deref())?;
    let (knowledge, model) = context(pool.inner(), lesson_ids).await?;
    let message = next_message(ollama.inner(), &model, theme, &knowledge, &[]).await?;
    Ok(ConversationStart {
        theme_id: theme.id,
        theme_label: theme.label,
        scenario: theme.scenario,
        message,
    })
}

// `history` : tous les messages affichés (ceux de l'IA en japonais seulement), le mien en dernier.
#[tauri::command]
pub(crate) async fn reply_conversation(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    lesson_ids: Option<Vec<i64>>,
    theme_id: String,
    history: Vec<ChatTurn>,
) -> Result<AssistantMessage, AppError> {
    let theme = find_theme(Some(&theme_id))?;
    let history = checked_history(history)?;
    let (knowledge, model) = context(pool.inner(), lesson_ids).await?;
    next_message(ollama.inner(), &model, theme, &knowledge, &history).await
}

#[tauri::command]
pub(crate) async fn end_conversation(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    lesson_ids: Option<Vec<i64>>,
    theme_id: String,
    history: Vec<ChatTurn>,
) -> Result<ConversationSummary, AppError> {
    let theme = find_theme(Some(&theme_id))?;
    if !history.iter().any(|t| t.role == Role::User) {
        return Err(AppError::Validation(
            "Réponds au moins une fois avant de terminer.",
        ));
    }
    let (knowledge, model) = context(pool.inner(), lesson_ids).await?;
    summarize(ollama.inner(), &model, theme, &knowledge, &history).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::items::KnownWord;

    fn turn(role: Role, content: &str) -> ChatTurn {
        ChatTurn {
            role,
            content: content.into(),
        }
    }

    fn knowledge() -> Knowledge {
        let word = |kana: &str, kanji: Option<&str>, meaning: &str| KnownWord {
            kana: kana.into(),
            kanji: kanji.map(String::from),
            meaning: meaning.into(),
        };
        Knowledge {
            words: vec![
                word("コーヒー", None, "café"),
                word("おちゃ", None, "thé"),
                word("のむ", Some("飲む"), "boire"),
                word("ください", None, "s'il vous plaît (donnez-moi)"),
                word("おいしい", None, "bon"),
                word("ひとつ", None, "un (objet)"),
            ],
            ..Knowledge::default()
        }
    }

    #[test]
    fn themes_surprise_et_inconnu() {
        assert_eq!(
            find_theme(Some("cafe")).map(|t| t.label).ok(),
            Some("Au café")
        );
        // Un seul tirage, puis on vérifie qu'il fait partie de la liste.
        let drawn = find_theme(None).map(|t| t.id).unwrap_or("");
        assert!(THEMES.iter().any(|t| t.id == drawn));
        assert!(find_theme(Some("surprise")).is_ok());
        assert!(find_theme(Some("plage")).is_err());
    }

    #[test]
    fn historique_doit_finir_par_mon_message_et_est_limite() {
        assert!(checked_history(vec![]).is_err());
        assert!(checked_history(vec![turn(Role::Assistant, "こんにちは")]).is_err());
        assert!(checked_history(vec![turn(Role::User, "  ")]).is_err());

        let long: Vec<ChatTurn> = (0..30)
            .map(|i| {
                turn(
                    if i % 2 == 0 {
                        Role::Assistant
                    } else {
                        Role::User
                    },
                    "x",
                )
            })
            .collect();
        assert_eq!(checked_history(long).map(|h| h.len()).ok(), Some(MAX_TURNS));
    }

    #[test]
    fn transcription_pour_le_bilan() {
        let history = [
            turn(Role::Assistant, "いらっしゃいませ"),
            turn(Role::User, "コーヒー ください"),
        ];
        assert_eq!(
            transcript(&history),
            "Toi : いらっしゃいませ\nÉlève : コーヒー ください"
        );
    }

    #[test]
    fn consignes_contiennent_le_role_et_les_regles() {
        let theme = find_theme(Some("cafe")).unwrap();
        let prompt = conversation_system_prompt(theme, &knowledge(), &["みせ".to_string()]);
        assert!(prompt.contains("un serveur sympathique"));
        assert!(prompt.contains("- のむ (飲む) : boire"));
        assert!(prompt.contains("飲む (pas のむ)"));
        assert!(prompt.contains("n'utilise pas ces mots, absents de sa liste : みせ"));
    }

    // Nécessite Ollama lancé : cargo test -- --ignored essai_reel_conversation --nocapture
    #[test]
    #[ignore]
    fn essai_reel_conversation() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = "gemma4:e4b-it-q4_K_M";
            let theme = find_theme(Some("cafe")).unwrap();
            let first = next_message(&ollama, model, theme, &knowledge(), &[])
                .await
                .unwrap();
            println!("IA 1 : {first:?}");
            // Erreur volontaire : を au lieu de の n'a pas de sens ici ; ください sans を.
            let mut history = vec![
                turn(Role::Assistant, &first.japanese),
                turn(Role::User, "コーヒー ひとつ を ください"),
            ];
            let second = next_message(&ollama, model, theme, &knowledge(), &history)
                .await
                .unwrap();
            println!("IA 2 : {second:?}");
            history.push(turn(Role::Assistant, &second.japanese));
            history.push(turn(Role::User, "おちゃ が おいしい です か"));
            let summary = summarize(&ollama, model, theme, &knowledge(), &history)
                .await
                .unwrap();
            println!("BILAN : {summary:?}");
        });
    }
}
