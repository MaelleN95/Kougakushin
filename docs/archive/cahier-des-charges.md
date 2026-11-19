# Cahier des charges
## Application locale d'apprentissage et de révision du japonais

**Version :** 1.1  
**Statut :** Cahier des charges consolidé  
**Nature du projet :** Side project open source  
**Environnement cible :** Windows

---

## 1. Présentation générale

### 1.1 Contexte

Le projet consiste à développer une application personnelle permettant d'apprendre et de réviser le japonais à partir d'un parcours individualisé.

Les outils d'apprentissage existants ne répondent pas nécessairement à tous les besoins d'organisation, de mémorisation et de personnalisation.

L'application doit permettre de centraliser les connaissances, construire ses propres cours, organiser ses révisions et pratiquer le japonais avec une intelligence artificielle locale.

### 1.2 Finalité

Créer un environnement complet d'apprentissage du japonais, utilisable gratuitement et localement, sans dépendance obligatoire à des services cloud.

L'application doit être suffisamment structurée pour être utilisée quotidiennement et suffisamment générique pour accueillir ultérieurement d'autres langues.

### 1.3 Nature du projet

Le projet est :
- Personnel dans son usage initial.
- Public sur GitHub.
- Open source.
- Ouvert aux contributions communautaires.
- Destiné à constituer un projet de portfolio professionnel.

Kōgakushin est une application de bureau Windows installable. Elle ne sera pas déployée sur un serveur public dans sa première version.

---

## 2. Objectifs

### 2.1 Objectifs principaux

1. Centraliser toutes les connaissances linguistiques dans une bibliothèque personnelle.
2. Permettre de construire un parcours d'apprentissage personnalisé.
3. Automatiser la répétition espacée.
4. Générer des exercices adaptés aux connaissances acquises.
5. Permettre des conversations immersives avec une IA locale.
6. Évaluer le niveau initial de l'utilisateur.
7. Mesurer les progrès dans le temps.
8. Garantir la sauvegarde et la restauration des données.
9. Proposer une expérience cohérente, immersive et accessible.

### 2.2 Objectifs secondaires

- Démontrer la maîtrise d'une architecture frontend/backend séparée.
- Expérimenter l'intégration d'une IA locale.
- Concevoir une architecture extensible à d'autres langues.
- Développer un système de données linguistiques structuré.
- Publier un projet documenté et maintenable sur GitHub.

---

## 3. Périmètre fonctionnel

### 3.1 Fonctionnalités incluses

| Domaine | Fonctionnalités |
|---|---|
| Comptes | Création, connexion, récupération, modification et suppression |
| Profil | Pseudo, mot de passe, photo, question de sécurité |
| Connaissances | Création, consultation, modification, archivage |
| Cours | Création, association, modification, archivage |
| Collections | Création, gestion et regroupement libre |
| Flashcards | Révisions, FSRS, sessions et bilans |
| Exercices | Génération IA, correction et progression |
| Conversations | Conversation libre, scénario et jeu de rôle |
| Évaluation | Identification des connaissances initiales |
| Référentiels | Import local, validation et mise à jour GitHub |
| Tableau de bord | Synthèse des activités et objectifs |
| Progression | Statistiques, graphiques, niveaux et régularité |
| Audio | TTS et STT locaux |
| IA | Ollama local et génération contrôlée |
| Sauvegardes | Google Drive, restauration, import et export |
| Paramètres | Apparence, apprentissage, audio, IA et sauvegardes |

### 3.2 Fonctionnalités exclues de la première version

- Apprentissage d'autres langues que le japonais.
- Synchronisation automatique multi-appareils.
- Déploiement public sur un serveur.
- Interactions sociales entre utilisateurs.
- Classements et compétitions.
- Partage de connaissances entre comptes.
- Catalogue manuel de scénarios de conversation.
- Évaluation phonétique détaillée.
- Audio préintégré dans les référentiels.
- Éditeur de référentiels directement dans l'application.
- Système de contributions linguistiques intégré.
- Notifications globales.

### 3.3 Évolutivité

L'architecture doit permettre :
- L'ajout d'autres langues.
- Le remplacement du moteur IA.
- Le remplacement des moteurs audio.
- L'ajout ultérieur de la synchronisation multi-appareils.
- L'évolution des référentiels sans casser les données personnelles.

---

## 4. Contraintes techniques

### 4.1 Stack imposée

- Tauri 2 pour l'application desktop.
- React et TypeScript pour l'interface, notamment issue de Figma Make.
- Vite.
- Tailwind CSS.
- Rust pour le backend.
- Axum et Tokio pour l'API REST JSON locale.
- SQLite avec SQLx.
- Serde pour la sérialisation.
- Ollama installé séparément.
- Whisper.cpp pour la reconnaissance vocale.
- Moteur TTS japonais local à sélectionner.
- FSRS pour la répétition espacée.

Docker peut être utilisé pour certains besoins de développement, mais n'est pas nécessaire pour installer ou exécuter l'application distribuée.

### 4.2 Environnement

Développement initial sous Windows avec VS Code et les prérequis Tauri, Rust et Node.js. Docker Desktop et WSL2 restent facultatifs.

Configuration matérielle disponible :
- 32 Go de RAM.
- GPU NVIDIA avec 8 Go de VRAM.
- Intel Core i9-14900HX.

L'interface React est affichée dans une fenêtre native grâce à WebView2. L'API Axum est lancée localement par l'application et limitée à `127.0.0.1`. Elle ne doit pas être accessible depuis le réseau local.

L'installateur comprend l'interface, le programme Rust et les composants locaux nécessaires. Ollama reste installé séparément ; les modèles sont obtenus à la demande et jamais téléchargés automatiquement sans accord explicite. Les moteurs audio sont prévus comme programmes indépendants embarqués, sous réserve de validation des licences, des modèles et du packaging. WebView2 est un prérequis Windows à prendre en charge lors de l'installation.

### 4.3 Coûts

L'objectif est un fonctionnement entièrement gratuit.

Sont exclus :
- API IA payantes.
- Services de génération vocale payants.
- Services de reconnaissance vocale payants.
- Hébergement cloud applicatif payant.
- Dépendances obligatoires à des abonnements.

Google Drive est utilisé pour les sauvegardes via le compte personnel de chaque utilisateur.

---

## 5. Contraintes pédagogiques

### 5.1 Source de vérité

La base de connaissances personnelle constitue la seule source de vérité pour les activités d'apprentissage quotidiennes.

Les référentiels linguistiques servent exclusivement à l'évaluation initiale.

Les connaissances doivent être :
- Ajoutées manuellement.
- Ou proposées après évaluation initiale puis validées individuellement.

### 5.2 Sécurité pédagogique

L'IA ne doit jamais introduire de connaissance non étudiée ou non autorisée pendant :
- Les flashcards.
- Les exercices.
- Les conversations.

Les résultats générés doivent être vérifiés.

Une correction incertaine ne doit pas modifier automatiquement la maîtrise d'une connaissance.

### 5.3 Statuts

Les statuts de maîtrise sont :
- Nouveau.
- En apprentissage.
- Maîtrisé.

La disponibilité pédagogique est indépendante :
- Active.
- Non appris.

Une connaissance non apprise est exclue des activités de révision, mais reste conservée.

### 5.4 Référentiels JLPT

Les référentiels sont communautaires et non officiels.

Leur contenu doit être :
- Sourcé.
- Vérifié.
- Légalement redistribuable.
- Versionné.
- Organisé par niveau et catégorie.

Les niveaux N5 et N4 sont prioritaires.

---

## 6. Exigences de sécurité et de confidentialité

### 6.1 Données personnelles

Les données restent locales.

Les comptes sont isolés et aucune interaction entre utilisateurs n'est prévue.

SQLite n'est pas chiffrée par une couche supplémentaire. Toute personne ayant accès aux fichiers de l'installation peut potentiellement consulter les données.

### 6.2 Authentification

- Aucun email obligatoire.
- Aucun mot de passe en clair.
- Question de sécurité hashée.
- Limitation des tentatives.
- Protection contre l'énumération des comptes.
- Sessions non persistantes.
- Vérification des droits à chaque requête.
- Protection de l'API locale contre les requêtes non autorisées, y compris depuis un navigateur.

### 6.3 Audio et IA

- IA locale.
- TTS local.
- STT local.
- Détection d'Ollama et indication de son état ; aide à l'installation ou au démarrage s'il est indisponible.
- Vérification du modèle sélectionné, sans téléchargement automatique.
- Microphone actif uniquement pendant l'enregistrement.
- Suppression des fichiers audio temporaires.
- Pas de conservation des conversations complètes.

### 6.4 Sauvegardes

Les sauvegardes contiennent les données nécessaires à la restauration, mais jamais les secrets d'authentification ou tokens OAuth.

Les utilisateurs doivent être informés que les sauvegardes Drive ne bénéficient pas d'un chiffrement supplémentaire par l'application.

---

## 7. Exigences ergonomiques et graphiques

### 7.1 Direction artistique

Univers "Nuit japonaise".

Ambiance :
- Cosy.
- Apaisante.
- Immersive.
- Minimaliste.
- Motivante.

### 7.2 Identité visuelle

Palette sombre :
- #19172E.
- #373052.
- #66577C.
- #D18B65.
- #F0E5DC.

Palette claire :
- #F0E5DC.
- #E5D8D5.
- #D2C2C8.
- #A96848.
- #19172E.

Typographies :
- Inter.
- Zen Kaku Gothic New.

Le logo doit être un symbole géométrique indépendant inspiré des motifs japonais, accompagné du nom de l'application.

Le nom définitif reste à choisir.

### 7.3 Adaptation de l'interface

- Fenêtre Windows desktop prioritaire et redimensionnable.
- Interface adaptée aux petites dimensions de fenêtre et aux interactions tactiles lorsque pertinent.
- Aucune distribution mobile ou tablette distincte n'est prévue dans la première version.

### 7.4 Accessibilité

- Navigation clavier.
- Contrastes conformes WCAG AA à vérifier.
- Animations réduites ou désactivables.
- Zones tactiles adaptées.

---

## 8. Organisation et livrables

### 8.1 Documents

Les livrables documentaires comprennent :
- Spécifications fonctionnelles.
- Spécifications techniques.
- Cahier des charges.
- Documentation d'installation.
- Documentation d'utilisation.
- Documentation de développement.
- Documentation des référentiels.
- Documentation des licences.

### 8.2 Code source

Le code doit être hébergé sur GitHub.

Le dépôt doit contenir :
- Frontend.
- Backend.
- Configuration de développement, avec Docker facultatif si nécessaire.
- Schémas JSON.
- Référentiels linguistiques.
- Tests.
- Documentation.
- Fichiers de configuration d'exemple.

Les secrets, tokens, données personnelles et fichiers locaux ne doivent pas être committés.

### 8.3 Documentation du code

Les commentaires, commits et documents de développement sont rédigés en français.

Le projet doit être compréhensible par un développeur souhaitant le cloner et le faire évoluer.

### 8.4 Configuration des assistants de développement

Le projet doit prévoir :
- Les règles de développement.
- Les conventions de code.
- Les instructions Claude Code.
- Un fichier CLAUDE.md.
- Les consignes nécessaires à GitHub Copilot pendant la phase de développement concernée.

---

## 9. Phasage prévisionnel

Le développement doit être réalisé progressivement afin de limiter les risques et de valider chaque couche avant d'ajouter la suivante.

### Phase 1 - Fondations desktop

- Initialisation GitHub.
- Fenêtre Tauri 2 sous Windows.
- React, TypeScript, Vite et Tailwind.
- Prérequis et structure du projet.

### Phase 2 - Fondations Rust et données

- Fondamentaux Rust et premiers tests.
- API REST locale avec Axum et Tokio, protégée et limitée à `127.0.0.1`.
- SQLite, SQLx et migrations.
- Détection d'Ollama et étude des moteurs audio locaux.

### Phase 3 - Comptes et sécurité

- Création des comptes.
- Authentification.
- Récupération.
- Gestion des sessions.
- Isolation des données.
- Profil.

### Phase 4 - Gestion des connaissances

- Modèles de données.
- Catégories.
- Création et modification.
- Relations.
- Cours.
- Collections.
- Archivage.

### Phase 5 - Référentiels linguistiques

- Schéma JSON.
- Référentiels N5 et N4.
- Validation.
- Chargement progressif.
- Détection des mises à jour GitHub.
- Correspondances avec les fiches personnelles.

### Phase 6 - Évaluation initiale

- Sélection des connaissances pertinentes.
- Évaluation adaptative.
- Génération des propositions.
- Validation individuelle.
- Enrichissement des fiches personnelles.

### Phase 7 - Flashcards et FSRS

- Génération des cartes.
- Sessions.
- Répétition espacée.
- Reprise.
- Bilans.

### Phase 8 - Exercices

- Génération IA.
- Correction.
- Difficulté adaptative.
- Sessions personnalisées.
- Statistiques.

### Phase 9 - Conversations

- Modes de conversation.
- Scénarios.
- Personnages.
- TTS.
- STT.
- Bilans.
- Progression.

### Phase 10 - Progression et tableau de bord

- Synthèse.
- Graphiques.
- Statistiques.
- Objectifs.
- Régularité.
- Bilans hebdomadaires.

### Phase 11 - Sauvegardes et import/export

- Google OAuth.
- Sauvegardes.
- Restauration.
- Export JSON/CSV.
- Import et fusion.
- Suppression des sauvegardes.

### Phase 12 - Finalisation et distribution

- Tests de non-régression.
- Tests de performance.
- Audit de sécurité.
- Audit des licences.
- Adaptation aux dimensions de fenêtre.
- Accessibilité.
- Installateur Windows, prise en charge de WebView2 et test sur une installation propre.
- Documentation finale.
- Publication GitHub.

Les phases peuvent être réordonnées ou partiellement parallélisées lorsque les dépendances techniques le permettent.

---

## 10. Critères de validation

Le projet sera considéré comme fonctionnellement abouti lorsque les critères suivants seront satisfaits.

### Comptes
- Plusieurs comptes peuvent être créés.
- Les données sont isolées.
- L'authentification et la récupération fonctionnent.
- La suppression locale est effective.

### Connaissances
- Toutes les catégories peuvent être gérées.
- Les relations fonctionnent.
- Les connaissances peuvent être enrichies.
- Les cours et collections sont indépendants et opérationnels.

### Révisions
- Les cartes sont générées à partir des informations disponibles.
- FSRS calcule les prochaines échéances.
- Les sessions peuvent être interrompues et reprises.
- Les statuts évoluent conformément aux résultats.

### Exercices
- Les exercices sont générés à partir des connaissances autorisées.
- Les corrections sont vérifiées.
- Les notions inconnues ne sont pas introduites.
- Les statistiques sont cohérentes.

### Conversations
- Les trois modes fonctionnent.
- Les conversations restent dans le périmètre linguistique autorisé.
- Le mode oral fonctionne hors ligne.
- Les bilans sont générés et conservés selon les règles prévues.

### Évaluation initiale
- Les référentiels sont exploités localement.
- Les connaissances sont proposées sans modification automatique.
- Les correspondances sont détectées.
- Les validations individuelles fonctionnent.
- Les connaissances existantes ne sont pas réévaluées lors d'une relance.

### Référentiels
- Les fichiers JSON sont validés.
- Les UUID et relations sont cohérents.
- Les mises à jour sont comparées.
- Les suppressions ne détruisent pas les fiches personnelles.
- Les changements de niveau sont gérés automatiquement.
- Les fichiers invalides n'empêchent pas les autres fonctionnalités.

### Sauvegardes
- Les snapshots sont créés selon les règles.
- Les échecs ne détruisent pas les sauvegardes existantes.
- La restauration fonctionne.
- Les données personnelles peuvent être exportées et importées.

### Qualité générale
- Les tests unitaires et d'intégration passent.
- Les tests de performance sont réalisés.
- Les tests de non-régression sont présents.
- L'application s'installe et se lance sous Windows sans Docker ; l'API n'est pas exposée sur le réseau local.
- L'indisponibilité d'Ollama ou d'un moteur audio est signalée sans bloquer les fonctions indépendantes.
- L'application fonctionne sans connexion Internet pour les fonctionnalités locales.
- La documentation permet l'installation et la compréhension du projet.

---

## 11. Risques et points de vigilance

### 11.1 Performances IA

La qualité des modèles locaux en japonais et leur consommation de ressources peuvent limiter certaines fonctionnalités.

Une phase de benchmark est indispensable avant de figer le modèle.

### 11.2 Fiabilité linguistique

Les référentiels ne sont pas officiels.

Les sources peuvent être incomplètes, contradictoires ou soumises à des restrictions de licence.

Une vérification humaine est indispensable.

### 11.3 Sauvegardes

La fermeture brutale du processus ne garantit pas l'exécution d'une sauvegarde à la fermeture.

Les snapshots périodiques et la sauvegarde des changements doivent limiter les pertes potentielles.

### 11.4 Isolation des données

L'isolation applicative ne protège pas contre une personne ayant accès directement aux fichiers SQLite.

Cette limite doit être documentée.

### 11.5 Cohérence pédagogique

L'IA peut produire des erreurs malgré les instructions.

Les validations doivent empêcher toute modification automatique non autorisée des connaissances.

### 11.6 Complexité

Le projet réunit plusieurs systèmes importants :
- Gestion pédagogique.
- IA locale.
- Traitement audio.
- Répétition espacée.
- Sauvegarde distante.
- Synchronisation de référentiels.

Une implémentation progressive est indispensable.

---

## 12. Éléments restant à définir techniquement

Les décisions fonctionnelles principales sont consolidées. Les points suivants nécessitent encore une étude ou un arbitrage avant ou pendant l'implémentation :

1. Nom définitif de l'application et identité du projet.
2. Modèle Ollama définitif après benchmark.
3. Moteur TTS japonais définitif.
4. Version et configuration exactes des dépendances.
5. Schéma détaillé de la base SQLite.
6. Structure JSON complète par catégorie.
7. Valeurs exactes des champs essentiels et facultatifs par catégorie.
8. Structure complète du JSON Schema.
9. Contrat détaillé des endpoints REST.
10. Seuils techniques de détection de saturation IA.
11. Paramètres FSRS complémentaires et stratégie d'initialisation.
12. Format exact de l'installateur, distribution des sidecars et prise en charge de WebView2.
13. Configuration OAuth Google adaptée à un dépôt public.
14. Stratégie de migration des données et versions de l'application.
15. Organisation des tests de performance et de validation linguistique.

Ces éléments ne remettent pas en cause les décisions fonctionnelles déjà validées. Ils constituent le travail de conception technique détaillée et de préparation à l'implémentation.

---

## 13. Conclusion

Le projet vise à fournir un environnement complet, local et personnalisable pour l'apprentissage du japonais.

La priorité est donnée à :
- La fiabilité des connaissances.
- La maîtrise des données personnelles.
- La cohérence pédagogique.
- L'autonomie hors ligne.
- La modularité technique.
- La maintenabilité.
- La gratuité.

La première version doit fournir un parcours complet allant de l'évaluation initiale à la mémorisation, la pratique, la conversation et l'analyse de la progression.

Le projet sera publié sur GitHub avec son code source et sa documentation, afin de permettre son utilisation personnelle et son évolution communautaire.