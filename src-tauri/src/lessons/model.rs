// Structures de données des leçons (équivalent des entités, sans ORM).

// Structs en pub(crate) : le code généré par #[tauri::command] les manipule hors de `lessons`.

// Leçon lue en base et renvoyée au front.
// Champs privés : les derive génèrent leur code ici, ils y ont donc accès.
#[derive(serde::Serialize, sqlx::FromRow)]
pub(crate) struct Lesson {
    id: i64,
    title: String,
    date: String,
    notes: Option<String>, // NULL en base => None
    created_at: String,
}

// Données envoyées par le front pour créer ou modifier une leçon.
#[derive(serde::Deserialize)]
pub(crate) struct CreateLessonInput {
    // pub(super) : lus par commands.rs, invisibles hors de `lessons`.
    pub(super) title: String,
    pub(super) date: String,
    pub(super) notes: Option<String>,
}
