# Kōgakushin - Spécification simplifiée (V1)

> Remplace les anciens documents (archivés dans `docs/archive/`) pour la V1.
> Principe : **le strict utile pour réviser chaque semaine ce qui a été vu en cours.** Tout le reste va dans "Plus tard".

---

## 1. Le besoin en une phrase

Chaque semaine j'apprends de nouvelles choses en cours ; je veux les saisir vite, les réviser en flashcards, puis les pratiquer avec une IA qui **ne m'utilise que ce que je connais**.

## 2. Contraintes

- Application de bureau Windows, **mono-utilisateur, hors ligne**, gratuite.
- Données locales dans un fichier SQLite (sauvegarde = copier le fichier ou exporter en JSON).
- IA locale via **Ollama** (gratuit, la machine a 8 Go de VRAM), derrière une petite interface pour pouvoir changer de moteur plus tard.
- Interface en français, contenu en japonais.

---

## 3. Fonctionnalités V1

### 3.1 Leçons

Une leçon = ce que j'ai vu à un cours.

| Champ | Obligatoire |
|---|---|
| Titre (ex. "Marugoto L5 - la famille") | oui |
| Date | oui (aujourd'hui par défaut) |
| Notes libres | non |

Une leçon regroupe des **éléments** (voir 3.2). Un élément appartient à une leçon (celle où je l'ai appris).

### 3.2 Éléments à apprendre

Deux types seulement en V1 :

**Vocabulaire**

| Champ | Obligatoire | Remarque |
|---|---|---|
| Kana | oui | ex. `たべる` |
| Romaji | non | généré automatiquement depuis les kana, modifiable |
| Kanji | non | ajoutable plus tard sur la même fiche (`食べる`) |
| Traduction française | oui | un ou plusieurs sens, un champ par sens (liste en base, D43) |
| Type | non | nom / verbe-ru / verbe-u / verbe irrégulier / adj-i / adj-na / autre |
| Exemple + traduction | non | |

**Point de grammaire** (inclut particules, formes verbales, expressions)

| Champ | Obligatoire | Remarque |
|---|---|---|
| Titre | oui | ex. `〜ています`, `は vs が`, `forme て` |
| Explication | oui | texte libre en français |
| Exemples | non | |

Chaque élément a un booléen **actif** (par défaut oui). Un élément inactif est exclu des révisions et de l'IA, sans être supprimé.

Écrans : liste filtrable (par leçon, type, texte), formulaire d'ajout **rapide** (saisie en rafale : Entrée = ajouter et rester sur le formulaire).

### 3.3 Flashcards (répétition espacée)

- Algorithme **FSRS** (bibliothèque existante, on ne le réimplémente pas).
- Cartes générées automatiquement à partir du vocabulaire :
  1. Français → japonais (réponse : kana ou kanji)
  2. Japonais (kana) → français
  3. Kanji → lecture + sens (seulement si le kanji est renseigné)
- Chaque carte a son propre suivi FSRS.
- Session : "cartes à réviser aujourd'hui" + au plus **N nouvelles** par jour (N réglable, 10 par défaut). Option : réviser une leçon précise.
- Je révèle la réponse, puis je note : **À revoir / Difficile / Bien / Facile**.
- Fin de session : nombre de cartes, répartition des notes.

Les points de grammaire ne sont pas en flashcards en V1 (ils servent au contexte de l'IA).

### 3.4 Traduction (IA)

1. Je choisis : toutes mes connaissances, ou une/des leçons ; sens FR→JP ou JP→FR.
2. L'IA reçoit **mon vocabulaire actif + mes points de grammaire actifs** et génère une phrase qui n'utilise que ça (en priorisant les éléments récents ou mal notés).
3. Je tape ma traduction.
4. L'IA corrige : correct / presque / faux, la bonne version et une courte explication en français.
5. Phrase suivante.

Garde-fou : après génération, l'application **signale les mots de la phrase absents de ma base** (vérification simple côté code) ; si trop d'intrus, on régénère.

### 3.5 Conversation (IA)

1. Je choisis un thème (ou "surprise") et les leçons à utiliser.
2. L'IA joue un personnage dans une mise en situation simple et me parle **uniquement en japonais**, avec mon vocabulaire et ma grammaire.
3. Je réponds en japonais (clavier IME). Je peux demander **un indice** ou **une traduction** de son dernier message.
4. Bouton "Terminer" → bilan en français : mes erreurs corrigées, ce qui était bien, 2-3 éléments à revoir.

Chaque message est corrigé en arrière-plan juste après l'envoi (D34) ; le bilan rassemble ces corrections. Les kanji absents de ma base sont signalés sous mon message (D35, erreur de conversion du clavier probable).

La conversation n'est pas sauvegardée en base ; elle est écrite dans le journal local pour le diagnostic (D36).

### 3.6 Réglages

- Modèle Ollama utilisé + bouton "tester la connexion".
- Nombre de nouvelles cartes par jour.
- Afficher/masquer le romaji.
- Export / import JSON de toutes les données.

---

## 4. V2 (validée, après la V1)

Démarrage après quelques jours d'usage réel de la V1 (D25). Ordre de réalisation (D26) :

1. Thème "Nuit japonaise" soigné (palette et typos de `docs/archive/`), furigana.
2. Conjugaisons : génération algorithmique des formes verbales (en Rust) + flashcards de grammaire.
3. Statistiques et suivi de régularité.
4. Fiches kanji dédiées (lectures on/kun, vocabulaire lié).
5. Sauvegarde automatique vers un dossier choisi, par exemple synchronisé (D27).
6. Peut-être : audio, lecture TTS locale, puis réponse orale (Whisper). À décider plus tard.

**Abandonnés** (trop coûteux pour le gain en usage perso) : comptes/authentification, API REST locale, référentiels JLPT et évaluation initiale, Google Drive OAuth, file d'attente IA avec priorités, collections (remplacées par le filtre par leçon).

---

## 5. Architecture technique

```
┌──────────────── Tauri 2 (fenêtre Windows) ────────────────┐
│  React + TypeScript + Vite + Tailwind                      │
│      │  invoke("commande", args)                           │
│      ▼                                                     │
│  Rust : commandes Tauri                                    │
│      ├── SQLite (sqlx ou rusqlite) + migrations            │
│      ├── FSRS                                              │
│      └── client HTTP → Ollama (http://localhost:11434)     │
└────────────────────────────────────────────────────────────┘
```

- **Objectif secondaire : apprendre Rust.** Toute la logique métier vit côté Rust (base de données, génération des cartes, FSRS, appels IA, vérification des mots intrus, import/export). Le front React ne fait que l'affichage et la saisie.
- Crates pressenties : `sqlx` (SQLite, migrations), `rs-fsrs` (planificateur FSRS léger), `reqwest` (Ollama), `serde` / `serde_json`, `thiserror` (erreurs), `chrono`.
- **Pas d'API Axum** : les commandes Tauri suffisent, et suppriment tout le volet sécurité réseau (CORS, CSRF, ports).
- **Pas de comptes** : un seul fichier `kougakushin.db` dans le dossier de données de l'application.
- Romaji ↔ kana : bibliothèque `wanakana` côté front pour la saisie en direct (le Rust stocke ce qu'on lui envoie).
- IA : un module `ai` avec une fonction par usage (`generer_phrase`, `corriger_traduction`, `repondre_conversation`, `bilan_conversation`), chacune avec son prompt et une **sortie JSON** validée.

### 5.1 Modèle de données

```sql
book      (id, title UNIQUE, created_at)              -- manuel, ex. "Marugoto A1"
lesson    (id, book_id NULL, title, date, notes, created_at)
item      (id, lesson_id, kind ['vocab'|'grammar'], active,
           kana, romaji, kanji, meaning, word_type,       -- vocab
           title, explanation,                            -- grammar
           example_jp, example_fr, created_at, updated_at)
card      (id, item_id, direction ['fr_jp'|'jp_fr'|'kanji_read'],
           next_review_at, stability, difficulty, elapsed_days, scheduled_days,
           reps, lapses, state, last_review, created_at)
review_log(id, card_id, rating, state, elapsed_days, scheduled_days, reviewed_at)
setting   (key, value)
```

Source de vérité : les migrations de `src-tauri/migrations/`.

### 5.2 Choix du modèle IA

Choix retenu après essais (réglable dans les Réglages) :

- Modèle principal : `gemma4:e4b-it-q4_K_M` (génération, correction, conversation), sur la carte graphique. `qwen3.5:9b` et `gemma4:12b-it-qat` ont été écartés : moins bons en conversation, et trop lents.
- Traducteur japonais vers français : `translategemma:4b`, sur le processeur, pour que les deux modèles restent chargés.

---

## 6. Ordre de réalisation

| Étape | Livrable | Utilisable ? |
|---|---|---|
| 1 | Projet Tauri + React qui s'ouvre, SQLite + migrations | - |
| 2 | Leçons + vocabulaire + grammaire (CRUD, ajout rapide) | ✅ je peux saisir mes cours |
| 3 | Flashcards FSRS | ✅ je peux réviser |
| 4 | Connexion Ollama + test des modèles | - |
| 5 | Exercice de traduction | ✅ |
| 6 | Conversation | ✅ V1 complète |
| 7 | Export/import JSON, finitions | ✅ V1 terminée (2026-11-24) |

Chaque étape doit être utilisable avant de passer à la suivante.
