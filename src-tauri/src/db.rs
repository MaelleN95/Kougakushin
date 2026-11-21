// Module "db" : ouverture de la base SQLite et application des migrations.
// Équivalent Symfony : la configuration Doctrine (DATABASE_URL) + `doctrine:migrations:migrate`.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use tauri::{AppHandle, Manager};

// Ouvre (ou crée) la base dans le dossier de données de l'application,
// applique les migrations en attente, puis renvoie le pool de connexions.
//
// `async` : ouvrir un fichier et exécuter du SQL prend du temps, on attend sans bloquer.
// `Result<…, Box<dyn Error>>` : renvoie soit le pool, soit "n'importe quelle erreur".
pub async fn init(app: &AppHandle) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    // Dossier propre à l'application, dérivé de l'identifier : %APPDATA%\com.kougakushin.app\
    // Le `?` à la fin : si la ligne échoue, on sort immédiatement de la fonction avec l'erreur.
    let dir = app.path().app_data_dir()?;
    // Crée le dossier et ses parents s'ils n'existent pas, et ne fait rien s'ils existent déjà. C'est l'équivalent de mkdir -p
    std::fs::create_dir_all(&dir)?;

    let options = SqliteConnectOptions::new()
        .filename(dir.join("kougakushin.db"))
        .create_if_missing(true);

    // Crée un pool de connexions SQLite, qui sera partagé entre toutes les commandes Tauri.
    // Un pool est un objet qui garde plusieurs connexions ouvertes à la base, pour ne pas avoir à les rouvrir à chaque requête.
    let pool = SqlitePool::connect_with(options).await?;

    // Les fichiers SQL de src-tauri/migrations/ sont intégrés au binaire à la compilation,
    // puis exécutés ici dans l'ordre ; ceux déjà appliqués sont ignorés (comme Doctrine).
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
