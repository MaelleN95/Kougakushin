// Commandes Tauri des particules : seul l'état "je connais" est modifiable, pas le catalogue.

use sqlx::SqlitePool;
use tauri::State;

use super::model::ParticleUsage;
use super::repository;
use crate::error::AppError;

#[tauri::command]
pub(crate) async fn list_particle_usages(
    pool: State<'_, SqlitePool>,
) -> Result<Vec<ParticleUsage>, AppError> {
    Ok(repository::find_all(pool.inner()).await?)
}

#[tauri::command]
pub(crate) async fn set_particle_usage_active(
    pool: State<'_, SqlitePool>,
    id: i64,
    active: bool,
) -> Result<ParticleUsage, AppError> {
    repository::set_active(pool.inner(), id, active)
        .await?
        .ok_or(AppError::NotFound("Emploi de particule"))
}

// Coche ou décoche d'un coup tous les emplois d'une particule ; renvoie le nombre d'emplois.
#[tauri::command]
pub(crate) async fn set_particle_active(
    pool: State<'_, SqlitePool>,
    particle: String,
    active: bool,
) -> Result<u64, AppError> {
    let count = repository::set_active_for_particle(pool.inner(), &particle, active).await?;
    if count == 0 {
        return Err(AppError::NotFound("Particule"));
    }
    Ok(count)
}
