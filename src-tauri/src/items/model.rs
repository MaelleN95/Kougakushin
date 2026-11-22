// Structures de données des éléments : un enum porte les champs propres à chaque type.

use crate::error::AppError;

// Nature grammaticale d'un mot de vocabulaire.
// Copy : un simple "numéro de variante", on le copie au lieu de l'emprunter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")] // RuVerb <=> "ru_verb" en JSON
pub(crate) enum WordType {
    Noun,
    RuVerb,
    UVerb,
    IrregularVerb,
    IAdjective,
    NaAdjective,
    Other,
}

impl WordType {
    // Valeur stockée en base (doit correspondre au CHECK de la migration).
    pub(super) fn as_str(self) -> &'static str {
        match self {
            WordType::Noun => "noun",
            WordType::RuVerb => "ru_verb",
            WordType::UVerb => "u_verb",
            WordType::IrregularVerb => "irregular_verb",
            WordType::IAdjective => "i_adjective",
            WordType::NaAdjective => "na_adjective",
            WordType::Other => "other",
        }
    }

    // "ru_verb" => Some(WordType::RuVerb), texte inconnu => None.
    fn from_db(value: &str) -> Option<WordType> {
        match value {
            "noun" => Some(WordType::Noun),
            "ru_verb" => Some(WordType::RuVerb),
            "u_verb" => Some(WordType::UVerb),
            "irregular_verb" => Some(WordType::IrregularVerb),
            "i_adjective" => Some(WordType::IAdjective),
            "na_adjective" => Some(WordType::NaAdjective),
            "other" => Some(WordType::Other),
            _ => None,
        }
    }
}

// Ce qui change selon le type d'élément. En JSON : { "kind": "vocab", "kana": ..., ... }
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ItemContent {
    Vocab {
        kana: String,
        romaji: Option<String>,
        kanji: Option<String>,
        meaning: String,
        word_type: Option<WordType>,
    },
    Grammar {
        title: String,
        explanation: String,
    },
}

// Élément renvoyé au front.
#[derive(serde::Serialize)]
pub(crate) struct Item {
    id: i64,
    lesson_id: i64,
    active: bool,
    content: ItemContent,
    example_jp: Option<String>,
    example_fr: Option<String>,
    created_at: String,
    updated_at: String,
}

// Ligne brute de la table `item` : une colonne = un champ, tout ce qui peut être NULL est Option.
// Ne sort jamais du module : on la convertit en `Item` avec `into_item`.
#[derive(Debug, sqlx::FromRow)]
pub(super) struct ItemRow {
    id: i64,
    lesson_id: i64,
    kind: String,
    active: bool,
    kana: Option<String>,
    romaji: Option<String>,
    kanji: Option<String>,
    meaning: Option<String>,
    word_type: Option<String>,
    title: Option<String>,
    explanation: Option<String>,
    example_jp: Option<String>,
    example_fr: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ItemRow {
    // Id de la ligne, pour générer les cartes avant la conversion en Item.
    pub(super) fn id(&self) -> i64 {
        self.id
    }

    // Ligne plate => Item typé. Un kind inconnu ou un champ obligatoire absent => InvalidData.
    pub(super) fn into_item(self) -> Result<Item, AppError> {
        // Seul `content` dépend du type : le match ne produit que lui.
        let content = match self.kind.as_str() {
            "vocab" => ItemContent::Vocab {
                kana: self.kana.ok_or(AppError::InvalidData("kana"))?,
                romaji: self.romaji,
                kanji: self.kanji,
                meaning: self.meaning.ok_or(AppError::InvalidData("meaning"))?,
                // None reste None ; un texte inconnu est une erreur, pas un None silencieux.
                // transpose : Option<Result<T, E>> -> Result<Option<T>, E>, pour pouvoir utiliser `?`.
                word_type: self
                    .word_type
                    .map(|s| WordType::from_db(&s).ok_or(AppError::InvalidData("word_type")))
                    .transpose()?,
            },
            "grammar" => ItemContent::Grammar {
                title: self.title.ok_or(AppError::InvalidData("title"))?,
                explanation: self
                    .explanation
                    .ok_or(AppError::InvalidData("explanation"))?,
            },
            _ => return Err(AppError::InvalidData("kind")),
        };

        Ok(Item {
            id: self.id,
            lesson_id: self.lesson_id,
            active: self.active,
            content,
            example_jp: self.example_jp,
            example_fr: self.example_fr,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

// Type d'élément, pour filtrer la liste.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ItemKind {
    Vocab,
    Grammar,
}

impl ItemKind {
    // Valeur de la colonne `kind` (doit correspondre au CHECK de la migration).
    pub(super) fn as_str(self) -> &'static str {
        match self {
            ItemKind::Vocab => "vocab",
            ItemKind::Grammar => "grammar",
        }
    }
}

// Critères de `list_items` ; un champ absent = pas de filtre sur ce critère.
// default : `{}` est accepté. deny_unknown_fields : une faute de frappe ("lessonId") est
// refusée au lieu d'être ignorée en silence (ce qui renverrait tout sans filtrer).
#[derive(Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ItemFilter {
    pub(super) lesson_id: Option<i64>,
    pub(super) kind: Option<ItemKind>,
    pub(super) active: Option<bool>,
    pub(super) search: Option<String>, // cherché dans tous les champs texte
}

// Données envoyées par le front pour créer ou modifier un élément.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateItemInput {
    pub(super) lesson_id: i64,
    pub(super) content: ItemContent,
    pub(super) example_jp: Option<String>,
    pub(super) example_fr: Option<String>,
}

impl CreateItemInput {
    // Refuse les champs obligatoires vides (espaces seuls compris).
    pub(super) fn validate(&self) -> Result<(), AppError> {
        match &self.content {
            ItemContent::Vocab { kana, meaning, .. } => {
                if kana.trim().is_empty() {
                    return Err(AppError::Validation("Le kana ne peut pas être vide"));
                }
                if meaning.trim().is_empty() {
                    return Err(AppError::Validation("La traduction ne peut pas être vide"));
                }
            }
            ItemContent::Grammar { title, explanation } => {
                if title.trim().is_empty() {
                    return Err(AppError::Validation(
                        "Le titre du point de grammaire ne peut pas être vide",
                    ));
                }
                if explanation.trim().is_empty() {
                    return Err(AppError::Validation("L'explication ne peut pas être vide"));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{ItemContent, ItemRow, WordType};
    use crate::error::AppError;

    // Ligne de vocabulaire valide, à modifier selon le cas testé.
    fn vocab_row() -> ItemRow {
        ItemRow {
            id: 1,
            lesson_id: 1,
            kind: "vocab".into(),
            active: true,
            kana: Some("たべる".into()),
            romaji: None,
            kanji: None,
            meaning: Some("manger".into()),
            word_type: Some("ru_verb".into()),
            title: None,
            explanation: None,
            example_jp: None,
            example_fr: None,
            created_at: "2026-11-22 00:00:00".into(),
            updated_at: "2026-11-22 00:00:00".into(),
        }
    }

    #[test]
    fn as_str_et_from_db_sont_inverses() {
        for word_type in [
            WordType::Noun,
            WordType::RuVerb,
            WordType::UVerb,
            WordType::IrregularVerb,
            WordType::IAdjective,
            WordType::NaAdjective,
            WordType::Other,
        ] {
            assert_eq!(WordType::from_db(word_type.as_str()), Some(word_type));
        }
    }

    #[test]
    fn convertit_une_ligne_de_vocabulaire() {
        let item = vocab_row().into_item();
        assert!(matches!(
            item.map(|i| i.content),
            Ok(ItemContent::Vocab {
                word_type: Some(WordType::RuVerb),
                ..
            })
        ));
    }

    #[test]
    fn word_type_absent_reste_none() {
        let row = ItemRow {
            word_type: None,
            ..vocab_row()
        };
        assert!(matches!(
            row.into_item().map(|i| i.content),
            Ok(ItemContent::Vocab {
                word_type: None,
                ..
            })
        ));
    }

    #[test]
    fn word_type_inconnu_est_une_erreur() {
        let row = ItemRow {
            word_type: Some("verbe".into()),
            ..vocab_row()
        };
        assert!(matches!(
            row.into_item(),
            Err(AppError::InvalidData("word_type"))
        ));
    }

    #[test]
    fn kind_inconnu_est_une_erreur() {
        let row = ItemRow {
            kind: "kanji".into(),
            ..vocab_row()
        };
        assert!(matches!(
            row.into_item(),
            Err(AppError::InvalidData("kind"))
        ));
    }

    #[test]
    fn champ_obligatoire_absent_est_une_erreur() {
        let row = ItemRow {
            meaning: None,
            ..vocab_row()
        };
        assert!(matches!(
            row.into_item(),
            Err(AppError::InvalidData("meaning"))
        ));
    }
}

#[cfg(test)]
mod deserialization_tests {
    use super::{CreateItemInput, ItemFilter};

    #[test]
    fn filtre_vide_accepte() {
        assert!(serde_json::from_str::<ItemFilter>("{}").is_ok());
    }

    #[test]
    fn champ_inconnu_refuse() {
        // Faute de frappe camelCase dans le filtre.
        assert!(serde_json::from_str::<ItemFilter>(r#"{"lessonId": 1}"#).is_err());
        // Champ de grammaire glissé dans un vocabulaire.
        let input = r#"{"lesson_id": 1, "example_jp": null, "example_fr": null,
            "content": {"kind": "vocab", "kana": "た", "romaji": null, "kanji": null,
                        "meaning": "x", "word_type": null, "title": "intrus"}}"#;
        assert!(serde_json::from_str::<CreateItemInput>(input).is_err());
    }
}
