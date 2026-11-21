# 📜 Carnet de quêtes Rust & TypeScript

## Rang actuel : 五級 (5e kyū) - 205 XP

Un rang exige **les deux** conditions : l'XP **et** un nombre de notions au niveau requis.

| Rang | XP | Notions requises |
|---|---|---|
| 五級 5e kyū | 0 | - |
| 四級 4e kyū | 300 | 3 notions 🥈 |
| 三級 3e kyū | 700 | 6 notions 🥈 dont 2 🥇 |
| 二級 2e kyū | 1 200 | 10 notions 🥈 dont 5 🥇 |
| 一級 1er kyū | 1 800 | toutes les notions Rust 🥈 dont 8 🥇 |
| 初段 Shodan - "je lis du Rust sans peur" | 2 500 | toutes les notions 🥇 |

## Barème XP

L'XP d'une quête dépend de **l'aide reçue**, évaluée objectivement par Claude à la validation.

| Niveau d'aide | Définition | XP Quête (base 100) | XP Duo (base 50) |
|---|---|---|---|
| **Autonome** | Seulement l'énoncé + la doc officielle + les messages du compilateur | 100 % | 100 % |
| **Aide légère** | 1 à 2 indices ciblés ("regarde le type de `.len()`") | 70 % | 70 % |
| **Aide forte** | Fiche de syntaxe, erreurs pointées ligne par ligne, plusieurs relectures correctives | 40 % | 40 % |
| **Solution donnée** | Claude a fourni le code | 10 % | 10 % |

Ce qui **compte comme de l'aide** : toute information de Claude au-delà de l'énoncé, y compris les relectures d'étape qui pointent des erreurs précises.
Ce qui **ne compte pas** : lire la doc officielle, le Rust Book, les messages de `cargo` / `tsc` / `clippy`.

| Bonus | XP |
|---|---|
| 🧹 `cargo clippy` + `tsc` propres au premier passage de relecture | +10 |
| 🎯 Question de compréhension : réponse juste **et complète** | +15 |
| 🧠 Interro de rappel : question juste et complète (voir ci-dessous) | +20 |
| 🧠 Interro de rappel : juste mais incomplète | +10 |
| ⚡ Défi de lecture (après un Express) : juste et complet | +20 |
| ⚡ Défi de lecture : juste mais incomplet ou imprécis | +10 |

## Maîtrise des notions

Chaque notion progresse en trois niveaux. **Une seule rencontre ne suffit jamais.**

| Niveau | Critère objectif |
|---|---|
| 🥉 **Découvert** | Utilisée une fois, avec ou sans aide |
| 🥈 **Pratiqué** | Réutilisée **seule** dans une quête ultérieure, sans indice sur cette notion |
| 🥇 **Maîtrisé** | Interro de rappel réussie **sans notes** au moins 7 jours après le 🥈, **et** capable d'expliquer la notion avec ses mots |

### Interros de rappel 🧠
Au début de certaines quêtes, Claude pose 1 à 3 questions courtes sur des notions déjà vues, **sans prévenir et sans notes** (répétition espacée, comme l'app qu'on construit). Une réponse fausse ne fait pas perdre de niveau, mais bloque la montée au niveau suivant jusqu'à la prochaine interro.

### Notions Rust

| Notion | Niveau | Dernière preuve |
|---|---|---|
| 🔤 UTF-8 - octets vs caractères, `.len()` / `.chars()` | 🥉 | Quête 1 (question réussie) |
| 📦 Serde - `#[derive(Serialize)]` | 🥉 | Quête 3a (`Deserialize` pour la struct d'entrée, avec aide) |
| 🧱 Struct - déclarer, instancier | 🥉 | Quête 1 (avec fiche) |
| 📝 Syntaxe de base - `let`, types, retour implicite | 🥉 | Quête 1 (avec fiche) |
| 🏠 Ownership - `String` / `&str`, emprunt | 🥉 | Quête 1 (notion expliquée, non comprise seule) |
| ❓ Option & Result | 🥉 | Quête 3a (`fetch_optional` + `ok_or_else`, solution donnée ; `expect` sur `Option` corrigé) |
| 🔀 Enum & match | - | |
| ⏳ Async / await | 🥉 | Phase 2 (`.await` dans une commande, avec aide) |
| 🗄️ SQLx & migrations | 🥉 | Quête 3a (`RETURNING` transposé seule dans `create`, `rows_affected`) |
| 🧩 Modules | 🥉 | Phase 2 (`mod`, `use lessons::list_lessons`) |
| 🧪 Tests | - | |
| 🎭 Traits | - | |
| 🌐 HTTP (`reqwest`) | - | |

### Notions TypeScript

| Notion | Niveau | Dernière preuve |
|---|---|---|
| 🏷️ Types & génériques - `type`, `useState<T>`, `invoke<T>` | 🥉 | Quête 1 (avec fiche) |
| ❔ Null & narrowing - `T \| null`, `&&` | 🥉 | Quête 1 (avec explication) |
| 🧷 Unions & littéraux | - | |
| 📐 Props typées | - | |

## Journal

| Date | Quête | Mode | Aide | XP | Notes |
|---|---|---|---|---|---|
| 2026-11-19 | Quête 1 - `analyser_mot` | ⚔️ Quête | **Forte** (fiche Rust, fiche TS, 3 relectures correctives) | 40 + 15 (question UTF-8) = **55** | Clippy non propre au 1er passage. Question `&str` / `String` non trouvée. |
| 2026-11-20 | Défi de lecture - hook de pré-commit | ⚡ Express | Aucune | **10** (partiel) | Parcours juste. `-D warnings` imprécis (manque : avertissement → erreur → code de sortie ≠ 0) ; lien code de sortie / annulation du commit par Git non expliqué. |
| 2026-11-20 | Interro de rappel n°1 | 🧠 | Aucune | 10 + 20 = **30** | Q1 UTF-8 : fait juste (9 octets) mais cause du plantage absente (découpe au milieu d'un caractère). Q2 `T \| null` : juste et complète. |
| 2026-11-21 | Phase 2 - SQLite et `list_lessons` | 🤝 Duo | **Forte** (erreurs sqlx décryptées, deux structs, `&**pool`) | 50 x 40 % = **20** | Migration et commande fonctionnelles. 2 relectures supplémentaires pour fmt et un commentaire faux. |
| 2026-11-21 | Défi de lecture - erreur dans `db::init` | 🤝 Duo | Aucune | **0** | Déroulement normal décrit au lieu du cas d'échec ; `app_data_dir` et `create_if_missing` mal compris. |
| 2026-11-21 | Interro de rappel n°2 | 🧠 | Aucune | 10 + 20 = **30** | Q1 `map_err` : rôle juste mais conséquence absente (sans lui, le `?` ne compile pas). Q2 `pub` : juste. |
| 2026-11-21 | Quête 3a - CRUD des leçons | ⚔️ Quête | **Solution donnée** (vérification de l'id dans `update`, demandée explicitement) | 100 x 10 % = **10** | Tests console OK. 5 relectures correctives : paramètres à plat, booléen bindé à la place du titre, `expect`, validation après l'écriture, vérification d'id inutile. `RETURNING` transposé seule dans `create`. |
| 2026-11-21 | Interro de rappel n°3 | 🧠 | Aucune | 20 + 10 = **30** | Q1 UTF-8 : juste et complète (6 / 9 octets, taille par type de caractère). Q2 async : "promise" au lieu de `Future`, paresse des futures ignorée (rien ne s'exécute sans `.await`), cause de l'erreur de compilation absente (`map_err` n'existe pas sur un `Future`). |
| 2026-11-21 | Quête 3b étape 1 - découpage de `lessons` en couches | ⚔️ Quête | **Solution donnée** (visibilité, nommage et commentaires corrigés par Claude à sa demande) | 100 x 10 % = **10** | Couches et tests console justes seule. Visibilité trop large (`pub` partout, `pub use` du modèle), nommage `repository::list_lessons`, en-têtes absents. Passage en mode "Claude écrit et explique" pour la soirée. |
| 2026-11-21 | Défis de lecture - visibilité dans `lessons` | ⚡ Express | Aucune | 0 + 10 = **10** | Q1 fausse : `pub(super)` attribué au front (c'est `commands.rs` qui lit les champs ; `Deserialize` est généré dans `model.rs`), cas de `Lesson` non trouvé. Q2 partielle : ré-export cité, mais pas la condition (l'item lui-même doit être `pub(crate)`). |
| 2026-11-21 | Quête 3b étapes 2-3 - `AppError`, `validate()`, date stricte, erreurs SQL masquées | ⚡ Express (écrit par Claude, à sa demande) | - | **0** | Tests console (8/8) et unitaires (4/4) conformes. Défis de lecture sur `#[from]` et la construction de `CreateLessonInput` encore ouverts. |

## Quêtes à venir

| Phase | Quête | Mode prévu |
|---|---|---|
| 1 | ~~`analyser_mot` - première commande Tauri~~ ✅ | ⚔️ Quête |
| 2 | ~~Brancher SQLite et créer la table `lesson`~~ ✅ | 🤝 Duo |
| 3a | ~~CRUD des leçons (`create`, `update`, `delete`)~~ ✅ | ⚔️ Quête |
| 3b | ~~Couches, erreurs propres (`enum` + `thiserror`) et validation factorisée~~ ✅ | ⚔️ Quête puis ⚡ Express |
| 4 | Vocabulaire et grammaire (`enum`, `Option`) | ⚔️ Quête |
| 4 | Écrans React de saisie | ⚡ Express |
| 5 | Flashcards FSRS + tests unitaires | ⚔️ Quête |
| 6 | Client Ollama | 🤝 Duo |
| 7 | Exercice de traduction | 🤝 Duo |
| 8 | Conversation | 🤝 Duo |
| 9 | Export / import JSON | ⚔️ Quête |
