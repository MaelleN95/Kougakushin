# Spécifications techniques
## Application locale d'apprentissage et de révision du japonais

**Version :** 1.1 - Document de conception  
**Statut :** Base technique consolidée

---

## 1. Architecture générale

### 1.1 Architecture retenue

Kōgakushin est une application Windows installable, structurée en couches :

- Tauri 2 crée la fenêtre native et gère le cycle de vie de l'application et les intégrations Windows.
- React et TypeScript, affichés avec WebView2, constituent l'interface.
- Le backend Rust porte la logique métier et lance une API REST JSON locale avec Axum et Tokio.
- SQLx accède à SQLite ; Serde assure la sérialisation.
- Le backend pilote Ollama, Whisper.cpp et un moteur TTS japonais local.

L'interface et la logique métier restent séparées : les opérations métier passent par l'API REST. Une commande Tauri limitée à l'initialisation transmet à l'interface les informations nécessaires pour joindre et authentifier l'API ; elle ne remplace pas l'API métier.

### 1.2 Environnement

Développement initial :
- Windows.
- VS Code.
- Prérequis Tauri 2, Rust et Node.js.
- Git et GitHub.
- Docker Desktop et WSL2 facultatifs pour certains besoins de développement.

L'application est destinée à un fonctionnement local. Aucun serveur cloud applicatif n'est prévu.

Le serveur Axum est lancé et arrêté avec l'application et écoute uniquement sur `127.0.0.1`, jamais sur toutes les interfaces réseau. Les fonctionnalités locales ne nécessitent ni Docker ni serveur distant.

### 1.3 Principes

- Séparation stricte des responsabilités.
- API indépendante du frontend.
- Données personnelles isolées par compte.
- Services IA et audio interchangeables.
- Fonctionnement hors ligne.
- Aucun service payant obligatoire.
- Aucune dépendance des fonctionnalités essentielles à Internet.
- Les référentiels linguistiques restent indépendants de la base personnelle.

---

## 2. Technologies et dépendances

| Domaine | Technologie |
|---|---|
| Application desktop | Tauri 2 et WebView2 |
| Frontend | React et TypeScript |
| Bundler frontend | Vite |
| CSS | Tailwind CSS |
| Backend | Rust |
| API et runtime | Axum, Tokio, REST JSON et Serde |
| Base de données | SQLite avec SQLx |
| Conteneurisation | Docker facultatif en développement |
| IA locale | Ollama installé séparément |
| STT | Whisper.cpp, sidecar prévu |
| TTS | Open JTalk, Piper ou alternative à sélectionner, sidecar prévu |
| Répétition espacée | FSRS |
| Versionnement | Git / GitHub |
| Éditeur | VS Code |

La version précise du modèle Ollama, du moteur TTS et les dépendances secondaires restent à sélectionner après tests.

Le projet doit documenter les licences des bibliothèques et ressources utilisées.

---

## 3. Architecture logicielle

### 3.1 Frontend

Responsabilités :
- Interface utilisateur.
- Navigation.
- Gestion des formulaires.
- Affichage des connaissances.
- Gestion des sessions côté interface.
- Communication API.
- Affichage des résultats IA.
- Gestion des états de chargement et erreurs.
- Affichage audio et enregistrement micro.

React doit rester indépendant de la logique métier du backend.

Les composants graphiques doivent être réutilisables et organisés par domaine fonctionnel.

### 3.2 Backend Rust

Responsabilités :
- Authentification.
- Autorisations.
- Gestion des comptes.
- Gestion des connaissances.
- Gestion des cours et collections.
- Gestion des sessions d'apprentissage.
- Calcul et stockage FSRS.
- Progression et statistiques.
- Évaluation initiale.
- Accès aux référentiels.
- Orchestration IA.
- Gestion des sauvegardes.
- Import et export.
- Validation des données.

La logique métier ne doit pas dépendre des contrôleurs HTTP.

### 3.3 Organisation logique

Les domaines doivent être séparés :
- Identity.
- Learning.
- Knowledge.
- Courses.
- Collections.
- Flashcards.
- Exercises.
- Conversations.
- Assessment.
- LinguisticResources.
- Progress.
- Backup.
- AI.
- Audio.
- Settings.

Les noms définitifs et l'organisation des modules Rust seront arrêtés lors de la conception détaillée.

### 3.4 API REST

- Échanges JSON.
- Routes privées authentifiées.
- Validation backend systématique.
- Vérification de l'appartenance des données au compte connecté.
- Réponses HTTP cohérentes.
- Erreurs structurées.
- Pagination côté serveur pour les listes volumineuses.

Les méthodes et routes précises feront l'objet d'une spécification API dédiée.

---

## 4. Authentification et sessions

### 4.1 Gestion des sessions

Les sessions sont gérées côté Rust, sans JWT ni cookie. Après authentification, le backend délivre un jeton de session opaque et aléatoire, distinct du secret d'accès à l'API locale décrit en section 5.

- Les jetons sont conservés en mémoire et transmis dans des en-têtes dédiés, jamais dans les URL, les logs ou un stockage persistant du frontend.
- Le jeton de session est renouvelé après authentification et invalidé à la déconnexion.
- La session reste active tant que l'application est ouverte, sans expiration liée à l'inactivité.
- À la fermeture ou au redémarrage du backend, les sessions sont invalidées, y compris après un arrêt brutal.

### 4.2 Sécurité des mots de passe

- Hash bcrypt avec sel unique.
- Paramètres de hash gérés et versionnés par le backend Rust.
- Vérification de la longueur en octets avant hash.
- Rehash lorsque nécessaire.
- Aucun mot de passe en clair.
- Aucun mot de passe dans les exports ou sauvegardes.

Les mots de passe sont limités à 72 octets UTF-8.

### 4.3 Question de sécurité

La réponse est normalisée puis hashée avec bcrypt.

Normalisation :
- Casse insensible.
- Suppression des espaces en début et fin.
- Conservation des espaces internes.
- Conservation des accents et symboles.

La réponse est limitée à 72 octets.

### 4.4 Limitation des tentatives

Compteurs indépendants pour connexion et récupération.

- 5 échecs : blocage d'une minute.
- 10 échecs : blocage de cinq minutes.
- 15 échecs : blocage de quinze minutes.

Réinitialisation après succès.

La réponse à un pseudo inexistant doit rester neutre et ne pas permettre l'énumération des comptes. Les délais doivent être similaires.

### 4.5 Changement de mot de passe

Le changement nécessite le mot de passe actuel.

La session courante reste active. Les autres sessions sont invalidées.

Une récupération réussie invalide toutes les sessions.

---

## 5. Sécurité de l'API

### 5.1 Accès

- Écoute exclusive sur `127.0.0.1`, sur un port choisi dynamiquement par le système.
- Secret d'accès aléatoire propre au lancement, transmis avec le port à l'interface par une commande Tauri réservée à la fenêtre principale ; aucune valeur n'est inscrite dans le code ou les fichiers de configuration.
- Chaque requête API effective, y compris les routes publiques de connexion et de récupération, présente ce secret dans un en-tête dédié. Le secret reste en mémoire et n'apparaît ni dans les URL ni dans les logs.
- Toutes les routes privées nécessitent une authentification.
- Un jeton de session distinct est exigé pour les routes privées ; le secret local ne remplace pas l'authentification du compte.
- Chaque requête vérifie le compte actif.
- Chaque opération vérifie les droits et l'appartenance des données.
- Le backend est arrêté avec Tauri ; les secrets et sessions sont renouvelés à chaque lancement.

### 5.2 CSRF et CORS

L'API n'utilise pas de cookie d'authentification. Le secret local et le jeton de session sont transmis dans des en-têtes non automatiques ; les requêtes sans secret valide sont refusées. CORS est une protection complémentaire, pas une authentification :
- Autoriser uniquement les origines exactes nécessaires à la fenêtre Tauri ou au serveur Vite de développement, selon l'environnement.
- Limiter les méthodes et en-têtes autorisés ; refuser les origines non prévues et ne pas autoriser les credentials.
- Traiter les pré-vérifications `OPTIONS` sans jeton, uniquement pour les origines, méthodes et en-têtes autorisés et sans effectuer d'opération métier ; les requêtes effectives restent protégées.
- Vérifier l'en-tête `Host` attendu pour limiter les attaques de rebinding DNS.
- Tester les requêtes simples, les pré-vérifications CORS et les accès directs depuis un navigateur ou un autre processus local.

Un autre processus du même utilisateur ou une interface compromise peut contourner une partie de ces protections : l'accès au poste et la sécurité de l'interface restent des prérequis. La CSP et la réduction des permissions Tauri limitent notamment le risque de vol des jetons.

### 5.3 XSS et contenus externes

- Échappement React par défaut.
- Pas de dangerouslySetInnerHTML sans assainissement.
- Politique CSP.
- Vérification des contenus produits par l'IA avant affichage ou exploitation.

### 5.4 Limitation des requêtes

Rate limiting sur :
- Connexion.
- Récupération.
- Création de compte.
- Modification de mot de passe.
- Modification de question de sécurité.

Pas de limite artificielle sur les opérations normales.

Réponse HTTP 429 en cas de dépassement.

### 5.5 Journalisation

Les logs ne doivent contenir :
- Aucun mot de passe.
- Aucun token OAuth.
- Aucune réponse de sécurité.
- Aucune conversation complète.
- Aucune donnée personnelle inutile.

---

## 6. Base de données SQLite

### 6.1 Rôle

SQLite stocke les données personnelles et le fonctionnement de l'application.

Les référentiels linguistiques ne sont pas importés dans SQLite.

SQLx gère les accès et migrations versionnées. La base et les données modifiables sont stockées dans un répertoire de données utilisateur Windows, séparé des fichiers installés et préservé lors des mises à jour.

### 6.2 Isolation

Chaque entité personnelle est associée à un compte propriétaire.

Toute lecture ou modification doit vérifier cette appartenance.

Aucun croisement entre comptes n'est autorisé.

### 6.3 Données stockées

La base doit notamment contenir :
- Comptes.
- Données de sécurité nécessaires, hors sessions conservées en mémoire.
- Connaissances.
- Sous-entrées et sens.
- Relations entre connaissances.
- Cours.
- Collections.
- Associations.
- Historique des modifications.
- Historique de maîtrise.
- Suivi FSRS.
- Sessions en cours.
- Progression.
- Statistiques.
- Objectifs.
- Paramètres.
- File de demandes IA.
- Bilans hebdomadaires.
- État des sauvegardes.

Les conversations complètes ne sont pas archivées.

### 6.4 Transactions

Les imports et opérations impliquant plusieurs entités doivent utiliser des transactions SQLite.

En cas d'échec, rollback complet.

---

## 7. Gestion des fichiers locaux

### 7.1 Organisation

Organisation logique sous le répertoire de données de l'application, distinct du répertoire d'installation :

```text
storage/
├── users/
│   ├── [identifiant-utilisateur-1]/
│   │   ├── profile/
│   │   └── media/
│   └── [identifiant-utilisateur-2]/
│       ├── profile/
│       └── media/
└── shared/
    └── resources/
```

Les ressources partagées comprennent les référentiels et autres ressources communes.
Les ressources embarquées en lecture seule sont copiées ou mises à disposition séparément des données modifiables.

### 7.2 Quotas

- 2 Go maximum par utilisateur.
- Avertissement à 80 %.
- Refus des nouveaux fichiers au-delà du quota.
- Aucune suppression automatique de fichiers personnels.

Les fichiers TTS temporaires sont exclus du quota personnel.

### 7.3 Photos de profil

Formats :
- JPG.
- PNG.
- WebP.

Taille maximale : 5 Mo.

Traitement :
- Vérification du MIME réel.
- Décodage de l'image.
- Réencodage.
- Suppression des métadonnées EXIF, notamment GPS.
- Nom de fichier aléatoire.
- Recadrage carré.
- Redimensionnement automatique.
- Empêchement de l'exécution du fichier.

La suppression de la photo est immédiate, avec retour à l'avatar par défaut.

### 7.4 Fichiers temporaires

- Dossier distinct.
- Nettoyage après utilisation.
- Suppression des fichiers de plus de 24 heures au démarrage.
- Aucun fichier temporaire dans les sauvegardes permanentes.

### 7.5 Audio

Les enregistrements micro sont temporaires.

Ils sont supprimés après transcription et confirmation.

Le microphone est fermé dès que l'enregistrement s'arrête.

### 7.6 Cache TTS

Cache identifié par :
- Texte.
- Voix.
- Vitesse.

Une ressource identique avec les mêmes paramètres est réutilisée.

Purge après 30 jours sans utilisation.

Limite : 2 Go pour l'installation.

Si la limite est dépassée, suppression LRU (Least Recently Used), sans supprimer un fichier en cours de lecture.

Le cache est exclu des sauvegardes.

---

## 8. Référentiels linguistiques

### 8.1 Emplacement

Les référentiels sont stockés sous forme de fichiers JSON dans le dépôt du projet.

Organisation :

```text
resources/
├── schema/
│   └── reference-schema-v1.json
├── N5/
│   ├── vocabulary.json
│   ├── kanjis.json
│   ├── grammar.json
│   ├── particles.json
│   ├── conjugation.json
│   ├── expressions.json
│   └── numbers.json
├── N4/
│   ├── vocabulary.json
│   ├── kanjis.json
│   ├── grammar.json
│   ├── particles.json
│   ├── conjugation.json
│   ├── expressions.json
│   └── numbers.json
└── ...
```

Les chemins définitifs restent adaptables à l'organisation réelle du projet.

### 8.2 Format JSON

Chaque fichier contient :
- metadata.
- entries.

Chaque entrée possède un UUID v4 stable.

Les relations utilisent uniquement les UUID. L'application retrouve les fichiers concernés.

Les connaissances personnelles issues du référentiel reçoivent un nouvel identifiant personnel. L'UUID source est conservé dans un champ de référence.

### 8.3 Schéma

JSON Schema définit la structure commune à tous les fichiers.

Version du schéma globale, commune aux référentiels.

Les validations Rust complètent les contrôles structurels par des règles métier.

### 8.4 Métadonnées

Chaque fichier possède :
- Identifiant unique du fichier.
- Version du schéma.
- Niveau JLPT.
- Catégorie.
- Version du référentiel.
- Date de mise à jour.
- Sources générales.
- Licences.

### 8.5 Chargement

- Chargement au premier besoin.
- Conservation en mémoire après premier chargement.
- Cache à durée de vie configurable.
- Limitation du nombre de fichiers en mémoire.
- Libération des données inutilisées.
- Chargement progressif.
- Limitation de la taille des lots transmis à l'IA.

Les connaissances pertinentes sont sélectionnées pour chaque étape de l'évaluation initiale.

Aucun référentiel complet n'est systématiquement transmis à l'IA.

### 8.6 Erreurs

Un fichier invalide ou illisible est ignoré.

L'application :
- Signale l'erreur.
- Continue avec les autres fichiers disponibles.
- Ne bloque pas les fonctionnalités indépendantes.

Une catégorie ou un niveau absent est ignoré pendant l'évaluation initiale. Les autres domaines restent utilisables.

### 8.7 Mise à jour

Vérification GitHub au démarrage.

Aucune installation automatique.

La comparaison affiche le nombre d'ajouts, modifications et suppressions.

L'utilisateur confirme l'application.

Déplacement d'une entrée vers un autre niveau JLPT :
- Automatique.
- UUID conservé.

Les fiches personnelles ne sont jamais modifiées automatiquement par une mise à jour du référentiel.

---

## 9. Architecture de l'intelligence artificielle

### 9.1 Ollama

Ollama est installé et exécuté séparément sur Windows. Le backend interroge son API locale, par défaut sur `http://127.0.0.1:11434`, sans l'exposer au réseau.

Au démarrage, il vérifie la disponibilité du service et récupère les modèles installés s'il répond. L'interface signale une indisponibilité et propose une aide à l'installation ou au démarrage. Le modèle sélectionné est vérifié avant utilisation. Aucun modèle n'est téléchargé automatiquement sans accord explicite ; leur taille et les ressources nécessaires sont indiquées avant un éventuel téléchargement.

Le modèle n'est pas encore arrêté.

Modèles envisagés pour comparaison :
- Qwen 3.5 4B.
- Qwen 3.5 9B.
- Gemma 3 4B.

Le choix définitif dépendra de tests en japonais, de la consommation mémoire et de la qualité pédagogique.

### 9.2 Abstraction

Une couche d'abstraction doit permettre de remplacer le moteur IA sans modifier les fonctionnalités métier.

Elle doit isoler :
- Communication avec le moteur.
- Paramètres.
- Gestion des modèles.
- Génération.
- Annulation.
- Gestion des erreurs.
- Vérification des résultats.

### 9.3 Sécurité pédagogique

Le backend constitue la source de vérité.

Avant génération :
- Sélection des connaissances autorisées.
- Vérification de leur disponibilité.
- Exclusion des connaissances "Non appris".
- Construction d'un contexte limité au besoin.

Après génération :
- Vérification des contenus.
- Rejet des notions non autorisées.
- Rejet des résultats non conformes aux règles pédagogiques.

Le prompt seul ne constitue pas une garantie suffisante.

L'IA n'est jamais l'autorité finale en japonais.

Les réponses attendues, variantes acceptées et corrections doivent être contrôlées par des données validées.

### 9.4 Paramètres

L'utilisateur peut choisir :
- Un modèle Ollama installé.
- Température.
- Longueur maximale.

Les règles pédagogiques de sécurité ne sont pas modifiables.

### 9.5 Compatibilité des modèles

Après une mise à jour manuelle d'un modèle :
- Test obligatoire avant réutilisation.
- Vérification du fonctionnement.
- Vérification du japonais.
- Vérification des règles pédagogiques.
- Vérification de la stabilité.
- Mesure du temps.
- Estimation RAM/VRAM.

Un modèle incompatible est signalé avec explication.

L'ancien modèle peut être sélectionné manuellement. Aucun rollback automatique.

### 9.6 Ressources matérielles

Configuration de référence :
- 32 Go de RAM.
- GPU NVIDIA avec 8 Go de VRAM.
- Intel Core i9-14900HX.

Si les ressources estimées sont insuffisantes, avertissement avant utilisation.

En cas de saturation détectée pendant une génération :
- Interruption.
- Suppression des résultats partiels.
- Message explicatif.

Les seuils précis de détection restent à déterminer lors des tests.

---

## 10. Gestion des demandes IA

### 10.1 File

- Une génération active.
- Dix demandes en attente maximum.
- Position visible.
- Annulation possible.
- Expiration après sept jours.

Chaque demande conserve uniquement :
- Identifiants.
- Paramètres.
- Références nécessaires.

Les données pédagogiques sont récupérées depuis SQLite au moment de l'exécution.

### 10.2 Priorités

Si la file est pleine :
- Remplacer la demande la moins prioritaire si elle est strictement moins prioritaire.
- En cas d'égalité, remplacer la plus ancienne.
- Refuser si aucune demande moins prioritaire n'existe.

### 10.3 Annulation et erreur

Annulation :
- Arrêt immédiat.
- Libération de la ressource.
- Suppression des résultats partiels.
- Passage à la demande suivante.

Erreur :
- Une tentative.
- Suppression des résultats partiels.
- Nouvelle tentative manuelle.

### 10.4 Ollama indisponible

Après deux minutes sans réponse ou progression, le service est déclaré indisponible.

La file est suspendue.

Au retour :
- Vérification du service.
- Vérification du modèle.
- Requête de test.
- Reprise automatique si les conditions sont réunies.

Après cinq échecs consécutifs, suspension durable jusqu'à intervention manuelle.

### 10.5 Persistance

La file est stockée dans SQLite.

À la fermeture, elle est sauvegardée.

Au lancement :
- Vérification des demandes.
- Vérification du compte propriétaire.
- Vérification de la compatibilité.
- Suppression des demandes obsolètes.
- Reprise selon priorité, puis ancienneté.

Lors d'un changement de compte, la file est isolée par utilisateur. La génération active est interrompue, les demandes non commencées sont conservées.

---

## 11. Audio

### 11.1 TTS

Synthèse vocale japonaise locale.

Moteurs envisagés :
- Open JTalk.
- Piper.
- Autre moteur japonais compatible.

Le moteur définitif sera choisi après tests de qualité, licences, performances et fonctionnement hors ligne.

L'intégration initiale privilégie un sidecar embarqué et piloté par le backend Rust. Le binaire, ses ressources, ses voix et leurs licences doivent être vérifiés avant distribution ; les éléments non embarqués sont documentés et leur téléchargement éventuel requiert un accord explicite. L'indisponibilité du moteur est signalée sans bloquer les fonctions indépendantes.

Le TTS doit prendre en charge :
- Prononciation des mots.
- Prononciation des phrases.
- Conversations.
- Vitesse normale et ralentie.
- Voix sélectionnable.
- Cache local.

Aucun fichier audio n'est préintégré aux référentiels.

### 11.2 STT

Whisper.cpp local, prévu comme sidecar embarqué et piloté par le backend Rust. Le modèle de transcription nécessaire, sa licence, son emplacement et sa méthode d'installation restent à valider ; aucun téléchargement automatique sans accord explicite.

Fonctions :
- Transcription japonaise.
- Enregistrement manuel.
- Confirmation de la transcription.
- Correction éventuelle avant envoi.

Le système ne doit pas assimiler automatiquement une erreur de transcription à une erreur linguistique.

Les sidecars sont lancés avec des chemins maîtrisés et des arguments séparés, sans interprétation par un shell. Leur cycle de vie, leurs erreurs, leur arrêt et leurs fichiers temporaires sont gérés par l'application.

### 11.3 Évaluation phonétique

L'évaluation phonétique détaillée est reportée à une phase ultérieure.

Elle devra distinguer :
- Morae.
- Voyelles longues.
- Consonnes doubles.
- Pitch accent.

Whisper.cpp ne constitue pas à lui seul un outil de mesure phonétique.

---

## 12. Sauvegardes Google Drive

### 12.1 Connexion

Chaque compte utilisateur connecte son propre compte Google.

Les tokens OAuth sont conservés dans le gestionnaire d'identifiants Windows.

Ils ne sont jamais enregistrés dans :
- SQLite.
- Configuration versionnée.
- Exports.
- Sauvegardes.
- Logs.

Les secrets OAuth ne sont jamais committés dans GitHub.

Le parcours OAuth doit être adapté à une application desktop publique, sans secret client considéré comme confidentiel dans le binaire ; le mécanisme exact et les autorisations minimales restent à définir.

### 12.2 Stockage distant

Dossier visible dans Google Drive :

Sauvegardes - [Nom application]

Nommage :
sauvegarde_YYYY-MM-DD_HH-mm.json

Un snapshot est un fichier distinct. Il n'est pas écrasé.

Les métadonnées incluent l'identifiant du compte concerné.

### 12.3 Rétention

- 30 derniers snapshots non protégés.
- Snapshots protégés jamais supprimés automatiquement.
- Protection individuelle.
- Suppression manuelle des snapshots protégés après confirmation.
- Les sauvegardes manuelles ne sont pas protégées par défaut.

Après création d'un nouveau snapshot :
1. Vérification de son exploitabilité.
2. Suppression des anciennes sauvegardes non protégées si nécessaire.

En cas d'échec, les anciennes sauvegardes restent intactes.

### 12.4 Déconnexion

La déconnexion Drive :
- Conserve les fichiers distants.
- Supprime les tokens locaux.
- Arrête les sauvegardes automatiques.

La reconnexion peut se faire avec le même compte Google ou un autre.

### 12.5 Suppression du compte

Suppression Drive séparée de la suppression locale.

Toutes les sauvegardes sont concernées, y compris protégées.

La révocation OAuth est tentée.

---

## 13. Import et export

### 13.1 Formats

- JSON.
- CSV.

### 13.2 Validation

Contrôles :
- Structure.
- Types.
- Formats.
- Tailles.
- Références.
- Compatibilité de version.
- Doublons.

Aucun code importé ne doit être exécuté.

### 13.3 Limites

- JSON : 200 Mo.
- CSV : 100 Mo.

Contrôle de taille avant lecture.

Pas d'import partiel.

### 13.4 Fusion

La fusion utilise les règles de correspondance déjà définies.

Les doublons importés remplacent les données existantes.

Une transaction garantit l'intégrité. Toute erreur entraîne un rollback.

---

## 14. Logs et diagnostic

Logs techniques conservés 30 jours.

Ils peuvent inclure :
- Erreurs de services.
- Échecs de génération.
- Erreurs de sauvegarde.
- Erreurs d'intégration.
- Événements techniques importants.

Aucune conversation complète ni donnée sensible ne doit être journalisée.

Les demandes IA annulées ou supprimées ne disposent pas d'un historique dédié.

---

## 15. Tests

### 15.1 Tests unitaires

À prévoir pour :
- Règles métier.
- Calculs de progression.
- Statuts.
- Conjugaison.
- FSRS.
- Validation des référentiels.
- Correspondances et doublons.
- Gestion des permissions.

### 15.2 Tests d'intégration

À prévoir pour :
- API.
- SQLite.
- Authentification.
- Sessions.
- Import/export.
- Google Drive.
- Ollama.
- TTS et STT.
- Chargement des référentiels.

### 15.3 Tests de performance

Sur les référentiels complets :
- Chargement.
- Recherche.
- Cache.
- Sélection de connaissances.
- Préparation des lots IA.
- Utilisation mémoire.

### 15.4 Non-régression

Tests obligatoires lors des mises à jour :
- Schémas JSON.
- Référentiels.
- Versions.
- Relations.
- Données personnelles.
- Compatibilité des modèles.

### 15.5 Desktop et sécurité locale

- Démarrage et arrêt de Tauri et de l'API, y compris après arrêt brutal.
- Refus des requêtes sans secret local ou sans session valide ; isolation entre comptes.
- Contrôle des origines, de l'en-tête `Host` et des requêtes depuis un navigateur.
- Gestion de l'absence d'Ollama, de modèle ou de sidecar, sans blocage des fonctions indépendantes.
- Installation et lancement sur Windows propre, sans Docker, avec prise en charge de WebView2.

---

## 16. Déploiement et distribution

Le projet est open source et hébergé sur GitHub.

Il n'est pas destiné à être déployé sur un serveur public dans sa première version.

Le fonctionnement initial est local sous Windows.

Tauri produit un installateur Windows comprenant l'interface React et le backend Rust. Les sidecars audio sont prévus dans la distribution après validation de leurs licences et de leur packaging. SQLite est intégrée au backend ; Ollama reste installé séparément et les modèles IA ne sont pas inclus par défaut.

WebView2 est nécessaire à l'affichage. Son installation ou sa détection doit être prise en charge et documentée, y compris pour un poste qui ne le possède pas. Le mode d'installation exact et les besoins éventuels de connexion restent à définir.

Docker peut faciliter certains environnements de développement, mais n'est pas requis pour installer ni utiliser l'application distribuée.

La procédure d'installation, de lancement, d'arrêt et de mise à jour doit être documentée, ainsi que l'installation d'Ollama, des modèles et des éventuelles ressources audio. Les données utilisateur doivent être conservées lors des mises à jour ; le comportement à la désinstallation reste à définir.

La synchronisation multi-appareils est hors périmètre de la première version, mais l'architecture doit rester compatible avec son ajout futur.

