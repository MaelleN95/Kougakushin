// Fonctionnalité "manuels" : regroupent des leçons (ex. "Marugoto A1").

mod commands;
mod model;
mod repository;

pub(crate) use commands::{create_book, delete_book, list_books, update_book};
