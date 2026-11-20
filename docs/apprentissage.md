# 📜 Carnet de quêtes Rust & TypeScript

## Rang actuel : 五級 (5e kyū) — 55 XP

Un rang exige **les deux** conditions : l'XP **et** un nombre de notions au niveau requis.

| Rang | XP | Notions requises |
|---|---|---|
| 五級 5e kyū | 0 | — |
| 四級 4e kyū | 300 | 3 notions 🥈 |
| 三級 3e kyū | 700 | 6 notions 🥈 dont 2 🥇 |
| 二級 2e kyū | 1 200 | 10 notions 🥈 dont 5 🥇 |
| 一級 1er kyū | 1 800 | toutes les notions Rust 🥈 dont 8 🥇 |
| 初段 Shodan — « je lis du Rust sans peur » | 2 500 | toutes les notions 🥇 |

## Barème XP

L'XP d'une quête dépend de **l'aide reçue**, évaluée objectivement par Claude à la validation.

| Niveau d'aide | Définition | XP Quête (base 100) | XP Duo (base 50) |
|---|---|---|---|
| **Autonome** | Seulement l'énoncé + la doc officielle + les messages du compilateur | 100 % | 100 % |
| **Aide légère** | 1 à 2 indices ciblés (« regarde le type de `.len()` ») | 70 % | 70 % |
| **Aide forte** | Fiche de syntaxe, erreurs pointées ligne par ligne, plusieurs relectures correctives | 40 % | 40 % |
| **Solution donnée** | Claude a fourni le code | 10 % | 10 % |

Ce qui **compte comme de l'aide** : toute information de Claude au-delà de l'énoncé, y compris les relectures d'étape qui pointent des erreurs précises.
Ce qui **ne compte pas** : lire la doc officielle, le Rust Book, les messages de `cargo` / `tsc` / `clippy`.

| Bonus | XP |
|---|---|
| 🧹 `cargo clippy` + `tsc` propres au premier passage de relecture | +10 |
| 🎯 Question de compréhension : réponse juste **et complète** | +15 |
| 🧠 Interro de rappel réussie (voir ci-dessous) | +20 |

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
| 🔤 UTF-8 — octets vs caractères, `.len()` / `.chars()` | 🥉 | Quête 1 (question réussie) |
| 📦 Serde — `#[derive(Serialize)]` | 🥉 | Quête 1 (avec fiche) |
| 🧱 Struct — déclarer, instancier | 🥉 | Quête 1 (avec fiche) |
| 📝 Syntaxe de base — `let`, types, retour implicite | 🥉 | Quête 1 (avec fiche) |
| 🏠 Ownership — `String` / `&str`, emprunt | 🥉 | Quête 1 (notion expliquée, non comprise seule) |
| ❓ Option & Result | — | |
| 🔀 Enum & match | — | |
| ⏳ Async / await | — | |
| 🗄️ SQLx & migrations | — | |
| 🧩 Modules | — | |
| 🧪 Tests | — | |
| 🎭 Traits | — | |
| 🌐 HTTP (`reqwest`) | — | |

### Notions TypeScript

| Notion | Niveau | Dernière preuve |
|---|---|---|
| 🏷️ Types & génériques — `type`, `useState<T>`, `invoke<T>` | 🥉 | Quête 1 (avec fiche) |
| ❔ Null & narrowing — `T \| null`, `&&` | 🥉 | Quête 1 (avec explication) |
| 🧷 Unions & littéraux | — | |
| 📐 Props typées | — | |

## Journal

| Date | Quête | Mode | Aide | XP | Notes |
|---|---|---|---|---|---|
| 2026-11-19 | Quête 1 — `analyser_mot` | ⚔️ Quête | **Forte** (fiche Rust, fiche TS, 3 relectures correctives) | 40 + 15 (question UTF-8) = **55** | Clippy non propre au 1er passage. Question `&str` / `String` non trouvée. |

## Quêtes à venir

| Phase | Quête | Mode prévu |
|---|---|---|
| 1 | ~~`analyser_mot` — première commande Tauri~~ ✅ | ⚔️ Quête |
| 2 | Brancher SQLite et créer la table `lesson` | 🤝 Duo |
| 3 | CRUD des leçons + erreurs propres | ⚔️ Quête |
| 4 | Vocabulaire et grammaire (`enum`, `Option`) | ⚔️ Quête |
| 4 | Écrans React de saisie | ⚡ Express |
| 5 | Flashcards FSRS + tests unitaires | ⚔️ Quête |
| 6 | Client Ollama | 🤝 Duo |
| 7 | Exercice de traduction | 🤝 Duo |
| 8 | Conversation | 🤝 Duo |
| 9 | Export / import JSON | ⚔️ Quête |
