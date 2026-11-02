# Plan — Entrée par thèmes, groupes et textes votés

> Statut : proposition validée dans son principe.
> But : rendre l’application immédiatement utile à une personne non spécialiste : partir d’un thème, lire les derniers actes des groupes, puis vérifier le texte et la source.

## 1. Principes non négociables

- Faits, sources et dates. Aucun score, classement, intention, causalité ou jugement politique.
- Le site montre des **groupes parlementaires**, pas des partis sous un autre nom.
- Les groupes, textes et versions sont toujours datés.
- Même traitement pour tous les groupes : mêmes critères, ordre chronologique, mêmes sources, mêmes mentions d’absence.
- Un aperçu ne remplace jamais l’ensemble : tout résultat mène vers la liste exhaustive concernée.
- Un résumé aide à lire ; le texte officiel et le détail du vote restent accessibles.

## 2. Sujet A — Nouvelle page d’entrée

### 2.1 Rôle

La nouvelle page devient l’accueil. Elle remplace une entrée par les objets institutionnels (« dossiers », « scrutins ») par une entrée lecteur : **un thème, puis un groupe**.

Promesse à afficher :

> Choisissez un thème. Regardez ce que les groupes ont voté.

L’actuel accueil devient la page **« À propos du site »**. Il conserve la promesse générale, les règles de neutralité, les explications et les sources.

### 2.2 Parcours lecteur

1. Le lecteur choisit une famille parmi les seize thèmes publiés.
2. Il choisit un groupe parlementaire avec son nom complet ; aucun groupe n’est présélectionné.
3. Il peut ajouter un groupe pour comparer, sans dépasser quatre groupes.
4. Il lit les cinq actes les plus récents du thème.
5. Il ouvre, pour un acte, le résumé, le texte complet, le détail du vote et la source officielle.
6. Il peut voir tous les actes et votes finaux associés au thème.

### 2.3 Premier écran

```text
Comment ont voté les groupes sur…

[ Logement ] [ Santé ] [ Pouvoir d’achat ] [ Énergie ]
[ Agriculture ] [ Travail ] [ Numérique ] [ Justice / sécurité ]
                         [ Voir les 13 thèmes ]
```

- Choix fermé au premier geste : il rend le parcours compréhensible et expose le référentiel public de thèmes.
- Pas de chat ni de LLM visible dans cette première version.
- Une recherche par terme précis peut venir ensuite ; elle complète les thèmes, ne crée pas de thème caché.

### 2.4 Choix des groupes

```text
Logement

Quel groupe voulez-vous regarder ?
[ Horizons & Indépendants ] [ Rassemblement National ] [ … ]

[ Ajouter un groupe pour comparer ]
```

- Lecture initiale : un groupe.
- Comparaison : option explicite, jusqu’à quatre groupes.
- Libellé complet, sigle et période d’existence ; le sigle seul ne suffit pas à un néophyte.
- La page rappelle qu’un groupe parlementaire n’est pas nécessairement un parti.

### 2.5 Les cinq cartes

Chaque carte doit faire comprendre avant de faire interpréter.

```text
[ Date · vote final ]

Titre du texte

Ce que prévoit le texte
Résumé neutre, sourcé, de la version liée au vote.

Vote de l’Assemblée
Adopté / rejeté · résultat et date.

Groupe sélectionné
Pour / contre / abstentions / non-votants.

[ Lire le texte intégral ] [ Voir le détail du vote ] [ Source officielle ]
```

- Ordre strict : du plus récent au plus ancien. Aucun tri par importance, résultat ou groupe.
- Le texte « cinq derniers » doit rester visible, avec un accès à l’ensemble des votes finaux du thème.
- Une absence d’acte, de texte source ou de répartition doit être explicitement décrite ; elle ne devient jamais une abstention.

## 3. Sujet B — Texte voté, résumé et lecture intégrale

### 3.1 Deux niveaux de lecture

| Niveau | Objet | Contenu affiché |
|---|---|---|
| Vote final | Version précise examinée lors de ce vote | Résumé « Ce que prévoit le texte », texte officiel intégral, vote et répartition par groupe |
| Dossier après son dernier vote final | État connu le plus récent | Résumé de l’état du texte ; texte adopté lorsqu’il existe, sinon dernière version votée et issue explicite |

Un dossier peut changer entre dépôt, commission, lectures et vote final. Le site ne doit jamais présenter le texte initial comme celui effectivement voté.

### 3.2 Sources à établir avant tout résumé

Pour chaque version affichée, conserver et exposer :

| Information | Exigence |
|---|---|
| Document | Texte officiel précis, pas seulement titre ou objet du scrutin |
| Version | Étape, lecture et date correspondant au vote |
| Provenance | Producteur, URL officielle, date de publication et date de récupération |
| Intégrité | Empreinte ou autre repère permettant de détecter un changement du document |
| Réutilisation | Licence ou conditions de réutilisation vérifiées pour cette source |

Si la version exacte ne peut pas être identifiée, aucune synthèse ne doit prétendre expliquer son contenu. La carte conserve le vote et annonce cette limite.

### 3.3 Résumé neutre

Format : deux à quatre phrases, sous le libellé visible **« Synthèse automatique »** lorsqu’elle est générée.

Le résumé peut décrire :

- les règles, obligations, droits, dispositifs ou modifications explicitement contenus dans le texte ;
- le champ d’application et les principales étapes prévues ;
- le statut du texte après le vote.

Le résumé ne peut pas :

- évaluer une mesure ou prévoir ses effets ;
- attribuer une intention à un groupe, au Gouvernement ou aux auteurs ;
- produire de chiffres ;
- utiliser une source non affichée ;
- remplacer le texte intégral.

Les chiffres et répartitions de vote restent calculés et affichés par le produit, avec leurs sources.

### 3.4 Texte intégral

- Fermé par défaut sous « Lire le texte intégral », immédiatement sous le résumé.
- Reproduction verbatim de la version officielle identifiée ; aucun mélange de versions.
- Lien permanent vers le document d’origine, producteur et date de mise à jour.
- Si la licence du document ne permet pas encore une reproduction locale certaine, afficher le résumé et un lien vers l’original, sans recopier le document.

### 3.5 Réutilisation et transparence

- Les données diffusées par l’Assemblée sous Licence Ouverte peuvent être réutilisées avec attribution, date de mise à jour et sans présentation trompeuse de la source.
- Chaque source tierce — notamment Sénat, Vie publique et Légifrance — fait l’objet d’une vérification de licence séparée avant copie ou adaptation.
- Le site indique qu’il réutilise des sources publiques et ne bénéficie d’aucune caution de l’Assemblée nationale.
- Une méthode publique décrit les documents utilisés, les limites de couverture, la génération des résumés et leur correction.

## 4. Plan de réalisation

| Étape | Livrable | Condition de passage |
|---|---|---|
| 1. Cadrage des actes présentés | ✅ Définition publiée du vote sur l’ensemble, du « plus récent » et du lien avec un dossier | Pas de doublon ou de version ambiguë dans les cinq cartes |
| 2. Référencement des textes | ✅ Inventaire des documents officiels, versions, URLs et licences ; registre de rattachement explicite vote → version, synchronisable depuis le VPS sans accès distant depuis le poste de développement ; cinq références pilotes vérifiées et synchronisées sur le VPS | Chaque carte pilote relie un vote à son texte exact |
| 3. Résumé pilote | ✅ Captures horodatées et empreintées des cinq textes pilotes exécutées sur le VPS ; résumés neutres et sourcés produits | Contrôle humain : chaque phrase est soutenue par le texte cité |
| 4. Lecture intégrale | Affichage replié de documents dont la réutilisation est confirmée | Version, source et date visibles ; aucune confusion entre versions |
| 5. Nouvelle entrée | Accueil thème → groupe → cinq cartes | Un néophyte atteint une carte sans connaître « scrutin » ou « dossier » |
| 6. Vérification publique | Méthode, limites, tests de couverture et de neutralité | Même thème et même groupe donnent le même résultat à tous les visiteurs |

## 5. Critères d’acceptation

- Un lecteur choisit un thème et un groupe en moins de deux interactions.
- Chaque carte rend lisible le contenu du texte avant les chiffres du vote.
- Chaque résumé expose ses sources et sa version de texte.
- Un lecteur accède au texte complet et à la page officielle sans quitter le fil de lecture.
- Les cinq cartes sont identiques pour tous les visiteurs et strictement chronologiques.
- Le lecteur peut atteindre l’ensemble des votes finaux du thème.
- Aucun libellé ne présente une interprétation ou une position globale d’un groupe.

## 6. Décisions à confirmer avant réalisation

| Sujet | Proposition | Pourquoi |
|---|---|---|
| Unité des cinq cartes | Une carte par dossier, basée sur son vote final le plus récent | Évite d’afficher plusieurs lectures du même dossier dans l’aperçu |
| Personnalisation | Aucune ; même aperçu pour tous | Empêche une recommandation politique ou une sélection invisible |
| Recherche libre | Après la première version | Le référentiel fermé suffit à valider l’entrée lecteur |
| Texte sans licence confirmée | Lien externe, pas de copie intégrale | Préserve la réutilisation licite |
