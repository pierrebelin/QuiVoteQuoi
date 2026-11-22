# NAVETTE — Où en est la loi

> Une adoption à l'Assemblée ne doit plus se lire comme une fin. Chaque dossier législatif porte l'un de trois états (En navette, Adopté définitivement, Devenu loi), affiché avec le même libellé, la même icône et la même couleur sur toutes les pages. Les textes qui vont s'appliquer passent en avant. Maquettes validées le 21/11/2026 : canevas, page « Où en est la loi (projet) ».

## 1. Contexte

Le site ne calcule aujourd'hui qu'une seule issue fiable, « Promulguée » (`DossierOutcome::Promulgated`, acte `PROM-PUB`). Un texte adopté par l'Assemblée puis modifié par le Sénat s'affiche « Étape : … » ou « Adopté », et le visiteur croit la loi faite. À l'ingestion, l'adoption (`statutConclusion` `TSORTF01`) est lue puis jetée : seuls les rejets sont gardés. Le sort de chaque décision des deux chambres (`statutConclusion` des actes `*-DEC`, Sénat et CMP compris) est pourtant déjà dans le jeu des dossiers de l'AN.

Aucune donnée du Sénat n'est ajoutée, aucun vote nominatif de sénateur. README §2 (exhaustivité : tous les dossiers ont un état calculé, aucun n'est choisi), §6 (libellés factuels, aucun jugement), §8 (dates et numéros lus dans la source, phrases à trous, aucun LLM).

## 2. Vocabulaire

| Terme | Définition |
|---|---|
| Décision | Acte dont le code finit par `-DEC` (`AN1-DEBATS-DEC`, `SN2-DEBATS-DEC`, `CMP-DEC`, `CMP-DEBATS-AN-DEC`, `ANLDEF-DEBATS-DEC`…), avec sa date et son `statutConclusion`. |
| Sens d'une décision | Adopté, Adopté conforme (« sans modification »), Modifié (« modifiée », « avec modifications »), Rejeté, Accord CMP, Désaccord CMP. « Considéré comme adopté (49.3) » compte comme adopté ; « dans les conditions de l'article 45 al. 3 » aussi. |
| Chambre de dépôt | Chambre de la première lecture (`AN1` ou `SN1` de premier rang). Elle fixe l'ordre des jalons. |
| Même texte | Les deux chambres ont adopté le même texte : vote conforme de la seconde chambre, adoption du texte de CMP par les deux chambres, ou lecture définitive adoptée par l'Assemblée. |

## 3. Règles

- **RM-01 Devenu loi** : un acte `PROM-PUB` existe. Complément : « loi n° {codeLoi} du {date de promulgation} » ; la date du JO figure sur la fiche. On réutilise `DossierOutcome::Promulgated` et `LawPublication`, sans second calcul.
- **RM-02 Adopté définitivement** : pas de `PROM-PUB`, et la dernière décision produit le même texte (§2). Complément : « promulgation attendue ». Date de l'état : la date de cette décision.
- **RM-03 En navette** : ni RM-01 ni RM-02, au moins une chambre a adopté (ou modifié) le texte, et le dossier n'est ni retiré ni fusionné. C'est le cas d'un texte adopté par une chambre que l'autre n'a pas encore examiné (Q3). Un rejet suivi d'une adoption par l'autre chambre reste aussi en navette (PLFSS) ; cela remplace « la navette peut reprendre » (`REJECTION_NOT_FINAL`). Complément : la dernière décision, par exemple « modifié par le Sénat le 20 nov. ».
- **RM-04 Sans état** : aucune adoption nulle part (rejeté sans adoption, aucune décision, retiré, fusionné). Pas d'étiquette, le bandeau d'issue actuel reste (« Rejeté », « Retiré »…).
- **RM-05 Jalons** : quatre étapes, `{chambre de dépôt} → {autre chambre} → Accord → Loi`. Une étape franchie porte un point noir. L'icône d'état ne va que sur l'étape en cours ou sur le jalon « Loi ». Une étape à venir porte un cercle en pointillé et « à venir ». Sous-titre d'une chambre : « {sens} {date} ». Sous-titre de « Accord » (Q7) : « même texte · {date} » sans CMP, « CMP · {date} » avec CMP, « lecture définitive · {date} », « à venir » sinon. Sous-titre de « Loi » : « {date de promulgation} », « promulgation attendue » ou « à venir ». Les chambres portent la dernière décision de chacune.
- **RM-06 Encadré « Ce texte est-il devenu loi ? »** : réponse « Pas encore. » / « Bientôt. » / « Oui. », suivie d'une seule phrase choisie dans un jeu fermé de gabarits :
  - navette, dernière décision par une chambre : « {Le Sénat / L'Assemblée} l'a {adopté / modifié / rejeté} le {date} : {il revient à l'Assemblée pour une {n}e lecture / l'autre chambre doit encore l'examiner}. » ;
  - navette après un accord en CMP : « Accord en commission mixte paritaire le {date} : reste le vote {de l'Assemblée / du Sénat}. » ;
  - adopté définitivement : « Adopté définitivement le {date}, il deviendra loi à sa promulgation. » ;
  - devenu loi : « Loi n° {code} du {date}, publiée au Journal officiel du {date JO}. Légifrance ↗ ».
- **RM-07 Couleurs** : En navette gris `#ecebe7` (flèches) ; Adopté définitivement `#edfaf5` / `#04785d` (horloge) ; Devenu loi `#04785d` plein, texte blanc (coche). Le vert est réservé aux textes qui vont s'appliquer.
- **RM-08 Avancées de la semaine** : un dossier « a avancé » si une décision des chambres ou un `PROM-PUB` tombe dans la semaine, avec ou sans scrutin public. Groupes dans cet ordre : Devenus loi (seulement s'il y a une promulgation dans la semaine, Q4), Adoptés définitivement, En navette.

## 4. Écrans

| Écran | Changement | Décisions |
|---|---|---|
| Fiche dossier | Ligne d'état + identifiant, jalons, encadré, à la place d'`OutcomeBand` pour les trois états. Grille 881 + 320 px à 1280 px. | — |
| Fiche de vote | Jalons + encadré à la place du bloc « Où en est le texte », **avant** le résultat (Q6), avec « · ce vote » sur l'étape du scrutin. La pastille « Adopté · 1re lecture » ne change pas (Q5). | Q5, Q6 |
| Liste par texte | Étiquette d'état sur chaque carte. Filtres Tous / Devenus loi / Adoptés définitivement / En navette, avec compteurs sur le périmètre affiché : textes votés par défaut, tous les dossiers en « Tous » (Q2). L'option « Promulguée » du filtre d'issue disparaît (doublon de « Devenus loi ») ; Retiré, Fusionné et Rejeté restent. | Q2 |
| La semaine | Section « Textes qui ont avancé » (RM-08) et nombre « N textes ont avancé » dans la carte des chiffres. | Q4 |
| Accueil | Carte « Ce qui devient loi » : tous les textes adoptés définitivement, puis les 2 derniers devenus loi, puis un lien vers la liste filtrée sur Devenus loi. À 1280 px, **dans la colonne de droite (380 px), sous « Cette semaine »** (Q1, choix différent du canevas bureau) ; sur mobile, après « Cette semaine ». | Q1 |

## 5. Données témoins (jeu AN du 21/11/2026, vérifiées)

| Dossier | État attendu | Jalons |
|---|---|---|
| DLR5L17N51037 légitime défense | En navette · modifié par le Sénat le 20 nov. | AN adopté 7/07 (scrutin 7987 : 313 / 199 / 5) · Sénat modifié 20/11 (en cours) · Accord à venir · Loi à venir |
| DLR5L17N51962 Territoires zéro chômeur | Adopté définitivement 19/11 | AN adopté 27/01 · Sénat modifié 9/06 · Accord même texte 19/11 (AN2 conforme) · Loi promulgation attendue |
| DLR5L17N51095 vie chère outre-mer | Adopté définitivement 19/11 | Sénat adopté 5/03/2025 · AN conforme 19/11 · Accord même texte 19/11 · Loi promulgation attendue |
| DLR5L17N54096 CREPS de Vichy | Devenu loi · loi n° 2026-992 du 26/10, JO du 27/10 | Sénat adopté 15/06 · AN conforme 12/10 · Accord même texte 12/10 · Loi 26/10 |
| DLR5L17N53886 médico-social | Devenu loi · loi n° 2026-975 du 21/10, JO du 22/10 | Sénat adopté 28/05 · AN conforme 12/10 · Accord même texte 12/10 · Loi 21/10 |

Écart avec le canevas : la vie chère a un scrutin public sur l'ensemble (n° 10080, 19/11). En production, ce scrutin est rattaché à tort au dossier Territoires zéro chômeur. Ce bug est traité dans une session à part ; la mention « sans scrutin public » ne doit pas être reprise.

## 6. Découpage en lots

1. **Domaine et ingestion** : persister le sens et la date de chaque décision (`legislative_acts` : `statutConclusion`, migration). Écrire une fonction pure du domaine qui calcule l'état et les jalons à partir des décisions et de `LawPublication`. Persister le résumé de l'état sur `legislative_dossiers` (filtre et compteurs en SQL). Rejouer le calcul sur les dossiers déjà « finals », que le rafraîchissement saute aujourd'hui. Tests : les 5 témoins en fixtures JSON de `parsing.rs`, plus les cas CMP, lecture définitive, 49.3, rejet puis adoption, rejet seul.
2. **API et types TS** : `state` dans `DossierStatusDto` (liste, fiche de vote, semaine, fiche dossier), jalons sur le détail, filtre `state` et `state_counts` sur `/api/dossiers`, avancées dans `/api/semaine`. Types TS écrits à la main, unifiés autour d'un seul type d'état. Tests : DTO et fakes de dépôt.
3. **Fiche dossier et fiche de vote** : composants partagés `LawStateBadge`, `LawMilestones`, `LawAnswer`. Retirer `OutcomeBadge.tsx`, qui n'est plus utilisé. Puis mettre à jour les cadres `navette-retenue-*` et `navette-vote-*`.
4. **Liste par texte et filtres**, puis le cadre `navette-liste-*`.
5. **La semaine**, puis le cadre `navette-semaine-*`.
6. **Accueil**, puis le cadre `navette-accueil-*` (bureau : carte déplacée dans la colonne de droite).

Le frontend n'a pas d'outil de test : la logique d'état reste côté Rust. Pour le front, contrôle par `tsc`, lint et navigateur à 375 et 1280 px. Aucun commit ni push sans accord.
