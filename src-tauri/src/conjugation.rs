// Fonctionnalité "conjugaison" : formes verbales générées par le code (D40, D41, D42),
// formes connues de l'utilisatrice, et verbes à vérifier.

mod commands;
mod model;
mod repository;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::{
    conjugate_verb, list_conjugation_forms, list_verb_warnings, set_conjugation_form_known,
};

// Lus par `cards` (cartes de conjugaison) et `items` (connaissances pour l'IA).
pub(crate) use model::{Conjugation, VerbForm, VerbType, conjugate};
pub(crate) use repository::known_forms;
