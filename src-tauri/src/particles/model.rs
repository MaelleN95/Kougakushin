// Structures de données des emplois de particules.

// Emploi lu en base et renvoyé au front.
#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub(crate) struct ParticleUsage {
    id: i64,
    pub(super) particle: String, // lu par les tests du repository
    usage: String,
    explanation: String,
    example_jp: String,
    example_fr: String,
    pub(super) active: bool, // lu par les tests du repository
    position: i64,
}

// Emploi coché "je connais", transmis aux consignes de l'IA.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow)]
pub(crate) struct KnownParticleUsage {
    pub(crate) particle: String,
    pub(crate) usage: String,
}
