# Spécifications fonctionnelles
## Application locale d'apprentissage et de révision du japonais

**Version :** 1.1 - Document de conception  
**Statut :** Base fonctionnelle consolidée  
**Périmètre :** Première version de l'application

---

## 1. Présentation du projet

### 1.1 Objectif

Développer une application locale permettant d'organiser, d'apprendre et de réviser le japonais à partir d'un programme personnel.

L'application doit réunir :
- La gestion des connaissances linguistiques.
- La création de cours et de collections personnalisés.
- Les flashcards et la répétition espacée.
- Les exercices adaptatifs.
- Les conversations avec une intelligence artificielle locale.
- L'évaluation initiale du niveau de japonais.
- Le suivi détaillé de la progression.
- La sauvegarde et la restauration des données.

L'application doit fonctionner sans abonnement, sans API payante et sans serveur distant.

Kōgakushin est une application de bureau Windows installable. Son interface React s'affiche dans une fenêtre native Tauri ; les données et traitements métier restent locaux. Docker n'est pas nécessaire à son utilisation.

### 1.2 Public cible

**Persona principal :** autodidacte souhaitant construire et suivre son propre programme d'apprentissage.

**Persona secondaire :** développeur ou passionné de langues souhaitant installer le projet, configurer son propre parcours et éventuellement contribuer au dépôt open source.

### 1.3 Principes fondamentaux

- Le japonais est la seule langue d'apprentissage disponible dans la première version.
- L'architecture doit permettre d'ajouter ultérieurement d'autres langues.
- L'application est utilisable hors ligne.
- L'intelligence artificielle fonctionne localement.
- Les fonctionnalités essentielles ne dépendent pas de l'IA.
- Les données personnelles restent isolées entre les comptes.
- Aucune connaissance n'est ajoutée ou modifiée sans validation de l'utilisateur.
- Les référentiels linguistiques servent exclusivement à l'évaluation initiale.
- Les révisions, exercices et conversations utilisent exclusivement les connaissances personnelles validées.

### 1.4 Installation et services locaux

- L'installateur fournit l'application et ses composants intégrés ; WebView2 est pris en charge comme prérequis Windows.
- Ollama est installé séparément. Au démarrage, l'application vérifie sa disponibilité et, s'il répond, récupère les modèles installés.
- Si Ollama est indisponible, son état est affiché avec une aide pour l'installer ou le démarrer. Les fonctions qui n'en dépendent pas restent utilisables.
- Le modèle sélectionné est vérifié avant utilisation. Aucun modèle n'est téléchargé automatiquement : un téléchargement éventuel nécessite l'accord explicite de l'utilisateur.
- Whisper.cpp et le moteur TTS sont prévus comme composants locaux indépendants fournis avec l'application, sous réserve de validation du packaging et des licences. Les modèles et voix nécessaires ainsi que leur installation sont précisés avant distribution.
- Les sauvegardes Google Drive et les recherches de mises à jour nécessitent Internet ; leur indisponibilité ne bloque pas les fonctions locales.

---

## 2. Gestion des comptes

### 2.1 Création d'un compte

La création d'un compte nécessite :
- Un pseudo unique sur l'installation.
- Un mot de passe saisi deux fois.
- Une question de sécurité.
- Une réponse à cette question.

L'adresse email n'est pas demandée.

Le pseudo :
- Est unique sans distinction de casse.
- Comprend entre 3 et 20 caractères.
- Accepte les lettres A-Z, chiffres, tirets et underscores.
- N'accepte ni espaces, ni accents, ni symboles.
- Ne commence ni ne termine par un tiret ou underscore.
- Peut être modifié sans mot de passe ni délai.

Le mot de passe :
- Comprend au minimum 8 caractères.
- Est soumis à un contrôle de robustesse.
- Accepte les phrases de passe et tous les caractères.
- N'est pas modifié par suppression automatique des espaces.
- Est limité à 72 octets UTF-8 pour la compatibilité bcrypt.

### 2.2 Authentification

- Connexion obligatoire à chaque lancement.
- Pas de connexion automatique persistante.
- Déconnexion manuelle disponible.
- Fermeture de l'application mettant fin à la session.
- Pas de verrouillage automatique après inactivité.
- Changement d'utilisateur après déconnexion et reconnexion.
- Données conservées après déconnexion.

### 2.3 Récupération du compte

La récupération repose sur :
1. La saisie du pseudo.
2. L'affichage de la question de sécurité.
3. La saisie de la réponse.
4. La vérification de la réponse.
5. La réinitialisation du mot de passe.

Une seule question de sécurité est utilisée. Elle peut être prédéfinie ou personnalisée.

La réponse est stockée sous forme de hash. La vérification normalise la casse et les espaces périphériques, sans supprimer les accents ou symboles.

Après récupération réussie :
- Toutes les sessions sont invalidées.
- Une nouvelle connexion est nécessaire.

### 2.4 Suppression du compte

La suppression locale nécessite :
- Le mot de passe actuel.
- Un récapitulatif des conséquences.
- Une confirmation explicite.
- La saisie du pseudo pour confirmer la suppression.

La suppression locale est définitive.

La suppression des sauvegardes Google Drive est une opération distincte, avec confirmation supplémentaire. Si Drive est indisponible, la suppression locale continue et l'utilisateur est averti que les sauvegardes distantes peuvent subsister.

---

## 3. Gestion des connaissances

### 3.1 Catégories

Les connaissances sont réparties en sept catégories :
- Vocabulaire.
- Kanjis.
- Grammaire.
- Particules.
- Conjugaison.
- Expressions.
- Nombres : comptage, compteurs et calendrier.

### 3.2 Structure générale

Une connaissance possède une fiche principale pouvant contenir plusieurs sous-entrées indépendantes pour les différents sens et usages.

Chaque connaissance possède :
- Un identifiant.
- Une catégorie.
- Un niveau JLPT indicatif éventuel.
- Des informations linguistiques propres à sa catégorie.
- Des relations avec d'autres connaissances.
- Un statut de maîtrise.
- Une disponibilité pédagogique.
- Un historique de modifications et d'évaluations.
- Des associations facultatives avec des cours et collections.

Une connaissance peut être associée à plusieurs cours et collections.

### 3.3 Vocabulaire

Champs prévus :
- Kanji facultatif.
- Kana.
- Romaji.
- Traduction française.
- Catégorie grammaticale.
- Groupe verbal si applicable.
- Exemple japonais.
- Traduction de l'exemple.
- Cours associés.
- Statut de maîtrise.
- Prononciation TTS.
- Formes conjuguées.

Le kanji peut être ajouté ultérieurement sans créer une nouvelle connaissance.

### 3.4 Kanjis

Informations prévues :
- Caractère.
- Signification française apprise.
- Lectures Onyomi.
- Lectures Kunyomi.
- Ordre de tracé.
- Vocabulaire associé.
- Prononciation TTS.

### 3.5 Grammaire et particules

Informations prévues :
- Structure.
- Signification.
- Construction.
- Exemples japonais.
- Traductions françaises.
- Niveau JLPT indicatif.
- Sources.

### 3.6 Conjugaison

Les formes conjuguées sont générées algorithmiquement à partir :
- Du type verbal.
- Des règles de conjugaison.
- Des informations de la connaissance.

Seules les formes étudiées et validées sont autorisées dans les exercices et conversations.

### 3.7 Expressions et nombres

Les expressions possèdent une signification, des usages et des exemples.

Les nombres couvrent notamment :
- Les nombres.
- Les compteurs.
- Le calendrier.
- Les règles de comptage.

### 3.8 Statuts et disponibilité

Chaque connaissance possède deux informations indépendantes.

**Statut de maîtrise :**
- Nouveau.
- En apprentissage.
- Maîtrisé.

**Disponibilité pédagogique :**
- Active.
- Non appris.

Une connaissance marquée "Non appris" est exclue des flashcards, exercices et conversations.

Son statut de maîtrise et son historique sont conservés. Elle peut être activée ultérieurement sans perte de données.

L'algorithme peut faire évoluer automatiquement le statut de maîtrise. L'utilisateur peut également le modifier manuellement.

---

## 4. Cours

### 4.1 Gestion

Un cours comprend :
- Un titre.
- Une date.
- Un support d'origine.
- Des connaissances associées.

Supports prédéfinis :
- Marugoto.
- Busuu.
- Autres supports déjà enregistrés.

Un nouveau support peut être créé depuis le formulaire.

### 4.2 Associations

Une connaissance peut apparaître dans plusieurs cours.

L'association se fait depuis une bibliothèque avec :
- Recherche.
- Filtres.
- Sélection multiple.
- Création d'une connaissance sans quitter le formulaire.

### 4.3 Affichage et gestion

Les cours sont affichés sous forme de cartes présentant :
- Le titre.
- La date.
- Le support.
- Le nombre de connaissances associées.

Tri par date décroissante.

Modification complète disponible.

La suppression est logique : archivage, restauration ou suppression définitive après confirmation.

---

## 5. Collections

Les collections permettent de regrouper librement les connaissances indépendamment des cours.

Une collection comprend :
- Un nom unique sans distinction de casse.
- Une description facultative.
- Des connaissances associées.

Fonctionnalités :
- Création.
- Modification.
- Suppression.
- Recherche par nom.
- Tri alphabétique.
- Pagination de 20 éléments.
- Ajout et retrait de connaissances.

La suppression d'une collection ne supprime jamais ses connaissances.

Le retrait d'une connaissance d'une collection est immédiat et ne nécessite pas de confirmation.

Il n'existe pas de limite au nombre de collections.

---

## 6. Flashcards et répétition espacée

### 6.1 Objectif

Permettre la mémorisation durable des connaissances grâce à la répétition espacée.

L'algorithme retenu est FSRS avec une rétention cible de 95 %.

### 6.2 Types de cartes

Six modes sont prévus :
1. Français vers japonais.
2. Romaji vers kana.
3. Kana vers kanji.
4. Kanji vers lecture et traduction.
5. Audio vers identification du mot.
6. Compléter une phrase.
7. Reconnaître une forme verbale.

Chaque compétence dispose d'un historique FSRS indépendant.

### 6.3 Génération des cartes

Les cartes sont créées automatiquement lorsque les informations nécessaires sont disponibles.

Une connaissance unique peut être enrichie progressivement :
- Romaji.
- Kana.
- Kanji.
- Autres informations linguistiques.

L'ajout d'une nouvelle écriture enrichit la fiche existante.

Les réponses alternatives doivent être explicitement définies dans les données de référence.

Une modification importante d'une carte entraîne la réinitialisation de son suivi FSRS. Une modification purement visuelle ne le réinitialise pas.

Les cartes inutilisables sont suspendues automatiquement. Elles peuvent être réactivées lorsque les informations manquantes sont rétablies. L'historique est conservé.

### 6.4 Préparation d'une session

Les révisions dues sont sélectionnées automatiquement, mais la sélection reste modifiable.

Modes :
- Automatique, avec alternance des types de cartes.
- Sélection personnalisée avec filtres avancés.

Quantités disponibles :
- 10.
- 20.
- 30.
- 50.
- Toutes.

Les nouvelles connaissances sont limitées à 10 par jour, par connaissance et non par carte.

Si le quota est atteint, les nouvelles connaissances attendent les jours suivants dans l'ordre de création.

### 6.5 Évaluation

Après chaque carte, l'utilisateur choisit :
- Encore difficile.
- Difficile.
- Bien.
- Facile.

FSRS calcule la prochaine échéance.

Les cartes maîtrisées restent susceptibles d'être révisées selon leur probabilité de rétention.

### 6.6 Interruption et reprise

- Sauvegarde après chaque carte.
- Reprise automatique d'une session interrompue.
- Conservation de la position, des cartes restantes et des filtres.
- Les cartes déjà évaluées ne sont pas rejouées.
- Une seule session inachevée à la fois.
- Abandon volontaire avec sauvegarde et possibilité de reprise.
- Remplacement d'une session inachevée uniquement après confirmation.

### 6.7 Bilan

Le bilan présente :
- Nombre de cartes évaluées.
- Durée.
- Répartition des évaluations FSRS.
- Évolution des statuts.
- Erreurs et difficultés.
- Prochaines échéances.

Les bilans de session ne sont pas archivés. L'historique FSRS individuel reste conservé.

---

## 7. Exercices

### 7.1 Modes

Deux modes :
- Automatique, adapté aux lacunes, erreurs et disponibilités.
- Personnalisé, avec filtres et critères définis par l'utilisateur.

### 7.2 Types d'exercices

- Phrases à trous.
- Conjugaison.
- Compréhension écrite.
- Compréhension orale.
- Traduction.

Les formats sont la saisie libre et la réponse orale. Aucun QCM n'est prévu comme format d'exercice.

### 7.3 Génération

Les exercices sont générés dynamiquement par l'IA locale à partir des connaissances personnelles disponibles.

L'IA peut utiliser toutes les connaissances personnelles actives et validées, sans se limiter aux connaissances du niveau JLPT.

Elle ne doit jamais introduire une notion inconnue ou non autorisée.

Les exercices sont vérifiés avant présentation.

### 7.4 Difficulté et durée

Difficulté :
- Facile.
- Intermédiaire.
- Difficile.
- Adaptative par défaut.

Durées :
- 5 minutes.
- 10 minutes.
- 15 minutes.
- 20 minutes.
- 30 minutes.

Le compte à rebours est facultatif. Le temps est suivi en arrière-plan.

Un exercice commencé peut être terminé après expiration de la durée prévue. L'utilisateur peut arrêter avant.

### 7.5 Comportement

- Correction immédiate.
- Explication uniquement en cas d'erreur.
- Compréhension écrite avec phrases, dialogues ou textes.
- Compréhension orale par TTS local, vitesse normale ou ralentie.
- Texte japonais masqué avant validation de la réponse.
- Traduction français-japonais et japonais-français.
- Conjugaison limitée aux formes étudiées et validées.
- Réponse aux phrases à trous limitée aux éléments manquants.

### 7.6 Filtres personnalisés

- Types.
- Catégories.
- Cours.
- Collections.
- Difficulté.
- Durée.

### 7.7 Progression et bilans

Les exercices contribuent aux statuts et priorités de révision.

Les erreurs augmentent la priorité des exercices concernés, sans modifier l'échéance FSRS.

Le bilan contient :
- Score.
- Réussites.
- Erreurs et corrections.
- Durée réelle.

Les statistiques sont conservées, mais pas les exercices et réponses détaillés.

La reprise automatique est disponible. Chaque exercice est sauvegardé après traitement.

---

## 8. Conversations avec l'IA

### 8.1 Modes

- Conversation libre.
- Scénario guidé.
- Jeu de rôle.

Les scénarios et personnages sont générés automatiquement par l'IA selon les connaissances disponibles et la difficulté.

Aucun catalogue manuel de scénarios n'est prévu.

### 8.2 Règles pédagogiques

- Conversation exclusivement en japonais.
- Explications en français dans le bilan final.
- L'IA utilise uniquement les connaissances personnelles actives et validées.
- Aucune expression ou notion inconnue ne doit être introduite.
- La conversation doit rester équilibrée, l'IA doit poser des questions et rebondir.
- L'utilisateur peut demander une répétition, une reformulation ou un indice.
- Le registre poli ou familier s'adapte au contexte.
- Les corrections sont intégrées naturellement par reformulation.
- Les corrections détaillées sont présentées à la fin.

### 8.3 Durée

Durées :
- 5 minutes.
- 10 minutes.
- 15 minutes.
- 20 minutes.
- 30 minutes.

Compte à rebours facultatif.

À expiration, possibilité de terminer ou de prolonger de 5 minutes. La prolongation est renouvelable.

Fin manuelle possible.

### 8.4 Oral et écrit

Les deux modes sont disponibles.

En mode oral :
- Bouton manuel d'enregistrement.
- Autorisation micro lors de la première utilisation.
- Micro ouvert uniquement pendant l'enregistrement.
- Transcription locale par Whisper.cpp.
- Transcription affichée avant envoi pour confirmation ou correction.
- Bascule immédiate vers l'écrit en cas de problème audio, sans perte de contexte.

Les réponses utilisateur sont affichées telles que saisies.

Chaque réponse japonaise de l'IA peut être lue automatiquement selon les préférences globales. Une réécoute est possible.

### 8.5 Évaluation des connaissances

Les conversations contribuent aux statuts et priorités.

Une connaissance correctement utilisée cinq fois dans des échanges différents constitue une preuve suffisante.

Les erreurs sont pondérées selon leur gravité et leur contexte. En cas d'incertitude, aucune évolution n'est appliquée.

Les erreurs de transcription vocale ne sont pas automatiquement considérées comme des erreurs de japonais.

Aucune échéance FSRS n'est modifiée par les conversations.

### 8.6 Bilan

Le bilan comprend :
- Corrections.
- Vocabulaire.
- Grammaire.
- Points positifs.
- Recommandations.
- Durée réelle.

Le transcript complet n'est pas conservé.

En cas d'interruption, la conversation est abandonnée automatiquement. Le contexte temporaire est supprimé, mais les progrès déjà enregistrés sont conservés. Aucun bilan n'est généré.

### 8.7 Statistiques

Conservation de :
- Date.
- Durée.
- Mode.
- Difficulté.
- Résultats globaux.

Indicateurs :
- Nombre de conversations.
- Durée totale et moyenne.
- Fréquence.
- Évolution des résultats.
- Répartition des modes et difficultés.

Un bilan hebdomadaire est généré le dimanche, conservé à vie et présenté dans une liste chronologique.

Il contient des recommandations IA. Les semaines ne sont pas comparées entre elles.

---

## 9. Évaluation initiale

### 9.1 Objectif

Identifier les connaissances déjà acquises par l'utilisateur afin de constituer sa bibliothèque personnelle initiale.

L'évaluation est facultative, déclenchée depuis le tableau de bord et relançable.

Elle n'est pas imposée à la création du compte.

### 9.2 Référentiels utilisés

Les référentiels communautaires N5 à N1 servent exclusivement à cette évaluation.

Ils ne constituent pas une bibliothèque pédagogique accessible aux autres fonctionnalités.

Le référentiel permet à l'IA de tester les connaissances, puis de proposer des fiches personnelles nouvelles ou enrichies.

### 9.3 Domaines

- Vocabulaire.
- Kanjis.
- Grammaire.
- Particules.
- Conjugaison.

Évaluation adaptative : progression initiale simple puis augmentation de difficulté selon les réponses.

Le niveau estimé est global et détaillé par domaine.

### 9.4 Résultats

Le résultat comprend :
- Niveau JLPT estimé global.
- Niveaux estimés par domaine.
- Connaissances identifiées.
- Statuts de maîtrise estimés.
- Réponses.
- Corrections.
- Explications.

Le niveau JLPT est indicatif, non officiel.

### 9.5 Création et enrichissement

L'IA prépare des fiches personnelles proposées à l'utilisateur.

Chaque proposition peut être :
- Acceptée.
- Refusée.
- Modifiée avant validation.

La validation peut porter sur la fiche entière ou sur ses informations détaillées.

Aucune création ni modification n'est effectuée sans consentement.

Si une fiche personnelle existe déjà, seules les informations manquantes sont proposées. Les informations existantes ne sont pas écrasées.

Une correspondance avec une fiche manuelle est détectée automatiquement selon les règles de doublons, mais l'utilisateur valide toute modification.

### 9.6 Relance

Une relance identifie uniquement les connaissances absentes de la bibliothèque personnelle. Les connaissances déjà présentes ne sont pas réévaluées.

Les résultats détaillés de l'évaluation sont visibles à la fin, puis ne sont pas conservés. Seuls le niveau estimé et les résultats par domaine sont conservés dans Progression.

---

## 10. Ressources linguistiques

### 10.1 Organisation

Les référentiels sont préparés indépendamment de l'application, puis déposés dans le dépôt GitHub.

Priorité initiale :
- N5.
- N4.

Les niveaux N3 à N1 seront développés ultérieurement.

Organisation :
- Un dossier par niveau JLPT.
- Un fichier JSON par catégorie.
- Un schéma JSON commun.
- Aucun référentiel intégré dans SQLite.

### 10.2 Données

Chaque fichier contient :
- Métadonnées.
- Tableau des connaissances.

Chaque fiche possède un UUID v4 stable.

Les relations entre fiches sont représentées par UUID.

Les métadonnées comprennent :
- Identifiant du fichier.
- Version du schéma.
- Niveau JLPT.
- Catégorie.
- Version du référentiel.
- Date de mise à jour.
- Sources générales.
- Licences associées.

La structure est hybride : fiche principale et sous-entrées indépendantes pour les sens et usages.

Les sources importantes sont tracées au niveau des informations concernées.

### 10.3 Qualité

Avant publication :
- Vérification humaine.
- Contrôles automatiques.
- Validation JSON et JSON Schema.
- Détection des doublons exacts.
- Signalement des ressemblances pour contrôle humain.

Les informations incertaines ou contradictoires non résolues sont exclues.

Les champs facultatifs peuvent être absents si les informations essentielles sont présentes.

Les exemples japonais sont originaux et vérifiés humainement pour leur grammaire et leur naturel.

### 10.4 Sources et licences

Seules les ressources légalement redistribuables sont intégrées, dans le respect des licences.

Les sources générales peuvent être indiquées par URL ou référence bibliographique.

Les référentiels sont non officiels. Le JLPT ne publie plus de listes exhaustives officielles de vocabulaire, kanjis et grammaire par niveau.

### 10.5 Mises à jour

- Vérification GitHub au démarrage.
- Aucune installation automatique.
- Comparaison des ajouts, modifications et suppressions.
- Affichage du nombre total de changements.
- Confirmation explicite avant application.

Règles :
- Suppression dans le référentiel : fiche personnelle conservée, suppression signalée.
- Modification d'une entrée : aucune modification automatique de la fiche personnelle.
- Modification d'une relation : aucune modification automatique des relations personnelles.
- Déplacement de niveau JLPT : automatique, UUID conservé.
- Version incompatible : refus de la mise à jour et conservation de la version actuelle.

### 10.6 Absence de référentiel

L'absence ou l'incomplétude d'un référentiel n'empêche pas les autres fonctions.

Pendant l'évaluation initiale, les domaines indisponibles sont ignorés et les domaines disponibles restent utilisables.

---

## 11. Tableau de bord

Le tableau de bord comporte :
- En-tête minimaliste sans salutation.
- Progression globale.
- Révisions dues et bouton "Commencer".
- Objectifs quotidiens et hebdomadaires.
- Trois dernières activités.

La progression globale est pondérée selon les statuts :
- Nouveau : 0 %.
- En apprentissage : 50 %.
- Maîtrisé : 100 %.

Formule : somme des valeurs de maîtrise divisée par le nombre total de connaissances prises en compte.

Affichage desktop en trois colonnes.

---

## 12. Progression

### 12.1 Synthèse

- Progression globale.
- Répartition des statuts.
- Révisions et taux de réussite.
- Exercices et scores.
- Conversations et durées.
- Régularité sur sept jours.

Période par défaut : 30 jours.

Périodes disponibles :
- 7 jours.
- 30 jours.
- 3 mois.
- 6 mois.
- 12 mois.
- Depuis le début.
- Personnalisée.

### 12.2 Onglets

**Connaissances :**
- Total.
- Statuts.
- Ajouts.
- Maîtrise.
- Répartition par catégorie.
- Progression JLPT N5 à N1.
- Non classées.

**Flashcards :**
- Total révisé.
- Taux de réussite.
- Évolution FSRS.
- Cartes en attente ou suspendues.

**Exercices :**
- Total.
- Score moyen.
- Évolution.
- Réussites et erreurs.
- Durées.
- Types.

**Conversations :**
- Nombre.
- Durées.
- Fréquence.
- Résultats.
- Modes et difficultés.
- Bilans hebdomadaires.

### 12.3 Régularité

Activités comptabilisées :
- Flashcards.
- Exercices.
- Conversations.
- Ajout et modification de connaissances.
- Création et modification de cours.

Les collections ne comptent pas.

La série est stricte. La journée en cours n'est pas considérée comme manquée avant sa fin.

Heatmap inspirée de GitHub, avec intensité selon le volume d'activité.

---

## 13. Paramètres

### Profil
- Pseudo.
- Mot de passe.
- Photo.
- Question de sécurité.
- Récupération.
- Suppression du compte.

### Apparence
- Thème sombre par défaut.
- Thème clair disponible.
- Inter.
- Zen Kaku Gothic New.

### Apprentissage
- Romaji.
- Furigana.
- Écriture japonaise prioritaire.
- Traduction française.
- Aides automatiques.

### Objectifs
- Quotidiens : minutes et connaissances révisées.
- Hebdomadaires : minutes et jours actifs.
- Activation indépendante.
- Objectif atteint si au moins un critère activé est rempli.

### Audio
- Lecture automatique TTS.
- Voix.
- Vitesse.
- Test.
- STT.
- Micro.
- Volume.
- Sons d'interface.

### IA
- Moteur.
- Disponibilité d'Ollama et aide à son installation ou à son démarrage.
- Modèle Ollama installé.
- État du modèle sélectionné et accord explicite avant tout téléchargement éventuel.
- Température.
- Longueur maximale.
- Test.
- Règles pédagogiques imposées.

### Sauvegardes
- Connexion Google Drive.
- Activation.
- Historique.
- Restauration.
- Export.
- Import.
- État et erreurs.

### Application
- Version.
- Licences et dépendances.
- Documentation.
- Logs et dépannage.
- Mise à jour GitHub manuelle.

---

## 14. Sauvegarde, import et export

### 14.1 Sauvegarde

Chaque utilisateur connecte son propre compte Google Drive.

Sauvegarde automatique :
- Snapshot quotidien.
- Après 10 modifications enregistrées.
- Au plus tard 30 minutes après des changements.
- À la fermeture.
- Aucune sauvegarde si rien n'a changé.

Conservation des 30 derniers snapshots non protégés. Les snapshots protégés ne sont jamais supprimés automatiquement.

La sauvegarde couvre :
- Cours.
- Connaissances.
- Collections.
- Progression et statistiques.
- Paramètres.
- Photos et fichiers liés.

Les mots de passe en clair, tokens OAuth et données temporaires sont exclus.

Les fichiers SQLite ne sont jamais exécutés directement depuis un dossier synchronisé. Une copie snapshot est envoyée vers Drive.

### 14.2 Échec de sauvegarde

- Données locales intactes.
- Erreur signalée.
- Sauvegarde en attente conservée localement.
- Nouvelle tentative automatique.
- Une sauvegarde incomplète ne remplace jamais une sauvegarde valide.

### 14.3 Restauration

La restauration :
- Remplace intégralement les données concernées.
- Présente un aperçu.
- Demande confirmation.
- Effectue une sauvegarde de l'état actuel avant remplacement.
- Vérifie la version du format.
- Effectue les migrations sur une copie temporaire.
- Refuse les formats futurs incompatibles.

### 14.4 Import et export

Formats :
- JSON.
- CSV.

Import JSON :
- Remplacement total ou fusion au choix.
- Aperçu et vérification avant action.
- Les doublons importés remplacent les existants en mode fusion.
- Transaction SQLite avec rollback en cas d'erreur.

Limites :
- JSON : 200 Mo.
- CSV : 100 Mo.

---

## 15. Notifications et erreurs

Aucune notification globale n'est prévue.

Les informations sont affichées dans les sections concernées.

Gestion :
- Erreurs récupérables : message et nouvelle tentative.
- Erreurs de validation : indication sur les champs.
- Erreurs critiques : écran dédié.
- Défaillance d'un service local : solution de repli sans bloquer les autres fonctionnalités.

---

## 16. Gestion des générations IA

- Une génération active maximum.
- Dix demandes maximum en attente.
- Position visible.
- Annulation possible.
- Priorités gérées automatiquement.
- Une demande moins prioritaire peut être remplacée si la file est pleine.
- En cas d'égalité, la plus ancienne est remplacée.
- Si aucune demande strictement moins prioritaire n'existe, la nouvelle demande est refusée.

En cas d'annulation :
- Arrêt immédiat.
- Libération de la ressource.
- Suppression des résultats partiels.
- Passage à la demande suivante.

En cas d'erreur :
- Une tentative.
- Suppression des résultats partiels.
- Nouvelle tentative manuelle.

Les demandes en attente expirent après sept jours.

La file est sauvegardée à la fermeture et vérifiée au lancement.

Un changement de compte suspend la file du compte précédent. La génération active est interrompue, mais les demandes non commencées sont conservées.

En cas de crash Ollama :
- Interruption de la génération.
- Suppression des résultats partiels.
- Suspension de la file.
- Vérification automatique du retour du service.
- Deux minutes sans réponse ou progression pour déclarer le service indisponible.
- Suspension durable après cinq vérifications consécutives infructueuses.
- Intervention manuelle nécessaire pour reprendre après suspension durable.

Les mises à jour de modèles Ollama sont gérées par l'utilisateur. Un test de compatibilité est obligatoire avant réutilisation.

---

## 17. Interface et expérience utilisateur

### Direction artistique

Ambiance cosy, immersive et apaisante : "Nuit japonaise".

Palette sombre :
- Nuit : #19172E
- Indigo : #373052
- Mauve : #66577C
- Cuivre : #D18B65
- Crème : #F0E5DC

Palette claire :
- Ivoire : #F0E5DC
- Rose grisé : #E5D8D5
- Mauve grisé : #D2C2C8
- Cuivre foncé : #A96848
- Nuit : #19172E

Contrastes à vérifier selon WCAG AA.

### Typographies

- Inter pour l'interface et le latin.
- Zen Kaku Gothic New pour le japonais.
- Romaji discret et masquable.
- Furigana masquable.

### Navigation

Navigation latérale adaptée à la largeur de la fenêtre Windows :
- Permanente lorsque l'espace le permet.
- Rétractable ou compacte dans une fenêtre étroite.

Les dispositions adaptées aux petits écrans ne constituent pas une distribution mobile ou tablette dans la première version.

Exercices et conversations en plein écran, sans distraction.

### Accessibilité

- Navigation clavier complète.
- Contrastes renforcés.
- Animations réduites ou désactivables.
- Zones tactiles suffisamment grandes.
- Lecteur d'écran non prioritaire.

Les suppressions et actions irréversibles nécessitent une confirmation.

