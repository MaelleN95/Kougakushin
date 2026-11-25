// Accès SQL au catalogue des particules. Renvoie les erreurs sqlx telles quelles.

use sqlx::SqlitePool;

use super::model::{KnownParticleUsage, ParticleUsage};

// Tous les emplois, dans l'ordre d'affichage.
pub(super) async fn find_all(pool: &SqlitePool) -> Result<Vec<ParticleUsage>, sqlx::Error> {
    sqlx::query_as::<_, ParticleUsage>(
        "SELECT id, particle, usage, explanation, example_jp, example_fr, active, position
         FROM particle_usage ORDER BY position",
    )
    .fetch_all(pool)
    .await
}

// Coche ou décoche un emploi ; None si l'id n'existe pas.
pub(super) async fn set_active(
    pool: &SqlitePool,
    id: i64,
    active: bool,
) -> Result<Option<ParticleUsage>, sqlx::Error> {
    sqlx::query_as::<_, ParticleUsage>(
        "UPDATE particle_usage SET active = ? WHERE id = ?
         RETURNING id, particle, usage, explanation, example_jp, example_fr, active, position",
    )
    .bind(active)
    .bind(id)
    .fetch_optional(pool)
    .await
}

// Coche ou décoche tous les emplois d'une particule ; renvoie le nombre d'emplois concernés.
pub(super) async fn set_active_for_particle(
    pool: &SqlitePool,
    particle: &str,
    active: bool,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE particle_usage SET active = ? WHERE particle = ?")
        .bind(active)
        .bind(particle)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

// Emplois cochés, dans l'ordre d'affichage (consignes de l'IA).
pub(crate) async fn find_known_usages(
    pool: &SqlitePool,
) -> Result<Vec<KnownParticleUsage>, sqlx::Error> {
    sqlx::query_as::<_, KnownParticleUsage>(
        "SELECT particle, usage FROM particle_usage WHERE active = 1 ORDER BY position",
    )
    .fetch_all(pool)
    .await
}

// Particules du catalogue dont aucun emploi n'est coché (garde-fou de l'IA).
pub(crate) async fn find_unknown_particles(pool: &SqlitePool) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT particle FROM particle_usage
         GROUP BY particle HAVING MAX(active) = 0
         ORDER BY MIN(position)",
    )
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use sqlx::SqlitePool;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    #[test]
    fn catalogue_complet_et_tout_desactive_au_depart() {
        tauri::async_runtime::block_on(async {
            let pool = pool().await;
            let all = super::find_all(&pool).await.unwrap();
            assert_eq!(all.len(), 60);
            assert!(all.iter().all(|u| !u.active));
            // Ordre d'affichage : は en premier, など en dernier.
            assert_eq!(all[0].particle, "は");
            assert_eq!(all[59].particle, "など");
            assert!(super::find_known_usages(&pool).await.unwrap().is_empty());
            let unknown = super::find_unknown_particles(&pool).await.unwrap();
            assert_eq!(unknown.len(), 22);
            assert_eq!(unknown[..3], ["は", "が", "を"]);
        });
    }

    #[test]
    fn une_particule_est_connue_des_qu_un_emploi_est_coche() {
        tauri::async_runtime::block_on(async {
            let pool = pool().await;
            // 11 : に, destination.
            let usage = super::set_active(&pool, 11, true).await.unwrap().unwrap();
            assert!(usage.active);
            assert!(super::set_active(&pool, 999, true).await.unwrap().is_none());

            let known = super::find_known_usages(&pool).await.unwrap();
            assert_eq!(known.len(), 1);
            assert_eq!(
                (known[0].particle.as_str(), known[0].usage.as_str()),
                ("に", "Destination")
            );
            let unknown = super::find_unknown_particles(&pool).await.unwrap();
            assert!(!unknown.iter().any(|p| p == "に"));
            assert!(unknown.iter().any(|p| p == "は"));
        });
    }

    #[test]
    fn tout_cocher_ou_decocher_une_particule() {
        tauri::async_runtime::block_on(async {
            let pool = pool().await;
            assert_eq!(
                super::set_active_for_particle(&pool, "で", true)
                    .await
                    .unwrap(),
                5
            );
            assert_eq!(super::find_known_usages(&pool).await.unwrap().len(), 5);
            super::set_active_for_particle(&pool, "で", false)
                .await
                .unwrap();
            assert!(super::find_known_usages(&pool).await.unwrap().is_empty());
            assert_eq!(
                super::set_active_for_particle(&pool, "ぞ", true)
                    .await
                    .unwrap(),
                0
            );
        });
    }
}
