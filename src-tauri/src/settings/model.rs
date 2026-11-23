// Réglages typés, valeurs par défaut et conversion depuis / vers les lignes clé / valeur.

use crate::error::AppError;

// Clés stockées en base. Une clé inconnue en base est ignorée (réglage retiré d'une version).
const NEW_CARDS_PER_DAY: &str = "new_cards_per_day";
const SHOW_ROMAJI: &str = "show_romaji";
const OLLAMA_MODEL: &str = "ollama_model";
const TRANSLATION_MODEL: &str = "translation_model";

// Modèle retenu après le comparatif du 2026-11-23 (consignes respectées, corrections justes).
const DEFAULT_OLLAMA_MODEL: &str = "gemma4:e4b-it-q4_K_M";
// Traducteur dédié japonais / français (D22), retenu après mesure du 2026-11-23.
const DEFAULT_TRANSLATION_MODEL: &str = "translategemma:4b";

// Bornes du nombre de nouvelles cartes par jour.
const MAX_NEW_CARDS_PER_DAY: i64 = 100;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Settings {
    pub(crate) new_cards_per_day: i64, // lu par `cards`
    pub(super) show_romaji: bool,
    pub(crate) ollama_model: String,      // lu par `ai`
    pub(crate) translation_model: String, // lu par `ai`
}

// Valeurs utilisées tant que l'utilisatrice n'a rien modifié.
impl Default for Settings {
    fn default() -> Self {
        Settings {
            new_cards_per_day: 10,
            show_romaji: true,
            ollama_model: DEFAULT_OLLAMA_MODEL.to_string(),
            translation_model: DEFAULT_TRANSLATION_MODEL.to_string(),
        }
    }
}

impl Settings {
    pub(super) fn validate(&self) -> Result<(), AppError> {
        if !(0..=MAX_NEW_CARDS_PER_DAY).contains(&self.new_cards_per_day) {
            return Err(AppError::Validation(
                "Le nombre de nouvelles cartes par jour doit être entre 0 et 100",
            ));
        }
        if self.ollama_model.trim().is_empty() {
            return Err(AppError::Validation("Le modèle d'IA ne peut pas être vide"));
        }
        if self.translation_model.trim().is_empty() {
            return Err(AppError::Validation(
                "Le modèle de traduction ne peut pas être vide",
            ));
        }
        Ok(())
    }

    // Lignes de la table => réglages ; une clé absente garde sa valeur par défaut.
    pub(super) fn from_pairs(pairs: Vec<(String, String)>) -> Result<Settings, AppError> {
        let mut settings = Settings::default();
        for (key, value) in pairs {
            match key.as_str() {
                NEW_CARDS_PER_DAY => {
                    settings.new_cards_per_day = value
                        .parse()
                        .map_err(|_| AppError::InvalidData("new_cards_per_day"))?;
                }
                SHOW_ROMAJI => {
                    settings.show_romaji = value
                        .parse()
                        .map_err(|_| AppError::InvalidData("show_romaji"))?;
                }
                OLLAMA_MODEL => settings.ollama_model = value,
                TRANSLATION_MODEL => settings.translation_model = value,
                _ => {} // clé inconnue : ignorée
            }
        }
        Ok(settings)
    }

    // Réglages => lignes à enregistrer.
    pub(super) fn to_pairs(&self) -> [(&'static str, String); 4] {
        [
            (NEW_CARDS_PER_DAY, self.new_cards_per_day.to_string()),
            (SHOW_ROMAJI, self.show_romaji.to_string()),
            (OLLAMA_MODEL, self.ollama_model.clone()),
            (TRANSLATION_MODEL, self.translation_model.clone()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::Settings;

    #[test]
    fn aller_retour_et_valeurs_par_defaut() {
        let settings = Settings {
            new_cards_per_day: 25,
            show_romaji: false,
            ollama_model: "autre:modele".to_string(),
            translation_model: "autre:traducteur".to_string(),
        };
        let pairs = settings
            .to_pairs()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        assert_eq!(Settings::from_pairs(pairs).ok(), Some(settings));
        assert_eq!(
            Settings::from_pairs(Vec::new()).ok(),
            Some(Settings::default())
        );
    }

    #[test]
    fn valeur_illisible_ou_hors_bornes() {
        let pairs = vec![("show_romaji".to_string(), "oui".to_string())];
        assert!(Settings::from_pairs(pairs).is_err());
        let too_many = Settings {
            new_cards_per_day: 101,
            ..Settings::default()
        };
        assert!(too_many.validate().is_err());
    }
}
