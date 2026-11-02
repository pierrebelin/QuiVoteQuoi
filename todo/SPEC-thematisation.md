# THÉMATISATION — Rattachement aux familles thématiques

> Rattacher chaque texte débattu à une à trois familles thématiques, pour qu'un visiteur parcoure les votes par sujet. Une règle publiée tranche quand la nature du texte suffit, le modèle de langage propose sinon, un humain corrige, la méthode est publiée. Seul endroit du produit où un jugement entre (README.md §5), et seul poste qui appelle un modèle — donc le seul qu'il faille tenir à l'économie (RM-14).

## 1. Contexte

Le site expose 8 434 scrutins. Aucun n'est parcourable par sujet. La promesse — « voici les textes sur le logement, voici comment chaque groupe a voté » — n'a pas de porteur.

README.md §5 fait porter la thématisation sur le dossier. Mesuré le 3 août 2026 : 5 826 scrutins sur 8 434 (69 %) n'ont aucun dossier, et les 2 608 restants ne renvoient qu'à 75 dossiers. Pire, la source publie le lien dossier de façon irrégulière **à l'intérieur d'un même texte** : la proposition de loi relative au droit à l'aide à mourir porte 297 scrutins avec dossier et 541 sans. Thématiser le dossier laisserait 541 votes du même texte hors thème — sélection éditoriale involontaire, interdite par README.md §2.

Le porteur retenu est donc le **texte débattu**, nommé dans l'objet de chaque scrutin.

## 2. Vocabulaire

| Terme | Définition |
|---|---|
| Famille thématique | Une des 16 familles de README.md §5. Référentiel fermé |
| Règle de rattachement | Énoncé publié rattachant un texte sur sa nature juridique, sans modèle |
| Texte débattu | Texte que l'objet du scrutin nomme : « projet de loi de finances pour 2026 ». Porteur du rattachement |
| Clé de texte | Forme normalisée du libellé du texte. Deux objets qui nomment le même texte donnent la même clé |
| Rattachement | Lien daté entre un objet et une famille |
| Proposition | Rattachement produit par le modèle de langage, avec justification. Publié par défaut |
| Arbitrage | Décision humaine sur un rattachement : confirmé, corrigé ou écarté |
| Auteur du rattachement | Ce qui a ouvert la ligne, en clair : nom de la règle, du modèle ou du mainteneur. Conservé pour l'historique, pas affiché comme une catégorie |
| Non rattaché | Objet sans aucune famille retenue. Reste consultable |
| Méthode | Page publique décrivant familles, extraction, rôle du modèle, limites |

## 3. Cas d'usage

### CU-01 — Extraire les textes débattus
**Acteur** : système · **Intention** : disposer du porteur de thématisation · **Fréquence** : chaque rafraîchissement

**Scénario nominal :**
1. Système lit l'objet de chaque scrutin.
2. Extrait le texte nommé par règle publiée (RM-02), calcule sa clé.
3. Crée le texte débattu s'il est nouveau, rattache le scrutin à sa clé.
4. Rattache le dossier au texte quand un scrutin porte les deux (RM-06).

**Erreurs :** objet sans formule de texte reconnue → scrutin sans texte débattu, compté dans les non rattachés, jamais écarté (RM-01).

### CU-02 — Rattacher les objets en attente
**Acteur** : système · **Intention** : couvrir tous les objets sans travail humain préalable · **Fréquence** : chaque rafraîchissement (RM-15)

**Scénario nominal :**
1. Système constitue la file d'attente : textes sans famille courante d'abord, du plus voté au moins voté, puis dossiers qu'aucun scrutin ne relie à un texte. Un objet déjà rattaché n'y entre jamais (RM-14).
2. Pour chaque objet, système cherche une règle applicable (RM-13). Si elle existe, rattachement ouvert avec la règle pour auteur, son énoncé pour justification, et aucun appel au modèle.
3. Les objets restants partent au modèle **par lot** (RM-14), qui ne reçoit que leurs libellés (RM-04).
4. Modèle rend, par libellé, une à trois familles ordonnées, chacune avec une justification d'une à deux phrases (RM-03, RM-05).
5. Système écarte toute famille hors référentiel (RM-08), enregistre la proposition — familles, justifications, modèle, version d'instruction, date — et ouvre les rattachements.
6. Rattachements publiés.

**Variantes :**
- Modèle ne retient aucune famille pour un libellé → objet non rattaché, consultable, listé en attente (RM-01).
- Modèle omet un libellé du lot → objet non rattaché, repris à la passe suivante. Distinct du cas précédent : il n'a pas été jugé sans famille.

**Erreurs :** modèle indisponible → tout le lot reste non rattaché, re-tenté au rafraîchissement suivant. Le rafraîchissement n'échoue jamais pour cette cause (RM-15).

### CU-03 — Arbitrer une proposition
**Acteur** : mainteneur · **Intention** : corriger un jugement automatique · **Fréquence** : à la demande

**Scénario nominal :**
1. Mainteneur ouvre l'écran d'arbitrage, filtre par famille ou par état.
2. Écran affiche le libellé du texte, les familles proposées, les justifications, le nombre de scrutins portés.
3. Mainteneur retient, retire ou ajoute des familles, dans la limite de trois (RM-03), et motive sa décision.
4. Système clôt le rattachement précédent et ouvre le nouveau à la date du jour (RM-07).
5. Nouveau rattachement affiché, proposition initiale conservée et consultable.

**Variantes :** mainteneur confirme sans changement → l'ancien rattachement est clos, un nouveau est ouvert à son nom, familles inchangées.
**Erreurs :** accès sans jeton valide → écran refusé.

### CU-04 — Parcourir les votes d'une famille
**Acteur** : visiteur · **Intention** : voir ce qui a été voté sur un sujet · **Fréquence** : courante

**Scénario nominal :**
1. Visiteur ouvre une famille.
2. Système liste les textes rattachés, du plus récemment voté au plus ancien, avec le nombre de scrutins de chacun.
3. Visiteur ouvre un texte → ses scrutins, leur date, leur objet, leur sort. L'historique du rattachement, son auteur et sa justification restent consultables sur la fiche du texte.
4. Visiteur ouvre un scrutin → répartition par groupe et positions nominales (spec SCRUTINS).

### CU-05 — Consulter les objets non rattachés
**Acteur** : visiteur · **Intention** : vérifier que rien n'est caché · **Fréquence** : occasionnelle

**Scénario nominal :**
1. Visiteur ouvre la liste des non rattachés depuis n'importe quelle page de thème.
2. Système liste textes, scrutins et dossiers sans famille retenue, avec le nombre de votes concernés.
3. Visiteur ouvre l'un d'eux comme n'importe quel autre.

### CU-06 — Consulter la méthode
**Acteur** : visiteur · **Intention** : juger la démarche · **Fréquence** : occasionnelle

**Scénario nominal :**
1. Visiteur ouvre la page méthode depuis toute page de thème.
2. Page décrit : les 16 familles avec leur périmètre, la règle d'extraction du texte, la table des règles de rattachement (RM-13), le rôle exact du modèle, la limite de trois familles, le compte des non rattachés.
3. Page dit ce que le modèle ne fait pas : aucun chiffre, aucune lecture des votes (RM-04, RM-10).
4. Page dit ce qui ne lui est jamais soumis : les scrutins et les dossiers reliés à un texte, qui en héritent (RM-06), et les objets déjà rattachés (RM-14).

## 4. Règles métier

### RM-01 — Non rattaché reste consultable
- **Énoncé** : un objet sans famille reste accessible, listé, et compté dans la page méthode. Jamais retiré, jamais masqué. · **Origine** : README.md §2 · **Sévérité** : bloquant · **Applies to** : transverse

### RM-02 — Extraction déterministe du texte
- **Énoncé** : le texte débattu est extrait de l'objet du scrutin par une règle publiée, sans modèle de langage. Deux objets nommant le même texte donnent la même clé, quelle que soit la mention de lecture, la casse, l'espacement ou la forme de l'apostrophe. · **Origine** : README.md §8 · **Sévérité** : bloquant · **Applies to** : CU-01
- **Conforme** : « l'amendement n° 234 après l'article 7 du projet de loi de financement de la sécurité sociale pour 2026 (première lecture) » et « l'article 12 du projet de loi de financement de la sécurité sociale pour 2026 (nouvelle lecture) » donnent la même clé.

### RM-03 — Trois familles au plus
- **Énoncé** : un objet porte une à trois familles. Au-delà, seules les trois premières de l'ordre proposé sont retenues. La limite est publiée. · **Origine** : choix produit · **Sévérité** : bloquant · **Applies to** : CU-02, CU-03

### RM-04 — Le modèle ne voit que le libellé
- **Énoncé** : le modèle reçoit le libellé du texte, rien d'autre. Ni décomptes, ni positions, ni groupes, ni sort du vote. · **Origine** : README.md §6 · **Sévérité** : bloquant · **Applies to** : CU-02

### RM-05 — Justification obligatoire
- **Énoncé** : toute famille proposée porte une justification d'une à deux phrases, conservée et affichable. Famille sans justification = proposition invalide, rejetée. · **Origine** : README.md §5, §9 · **Sévérité** : bloquant · **Applies to** : CU-02

### RM-06 — Héritage descendant
- **Énoncé** : un scrutin porte les familles de son texte. Un dossier porte les familles du texte que ses scrutins nomment ; sans scrutin, il est classé sur son titre comme un texte. Aucun rattachement direct sur un scrutin. · **Origine** : choix produit, mesure du 3 août 2026 · **Sévérité** : bloquant · **Applies to** : CU-01, CU-04
- **Non conforme** : thématiser le dossier « Fin de vie » et laisser hors thème les 541 scrutins du même texte.

### RM-07 — Historique conservé
- **Énoncé** : réviser un rattachement clôt l'ancien à la date de la révision et en ouvre un nouveau. Aucune suppression. L'état d'un objet à une date passée reste reconstituable. · **Origine** : README.md §5, §9 · **Sévérité** : bloquant · **Applies to** : CU-03

### RM-08 — Référentiel de familles fermé
- **Énoncé** : les 16 familles de README.md §5 sont le seul jeu de valeurs. Toute famille rendue hors référentiel est écartée et journalisée. Aucune famille créée par le modèle. · **Origine** : README.md §5 · **Sévérité** : bloquant · **Applies to** : CU-02

### RM-09 — *(retirée le 9 août 2026)*
- La règle imposait d'afficher, à côté de chaque famille, si le rattachement venait d'une règle, du modèle ou d'un humain. Elle est retirée : la distinction ne dit rien au lecteur sur le vote qu'il vient consulter, et occupait la place au plus près de l'information utile. La traçabilité reste entière — l'auteur de chaque ligne est conservé en clair et l'historique n'efface rien (RM-07, README.md §9). Le numéro n'est pas réattribué, les règles suivantes gardant le leur.

### RM-10 — Le modèle ne produit aucun chiffre
- **Énoncé** : le modèle rend des familles et du texte de justification. Aucune note, aucun score, aucun rang, aucun décompte. Tout nombre affiché vient de la base. · **Origine** : README.md §6, §8 · **Sévérité** : bloquant · **Applies to** : CU-02, CU-04

### RM-11 — Famille sensible, factuel seul
- **Énoncé** : les familles « société / libertés » et « immigration » rattachent sur l'objet du texte, jamais sur son orientation. Une justification qui qualifie le texte est une proposition invalide. · **Origine** : README.md §5, §6 · **Sévérité** : bloquant · **Applies to** : CU-02, CU-03

### RM-12 — Aucun agrégat comparatif par thème
- **Énoncé** : une page de thème n'affiche aucun cumul, taux ou classement comparant les groupes entre eux. Les chiffres restent attachés à un scrutin. · **Origine** : README.md §6 · **Sévérité** : bloquant · **Applies to** : CU-04

### RM-13 — Règle publiée avant modèle
- **Énoncé** : un texte dont la **nature juridique** porte sa famille est rattaché par une règle publiée, sans appel au modèle. Chaque règle nomme le fragment qu'elle cherche, les familles qu'elle ouvre et son énoncé ; la table entière est publiée sur la page méthode. Une règle porte sur l'objet du texte, jamais sur son orientation. Le rattachement qu'elle produit nomme la règle comme auteur et reste révisable par arbitrage humain comme n'importe quel autre. · **Origine** : README.md §5, §8 · **Sévérité** : bloquant · **Applies to** : CU-02, CU-06
- **Conforme** : « projet de loi de finances pour 2026 » → économie / finances publiques, parce qu'une loi de finances fixe les recettes et les charges de l'État.
- **Non conforme** : une règle qui rattacherait sur un mot du titre sans que la nature du texte le commande — « sécurité » dans un intitulé ne fait pas un texte de justice / sécurité. Le doute laisse le texte au modèle.

### RM-14 — Un objet n'est soumis qu'une fois
- **Énoncé** : un objet déjà rattaché n'est jamais resoumis au modèle. Un objet qui hérite de son texte n'y est jamais soumis du tout (RM-06). Ce qui reste part au modèle par lot, le cadrage n'étant transmis qu'une fois par lot. Le coût d'une passe suit ce qui est nouveau, jamais la taille de la base. · **Origine** : README.md §5 · **Sévérité** : bloquant · **Applies to** : CU-02
- **Non conforme** : soumettre chaque scrutin — 8 434 appels pour 322 textes ; soumettre un dossier que ses scrutins relient déjà à un texte ; rappeler le modèle sur un texte arbitré.

### RM-15 — La catégorisation ne fait jamais échouer un rafraîchissement
- **Énoncé** : extraction et rattachement ferment le rafraîchissement, après l'écriture des scrutins. Leur échec est signalé dans le compte rendu de la passe, jamais propagé : les objets concernés restent non rattachés et consultables. Le nombre d'objets soumis par passe est plafonné ; le reliquat est repris à la passe suivante. · **Origine** : README.md §2 · **Sévérité** : bloquant · **Applies to** : CU-02

## 5. Données

| Donnée | Description | Source | Importance |
|---|---|---|---|
| Famille thématique | Code stable, libellé public, périmètre, ordre d'affichage | Catalogue | Essentiel |
| Règle de rattachement | Nom, fragment cherché, familles ouvertes, énoncé publié | Catalogue | Essentiel |
| Tentative sur un dossier | Date et issue de la dernière soumission d'un dossier sans scrutin | Calculée | Essentiel |
| Texte débattu | Clé normalisée, libellé publié, date du premier et du dernier vote | Calculée | Essentiel |
| Lien scrutin → texte | Clé du texte que l'objet du scrutin nomme | Calculée | Essentiel |
| Lien dossier → texte | Établi quand un scrutin porte dossier et texte | Calculée | Essentiel |
| Rattachement | Objet, famille, date d'ouverture, date de clôture, auteur, motif | Saisie ou calculée | Essentiel |
| Proposition | Familles ordonnées, justification par famille, modèle, version d'instruction, date | Calculée | Essentiel |
| Compte des non rattachés | Nombre de textes, scrutins et dossiers sans famille | Calculée | Essentiel |

## 6. États & transitions

| État | Événement | État suivant | Condition |
|---|---|---|---|
| absent | une règle s'applique | réglé | règle de la table publiée (RM-13) |
| absent | modèle propose | proposé | au moins une famille du référentiel, justifiée |
| absent | modèle ne retient rien | non rattaché | — |
| absent | modèle omet le libellé | absent | repris à la passe suivante |
| réglé | mainteneur confirme ou corrige | arbitré | familles ≤ 3 |
| proposé | mainteneur confirme | arbitré | — |
| proposé | mainteneur corrige | arbitré | familles ≤ 3 |
| proposé | mainteneur écarte tout | non rattaché | motif saisi |
| arbitré | mainteneur révise | arbitré | ancien rattachement clos, nouveau ouvert |
| non rattaché | mainteneur rattache | arbitré | familles ≤ 3 |

Réglé, proposé et arbitré se publient à l'identique : le visiteur voit la famille, pas l'état. Non rattaché est publié aussi (RM-01). Un objet réglé n'est jamais soumis au modèle (RM-14).

## 7. Comportements transverses

**Re-soumission** — un objet déjà rattaché n'est pas re-soumis au modèle, quoi qu'il l'ait rattaché (RM-14). Changer de modèle ou d'instruction produit de nouvelles propositions sur les seuls objets non rattachés. Redécouper le référentiel, en revanche, clôt les rattachements des familles touchées et rend leurs objets à la file d'attente.

**Texte sans famille après échec du modèle** — indistinguable, côté visiteur, d'un texte que le modèle n'a pas su rattacher : les deux sont non rattachés. La page méthode distingue les trois causes — jamais soumis, soumis sans famille retenue, soumis sans réponse.

**Ordre dans le rafraîchissement** — référentiel, dossiers, scrutins, extraction, rattachement. L'extraction lit les objets de scrutin qui viennent d'être écrits ; la rattacher plus tôt travaillerait sur l'état précédent. Elle reste utile même quand la source des scrutins n'a pas répondu, puisqu'elle relit ce qui est déjà stocké.

## 8. Relations

| Amont | Aval |
|---|---|
| Objets des scrutins (spec SCRUTINS) | Textes débattus, clés |
| Dossiers ingérés | Rattachement des dossiers sans scrutin |
| Textes débattus rattachés | Pages thème × groupe × période (Phase 5) |
| Rattachements et propositions | Page méthode (README.md §9, Phase 7) |

## 9. Hors périmètre

| Exclusion | Raison |
|---|---|
| Sous-familles, mots-clés libres | Référentiel fermé (RM-08) |
| Rattachement thématique d'un amendement pris isolément | Le porteur est le texte (RM-06) |
| Agrégats thématiques par groupe | RM-12, README.md §6 |
| Traduction des familles en axes politiques | README.md §3.1 |

## 10. Hypothèses

Mesurées sur les données réelles le 3 août 2026, législature 17, par la règle d'extraction telle qu'elle est livrée.

| # | Hypothèse | Statut |
|---|---|---|
| H1 | L'objet du scrutin nomme le texte débattu | **Confirmée.** 8 428 objets sur 8 434 nomment un texte, mesuré par la règle livrée. 6 exceptions, toutes des scrutins sans texte à débattre |
| H2 | Les objets se regroupent en un nombre de textes gérable | **Confirmée.** 322 textes distincts. Les 30 premiers portent 6 526 votes (77 %), les 100 premiers 7 918 (94 %). Le texte le plus voté en porte 931 |
| H3 | Le lien dossier publié par la source suffirait à thématiser | **Infirmée.** 2 608 scrutins sur 8 434 (31 %), 75 dossiers. La proposition de loi relative au droit à l'aide à mourir compte 297 scrutins avec dossier et 541 sans — même texte, même clé |
| H4 | Les dossiers référencés par un scrutin sont ingérés | **Confirmée.** 75 sur 75, zéro lien mort. Répond à SPEC-scrutins Q3 |
| H5 | Un texte débattu correspond à un dossier au plus | **Confirmée pour l'essentiel.** 75 dossiers reliés, 91 liens : 16 dossiers portent deux textes, le libellé ayant changé d'une lecture à l'autre (Q2). Les deux liens sont conservés, aucun n'est arbitré |
| H6 | La normalisation de la clé est nécessaire, pas cosmétique | **Confirmée.** Avant normalisation, la seule forme de l'apostrophe séparait « droit à l'aide à mourir » en deux clés de 80 et 838 scrutins ; la mention de lecture en séparait d'autres. Après, le texte en compte 931 |

## 11. Questions ouvertes

| # | Question | Impact | Options |
|---|---|---|---|
| Q1 | 6 objets ne nomment aucun texte (déclarations de politique générale, motions hors texte). Aucun porteur de thème. | 6 scrutins sur 8 434, non rattachés | Laisser non rattachés (retenu) · rattacher à « institutions / procédure » par règle publiée |
| Q2 | Le libellé du texte change entre deux lectures (« garantir l'égal accès … » puis « accompagnement et soins palliatifs »). La clé les sépare. | Un texte suivi sur deux lectures s'affiche en deux entrées | Laisser tel quel · rapprocher par arbitrage humain · rapprocher par le dossier quand il existe |
| Q3 | La justification produite par le modèle est-elle affichée au visiteur, ou réservée à l'écran d'arbitrage ? | Transparence contre bruit sur la page de thème | Affichée sur la fiche du texte · réservée à la page méthode · réservée à l'arbitrage |
| Q4 | Un rattachement automatique reste publié sans limite de temps, et depuis le retrait de RM-09 rien ne le signale plus au visiteur. Rien ne force l'arbitrage. | Le contrôle humain de README.md §4 peut ne jamais avoir lieu, sans que cela se voie | Priorisation des textes par nombre de votes portés · revue périodique des rattachements jamais arbitrés · aucune contrainte |

→ Étape suivante : /plan-implementation
