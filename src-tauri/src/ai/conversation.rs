// Conversation avec l'IA (SPEC.md 3.5) : l'IA joue un personnage et ne parle que japonais,
// avec mes connaissances ; à la fin, elle fait le bilan de mes messages en français.

use serde_json::json;
use sqlx::SqlitePool;
use tauri::State;

use super::client::{ChatRequest, ChatTurn, Ollama, Role};
use super::knowledge_prompt::{
    NO_PLURAL_IN_JAPANESE, allowed_kanji, is_kanji, kanji_reminder, knowledge_rules,
    normalize_japanese, pick_index, unknown_words, writing_rule,
};
use super::to_french::to_french;
use crate::error::AppError;
use crate::items::{self, Knowledge};
use crate::settings;

// Essais (JSON illisible ou mots inconnus) avant d'accepter le meilleur message obtenu.
const MAX_ATTEMPTS: usize = 3;
// Messages transmis à l'IA : les plus récents seulement (consignes plus courtes, réponses rapides).
const MAX_TURNS: usize = 20;

// En dessous de ce nombre de mots actifs, l'écran prévient que les conversations seront très
// simples (décision D19). Valeur choisie par Claude, modifiable.
const LIMITED_VOCABULARY: usize = 30;

// Situations du quotidien pour débutante (décision D14).
struct Theme {
    id: &'static str,
    label: &'static str,
    role: &'static str,     // qui joue l'IA (consigne)
    scenario: &'static str, // présentation de la situation, montrée à l'élève
    opening: &'static str,  // ce que doit faire le premier message de l'IA, selon son rôle
}

const THEMES: [Theme; 6] = [
    Theme {
        id: "cafe",
        label: "Au café",
        role: "un serveur sympathique dans un petit café de Tokyo ; l'élève est cliente",
        scenario: "Tu entres dans un café à Tokyo. Le serveur vient prendre ta commande.",
        opening: "accueille ta cliente comme un serveur et invite-la à commander",
    },
    Theme {
        id: "introduction",
        label: "Se présenter",
        role: "un étudiant japonais qui rencontre l'élève pour la première fois",
        scenario: "Tu rencontres un étudiant japonais pour la première fois.",
        opening: "salue-la et présente-toi en une phrase",
    },
    Theme {
        id: "invitation",
        label: "Inviter un ami",
        role: "un ami japonais de l'élève ; vous organisez une sortie ensemble",
        scenario: "Tu discutes avec un ami japonais pour organiser une sortie.",
        opening: "salue ton amie et propose-lui une activité simple",
    },
    Theme {
        id: "shopping",
        label: "Faire des courses",
        role: "le vendeur d'une petite épicerie de quartier ; l'élève vient acheter quelque chose",
        scenario: "Tu fais des courses dans une petite épicerie. Le vendeur t'accueille.",
        opening: "accueille ta cliente comme un vendeur",
    },
    Theme {
        id: "directions",
        label: "Demander son chemin",
        role: "un passant aimable dans une rue de Tokyo ; l'élève cherche un lieu",
        scenario: "Tu es perdue dans Tokyo et tu demandes ton chemin à un passant.",
        // C'est elle qui aborde le passant : il ne lance pas la conversation de lui-même.
        opening: "elle vient de t'interpeller dans la rue : réponds-lui brièvement et \
                  montre-toi prêt à l'aider",
    },
    Theme {
        id: "weekend",
        label: "Parler du week-end",
        role: "un collègue japonais ; vous parlez de ce que vous faites le week-end",
        scenario: "Tu parles du week-end avec un collègue japonais.",
        opening: "salue-la et demande-lui ce qu'elle fait ce week-end",
    },
];

// Premier message (invisible) qui lance la conversation, propre à la situation.
fn opening(theme: &Theme) -> String {
    format!(
        "Commence la conversation. Ton premier message : {}.",
        theme.opening
    )
}

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
    hint: String,
    words: Vec<String>,
}

// Message de l'IA envoyé au front. L'indice est affiché à la demande ; la traduction est
// demandée à part au traducteur dédié (commande translate_message), seulement si on clique.
#[derive(Debug, serde::Serialize)]
pub(crate) struct AssistantMessage {
    japanese: String,
    hint: String,
    unknown_words: Vec<String>, // garde-fou, comme pour la traduction
}

#[derive(Debug, serde::Serialize)]
pub(crate) struct ConversationStart {
    theme_id: &'static str,
    theme_label: &'static str,
    scenario: &'static str,
    // Moins de LIMITED_VOCABULARY mots actifs : l'écran prévient (D19).
    limited_vocabulary: bool,
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
         - Chaque message a un sens clair dans la situation et répond logiquement à ce qu'elle \
         vient de dire. Si son vocabulaire ne permet pas vraiment la situation, adapte la \
         situation à ce qu'elle sait dire, sans sortir de ton personnage, plutôt que d'écrire \
         des phrases sans lien entre elles.\n\
         - Termine le plus souvent par une question simple, pour la faire parler.\n\
         - Ne corrige pas ses erreurs pendant la conversation : le bilan s'en chargera. \
         Si tu ne comprends pas, demande-lui gentiment de reformuler, en japonais simple.\n\
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
            "hint": { "type": "string" },
            "words": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["japanese", "hint", "words"]
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

// Correction d'UN message (D34) : une petite tâche, bien plus fiable pour un petit modèle
// que tout le bilan d'un coup.
fn review_system_prompt(theme: &Theme, knowledge: &Knowledge) -> String {
    format!(
        "Tu es un professeur de japonais natif, expérimenté et honnête. Dans une conversation, \
         tu joues {} ; ton élève francophone vient de te répondre. Juge UNIQUEMENT sa réponse, \
         en tenant compte de ta question.\n\
         Ne sont PAS des erreurs : une réponse courte mais naturelle (ex. おなまえは？) ; un \
         sujet omis ; un mot en kanji plutôt qu'en kana (ou l'inverse) ; l'absence d'espaces ; \
         une autre formulation correcte.\n\
         {NO_PLURAL_IN_JAPANESE}\n\
         Sont des erreurs : particule, conjugaison, mot faux ou qui ne répond pas à la question, \
         kanji qui ne correspond pas au mot voulu (souvent une erreur de conversion du clavier, \
         ex. 先制 au lieu de 先生).\n\
         Si une partie est incompréhensible, ne devine pas une autre phrase : dis-le dans \
         l'explication.\n\
         - analysis : D'ABORD, analyse sa réponse mot par mot.\n\
         - is_correct : true si la réponse est juste et naturelle.\n\
         - correction : sa réponse corrigée, au plus près de ce qu'elle a écrit (vide si juste).\n\
         - explanation : en français, 1 à 2 phrases qui nomment la notion en jeu (vide si juste).\n\
         Pour la correction : {}",
        theme.role,
        writing_rule(&allowed_kanji(knowledge))
    )
}

fn review_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "analysis": { "type": "string" },
            "is_correct": { "type": "boolean" },
            "correction": { "type": "string" },
            "explanation": { "type": "string" }
        },
        "required": ["analysis", "is_correct", "correction", "explanation"]
    })
}

// Bilan final : les erreurs sont déjà relevées message par message ; l'IA ne rédige que
// les réussites et les notions à revoir.
fn summary_system_prompt(theme: &Theme, errors: &[SummaryError]) -> String {
    let errors = if errors.is_empty() {
        "aucune".to_string()
    } else {
        errors
            .iter()
            .map(|e| {
                format!(
                    "\"{}\" => \"{}\" ({})",
                    e.student, e.correction, e.explanation
                )
            })
            .collect::<Vec<_>>()
            .join(" ; ")
    };
    format!(
        "Tu es un professeur de japonais natif. Voici une conversation entre toi ({}) et ton \
         élève francophone. Ses erreurs, déjà vérifiées : {errors}.\n\
         Réponds en français :\n\
         - strengths : 1 à 3 réussites précises, en citant ce qu'elle a RÉELLEMENT écrit et qui \
         est juste (jamais une phrase de la liste des erreurs).\n\
         - to_review : 2 ou 3 notions à revoir, tirées de ses erreurs (s'il n'y en a pas, des \
         notions qu'elle a utilisées et qui méritent d'être consolidées). Jamais le pluriel : \
         {NO_PLURAL_IN_JAPANESE}",
        theme.role
    )
}

fn summary_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "strengths": { "type": "array", "items": { "type": "string" } },
            "to_review": { "type": "array", "items": { "type": "string" } }
        },
        "required": ["strengths", "to_review"]
    })
}

// Kanji de mon message absents de ma base (D35) : souvent une erreur de conversion du clavier
// (先制 au lieu de 先生). Contrôle par le code, sans IA. Sans doublon, dans l'ordre.
fn my_unknown_kanji(text: &str, knowledge: &Knowledge) -> Vec<String> {
    let known = allowed_kanji(knowledge);
    let mut unknown: Vec<String> = Vec::new();
    for c in text.chars().filter(|&c| is_kanji(c) && !known.contains(c)) {
        let kanji = c.to_string();
        if !unknown.contains(&kanji) {
            unknown.push(kanji);
        }
    }
    unknown
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
    let mut turns = vec![ChatTurn::user(opening(theme))];
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

// Réponse de l'IA pour la correction d'un message.
#[derive(serde::Deserialize)]
struct MessageReview {
    is_correct: bool,
    correction: String,
    explanation: String,
}

// Corrige mon dernier message (D34), au vu du message de l'IA qui le précède.
// None si le message est juste ou si l'erreur annoncée ne résiste pas aux contrôles.
async fn review_message(
    ollama: &Ollama,
    model: &str,
    theme: &Theme,
    knowledge: &Knowledge,
    history: &[ChatTurn],
) -> Result<Option<SummaryError>, AppError> {
    let Some((answer, before)) = history.split_last() else {
        return Ok(None);
    };
    // Juste la question et la réponse : le contexte utile, sans noyer le modèle.
    let exchange = transcript(&history[before.len().saturating_sub(1)..]);
    let review: MessageReview = ollama
        .chat_json(ChatRequest {
            model,
            system: &review_system_prompt(theme, knowledge),
            turns: &[ChatTurn::user(exchange.clone())],
            schema: review_schema(),
            temperature: 0.2,
        })
        .await?;
    if review.is_correct || review.correction.trim().is_empty() {
        return Ok(None);
    }
    // `student` est mon message exact, recopié par le code : l'IA ne peut pas l'inventer.
    let error = SummaryError {
        student: answer.content.trim().to_string(),
        correction: review.correction.trim().to_string(),
        explanation: review.explanation.trim().to_string(),
    };
    // 1. Contrôle par le code : correction identique à mon message (espaces, ponctuation,
    //    kanji / kana mis à part) => pas une erreur.
    if normalize_japanese(&error.student, knowledge)
        == normalize_japanese(&error.correction, knowledge)
    {
        log::info!("Fausse erreur écartée (identique) : {}", error.student);
        return Ok(None);
    }
    // 2. Second avis de l'IA, centré sur cette seule phrase.
    if !is_real_error(ollama, model, &exchange, &error).await? {
        log::info!("Fausse erreur écartée (second avis) : {}", error.student);
        return Ok(None);
    }
    Ok(Some(error))
}

// Réponse de l'IA pour le bilan (réussites et notions à revoir).
#[derive(serde::Deserialize)]
struct SummaryText {
    strengths: Vec<String>,
    to_review: Vec<String>,
}

// Bilan : `errors` vient des corrections faites au fil de la conversation (D34).
async fn summarize(
    ollama: &Ollama,
    model: &str,
    theme: &Theme,
    history: &[ChatTurn],
    errors: Vec<SummaryError>,
) -> Result<ConversationSummary, AppError> {
    let text: SummaryText = ollama
        .chat_json(ChatRequest {
            model,
            system: &summary_system_prompt(theme, &errors),
            turns: &[ChatTurn::user(format!(
                "Conversation :\n{}",
                transcript(history)
            ))],
            schema: summary_schema(),
            temperature: 0.2, // bilan : on veut de la constance
        })
        .await?;
    Ok(ConversationSummary {
        errors,
        strengths: text.strengths,
        to_review: text.to_review,
    })
}

// Réponse du second avis.
#[derive(serde::Deserialize)]
struct ErrorCheck {
    is_error: bool,
}

// Juge une seule erreur annoncée : est-elle réelle ?
async fn is_real_error(
    ollama: &Ollama,
    model: &str,
    conversation: &str,
    error: &SummaryError,
) -> Result<bool, AppError> {
    let system = format!(
        "Tu es un professeur de japonais natif, rigoureux et honnête. Un correcteur \
         affirme qu'une phrase d'élève contient une erreur. Vérifie TOI-MÊME, \
         caractère par caractère, avant de répondre.\n\
         Ne sont JAMAIS des erreurs : écrire un mot en kanji plutôt qu'en kana (ou \
         l'inverse) ; l'absence d'espaces ; omettre un sujet évident ; une formulation \
         correcte et naturelle différente de la correction ; l'absence de marque de pluriel.\n\
         {NO_PLURAL_IN_JAPANESE}\n\
         - analysis : d'abord, vérifie point par point ce que dit le correcteur.\n\
         - is_error : true seulement si la phrase de l'élève est réellement fautive."
    );
    let user = format!(
        "Conversation :\n{conversation}\n\nPhrase de l'élève : {}\nCorrection proposée : {}\n\
         Explication du correcteur : {}",
        error.student, error.correction, error.explanation
    );
    let check: ErrorCheck = ollama
        .chat_json(ChatRequest {
            model,
            system: &system,
            turns: &[ChatTurn::user(user)],
            schema: json!({
                "type": "object",
                "properties": {
                    "analysis": { "type": "string" },
                    "is_error": { "type": "boolean" }
                },
                "required": ["analysis", "is_error"]
            }),
            temperature: 0.0, // jugement : le plus déterministe possible
        })
        .await?;
    Ok(check.is_error)
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
    // Journal (D36) : conversations relisibles pour diagnostiquer un bilan absurde.
    log::info!(
        "Conversation [{}] début. IA : {}",
        theme.id,
        message.japanese
    );
    Ok(ConversationStart {
        theme_id: theme.id,
        theme_label: theme.label,
        scenario: theme.scenario,
        limited_vocabulary: knowledge.words.len() < LIMITED_VOCABULARY,
        message,
    })
}

// Traduction en français d'un message de l'IA, à la demande (bouton "Traduction").
// Faite par le traducteur dédié, pas par le modèle de conversation (D22).
#[tauri::command]
pub(crate) async fn translate_message(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    text: String,
) -> Result<String, AppError> {
    if text.trim().is_empty() {
        return Err(AppError::Validation("Rien à traduire."));
    }
    let settings = settings::load(pool.inner()).await?;
    // Toute ma base : le glossaire du traducteur ne contient que les mots de la phrase.
    let knowledge = items::find_knowledge(pool.inner(), &[]).await?;
    to_french(
        ollama.inner(),
        &settings.ollama_model,
        &settings.translation_model,
        text.trim(),
        &knowledge,
    )
    .await
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
    if let Some(mine) = history.last() {
        log::info!("Conversation [{}] élève : {}", theme.id, mine.content);
    }
    let message = next_message(ollama.inner(), &model, theme, &knowledge, &history).await?;
    log::info!("Conversation [{}] IA : {}", theme.id, message.japanese);
    Ok(message)
}

// D35 : kanji de mon message absents de toute ma base (pas seulement des leçons choisies).
// JS : invoke("check_my_message", { text }) => ["制"].
#[tauri::command]
pub(crate) async fn check_my_message(
    pool: State<'_, SqlitePool>,
    text: String,
) -> Result<Vec<String>, AppError> {
    let knowledge = items::find_knowledge(pool.inner(), &[]).await?;
    Ok(my_unknown_kanji(&text, &knowledge))
}

// D34 : correction de mon dernier message, lancée en arrière-plan par l'écran juste après
// l'envoi. Même `history` que reply_conversation. None = pas d'erreur.
#[tauri::command]
pub(crate) async fn review_conversation_message(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    lesson_ids: Option<Vec<i64>>,
    theme_id: String,
    history: Vec<ChatTurn>,
) -> Result<Option<SummaryError>, AppError> {
    let theme = find_theme(Some(&theme_id))?;
    let history = checked_history(history)?;
    let (knowledge, model) = context(pool.inner(), lesson_ids).await?;
    let review = review_message(ollama.inner(), &model, theme, &knowledge, &history).await?;
    match &review {
        Some(e) => log::info!(
            "Conversation [{}] erreur : {} => {} ({})",
            theme.id,
            e.student,
            e.correction,
            e.explanation
        ),
        None => log::info!("Conversation [{}] message juste", theme.id),
    }
    Ok(review)
}

// `errors` : les corrections renvoyées par review_conversation_message pendant la conversation.
#[tauri::command]
pub(crate) async fn end_conversation(
    pool: State<'_, SqlitePool>,
    ollama: State<'_, Ollama>,
    lesson_ids: Option<Vec<i64>>,
    theme_id: String,
    history: Vec<ChatTurn>,
    errors: Vec<SummaryError>,
) -> Result<ConversationSummary, AppError> {
    let theme = find_theme(Some(&theme_id))?;
    if !history.iter().any(|t| t.role == Role::User) {
        return Err(AppError::Validation(
            "Réponds au moins une fois avant de terminer.",
        ));
    }
    let (_, model) = context(pool.inner(), lesson_ids).await?;
    let summary = summarize(ollama.inner(), &model, theme, &history, errors).await?;
    log::info!(
        "Conversation [{}] bilan : {} erreur(s), réussites {:?}, à revoir {:?}",
        theme.id,
        summary.errors.len(),
        summary.strengths,
        summary.to_review
    );
    Ok(summary)
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
            meanings: vec![meaning.into()],
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
    fn kanji_hors_de_ma_base_dans_mon_message() {
        // 飲 est connu (飲む) ; 先 et 制 non, 先 n'est signalé qu'une fois.
        assert_eq!(
            my_unknown_kanji("おちゃを飲みます。先制、先", &knowledge()),
            vec!["先".to_string(), "制".to_string()]
        );
        assert!(my_unknown_kanji("コーヒー ください", &knowledge()).is_empty());
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

    #[test]
    fn correction_et_bilan_n_exigent_pas_de_pluriel() {
        let theme = find_theme(Some("cafe")).unwrap();
        assert!(review_system_prompt(theme, &knowledge()).contains(NO_PLURAL_IN_JAPANESE));
        assert!(summary_system_prompt(theme, &[]).contains("Jamais le pluriel"));
    }

    // Réponses justes qu'un francophone pense au pluriel : aucune erreur attendue.
    // cargo test -- --ignored essai_reel_pluriel_conversation --nocapture
    #[test]
    #[ignore]
    fn essai_reel_pluriel_conversation() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            let mut k = knowledge();
            for (kana, meaning) in [("ねこ", "chat"), ("いぬ", "chien"), ("すき", "aimer")] {
                k.words.push(KnownWord {
                    kana: kana.into(),
                    kanji: None,
                    meaning: meaning.into(),
                });
            }
            let theme = find_theme(Some("introduction")).unwrap();
            for (question, answer) in [
                (
                    "いぬ と ねこ、どちら が すき です か？",
                    "ねこ が すき です。",
                ),
                ("なに を のみます か？", "コーヒー と おちゃ を のみます。"),
            ] {
                let history = [turn(Role::Assistant, question), turn(Role::User, answer)];
                let review = review_message(&ollama, model, theme, &k, &history)
                    .await
                    .unwrap();
                println!("{question} / {answer} => {review:?}");
                assert!(review.is_none(), "fausse erreur : {review:?}");
            }
        });
    }

    // Vocabulaire réel de la développeuse au 2026-11-23 (8 mots) sur la situation la plus difficile.
    // cargo test -- --ignored essai_reel_petit_vocabulaire --nocapture
    #[test]
    #[ignore]
    fn essai_reel_petit_vocabulaire() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            let w = |kana: &str, kanji: Option<&str>, meaning: &str| KnownWord {
                kana: kana.into(),
                kanji: kanji.map(String::from),
                meanings: vec![meaning.into()],
            };
            let k = Knowledge {
                words: vec![
                    w("たべる", Some("食べる"), "manger"),
                    w("わたし", Some("私"), "je; moi"),
                    w("ともだち", None, "ami"),
                    w("すし", None, "sushi"),
                    w("さかな", Some("魚"), "poisson"),
                    w("ばんごはん", None, "dîner"),
                    w("あさごはん", None, "petit-déjeuner"),
                ],
                grammar: vec![crate::items::KnownGrammar {
                    title: "〜ています".into(),
                    explanation: "Action en cours".into(),
                }],
                focus: vec![],
                ..Knowledge::default()
            };
            let theme = find_theme(Some("directions")).unwrap();
            let first = next_message(&ollama, model, theme, &k, &[]).await.unwrap();
            let fr = ollama
                .translate_ja_to_fr("translategemma:4b", &first.japanese, "")
                .await
                .unwrap();
            println!(
                "IA 1 : {} => {fr} | inconnus {:?}",
                first.japanese, first.unknown_words
            );
            let history = [
                turn(Role::Assistant, &first.japanese),
                turn(Role::User, "すし を 食べて います"),
            ];
            let second = next_message(&ollama, model, theme, &k, &history)
                .await
                .unwrap();
            let fr = ollama
                .translate_ja_to_fr("translategemma:4b", &second.japanese, "")
                .await
                .unwrap();
            println!(
                "IA 2 : {} => {fr} | inconnus {:?}",
                second.japanese, second.unknown_words
            );
        });
    }

    // Cas signalé le 2026-11-23 : phrase juste (kanji, sans espaces) déclarée fautive.
    // cargo test -- --ignored essai_reel_fausse_erreur --nocapture
    #[test]
    #[ignore]
    fn essai_reel_fausse_erreur() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let mut k = knowledge();
            for (kana, kanji, meaning) in [
                ("すし", None, "sushi"),
                ("たべる", Some("食べる"), "manger"),
                ("あなた", None, "toi"),
            ] {
                k.words.push(KnownWord {
                    kana: kana.into(),
                    kanji: kanji.map(String::from),
                    meanings: vec![meaning.into()],
                });
            }
            let theme = find_theme(Some("weekend")).unwrap();
            let history = [
                turn(Role::Assistant, "しゅうまつ は なに を たべます か？"),
                turn(Role::User, "すしを食べます。あなたは？"),
            ];
            let review = review_message(
                &ollama,
                &super::super::knowledge_prompt::test_model(),
                theme,
                &k,
                &history,
            )
            .await
            .unwrap();
            println!("ERREUR : {review:?}");
            assert!(review.is_none(), "fausse erreur : {review:?}");
        });
    }

    // Nécessite Ollama lancé : cargo test -- --ignored essai_reel_conversation --nocapture
    #[test]
    #[ignore]
    fn essai_reel_conversation() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
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
            // Correction de mon message, puis réponse de l'IA (comme l'écran, en parallèle).
            let first_review = review_message(&ollama, model, theme, &knowledge(), &history)
                .await
                .unwrap();
            println!("CORRECTION 1 : {first_review:?}");
            let second = next_message(&ollama, model, theme, &knowledge(), &history)
                .await
                .unwrap();
            println!("IA 2 : {second:?}");
            history.push(turn(Role::Assistant, &second.japanese));
            history.push(turn(Role::User, "おちゃ が おいしい です か"));
            let second_review = review_message(&ollama, model, theme, &knowledge(), &history)
                .await
                .unwrap();
            println!("CORRECTION 2 : {second_review:?}");
            let errors = first_review.into_iter().chain(second_review).collect();
            let summary = summarize(&ollama, model, theme, &history, errors)
                .await
                .unwrap();
            println!("BILAN : {summary:?}");
        });
    }

    // Cas signalés le 2026-11-25 (thème "Se présenter") : chaque message corrigé seul.
    // Questions de l'IA reconstituées : la vraie conversation n'était pas enregistrée.
    // cargo test -- --ignored essai_reel_presentation --nocapture
    #[test]
    #[ignore]
    fn essai_reel_presentation() {
        tauri::async_runtime::block_on(async {
            let ollama = Ollama::new().unwrap();
            let model = &super::super::knowledge_prompt::test_model();
            let theme = find_theme(Some("introduction")).unwrap();
            for (question, answer) in [
                ("おはようございます。", "おはようございます先制。"),
                ("はじめまして。わたし は たなか です。", "おなまえは？"),
                ("おくに は どちら です か？", "フランス語です。"),
                ("どこ に すんで います か？", "パリに積んでいま。"),
            ] {
                let history = [turn(Role::Assistant, question), turn(Role::User, answer)];
                let review = review_message(&ollama, model, theme, &knowledge(), &history)
                    .await
                    .unwrap();
                println!(
                    "{question} / {answer}
  => {review:?}"
                );
            }
        });
    }
}
