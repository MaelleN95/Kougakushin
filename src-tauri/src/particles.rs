// Fonctionnalité "particules" : catalogue des emplois de particules (D37 A), fourni par la
// migration, avec un interrupteur "je connais" par emploi.

mod commands;
mod model;
mod repository;

// Commandes pour `generate_handler!` dans lib.rs.
pub(crate) use commands::{list_particle_usages, set_particle_active, set_particle_usage_active};

// Lus par `items::knowledge` pour les consignes et le garde-fou de l'IA (D38 A).
pub(crate) use model::KnownParticleUsage;
pub(crate) use repository::{find_known_usages, find_unknown_particles};
