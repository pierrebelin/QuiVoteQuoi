# SUJET-PLAN — Sujet des amendements mis aux voix

> Plan dérivé de `todo/sujet-amendements/SPEC-sujet-amendements.md`.
> Fiches de lot : `SUJET-PLAN-F1.md` (lot A), `SUJET-PLAN-F2.md` (lot B). En réserve, hors plan : `reserve/SUJET-RESERVE-RM07.md`, `reserve/SUJET-RESERVE-RM04.md`.
> Maquettes validées : rangée « Version 2 » de https://claude.ai/artifact/Dstm7rJCCA16zKqJg48p8H.
> Redécoupé le 2026-11-02 à la demande de Pierre Belin : valeur visible d'abord, rapprochement sur le dossier publié seul, RM-07 et RM-04 suspendus à une mesure.

## 0. Résumé <!-- kit:summary -->

**Avancement** : `0 / 2` (`0 %`)

### Batch execution

| Batch | Intent | RM/CU | Depends on | Repo | Execution | Reason |
|-------|--------|-------|------------|------|-----------|--------|
| F1 | Lot A — cartes d'amendement et liste des scrutins lisibles sans rapprochement | RM-05, RM-08, RM-09, RM-10 / CU-02, CU-03 | — | . | sequential | lit des données déjà en base ; valeur visible immédiate |
| F2 | Lot B — rapprochement sur le dossier publié, compteurs, sujet affiché sur les scrutins | RM-01, RM-02, RM-03, RM-04, RM-06, RM-08 / CU-01, CU-02, CU-03 | F1 | . | sequential | réutilise la vue d'auteur, l'aperçu et les doubles partagés de F1 |

### Batch F1 — Lot A : cartes d'amendement et libellé court — ✅ DONE (2026-11-02)
- [ ] 1. Une carte d'amendement montre son sort, son auteur aux couleurs de son groupe et l'aperçu de son exposé
- [ ] 2. Un scrutin de la liste est intitulé par son libellé court

### Batch F2 — Lot B : rapprochement et sujet des scrutins — ⬜
- [ ] 1. Un numéro cité désigne l'amendement unique de même numéro de référence, déposé au plus tard le jour du scrutin, départagé par l'article cité
- [ ] 2. Chaque passe recalcule tous les liens et note la cause de chaque absence de lien
- [ ] 3. La page méthode publie les compteurs du rapprochement
- [ ] 4. La liste et le détail d'un scrutin rapproché disent ce qui a été mis aux voix
- [ ] 5. Une carte d'amendement renvoie aux scrutins qui l'ont mis aux voix

## 1. Périmètre <!-- kit:scope -->

- **Spec** : `todo/sujet-amendements/SPEC-sujet-amendements.md` (delta sur `todo/SPEC-amendements.md`)
- **Contexte borné** : amendements et scrutins (lecture publique), thématisation pour la page méthode
- **Couches** : domaine, application, infrastructure (Postgres, migration), API, frontend React
- **Réutilisation** : `amendment_numbers_in_subject` (branché en F2, étendu au préfixe de partie) ; `AmendmentNumber`, `AmendmentTarget`, `FateCode` ; jointure de lecture `amendments.text_ref = dossier_documents.document_uid` (candidats de F2) ; `ParliamentaryGroup::color()` et `ActorDirectory` (F1) ; `normalize_source_text` déjà appliqué à l'ingestion ; `GroupDot`, `OutcomeBadge`, `TallyLine` côté interface
- **Hors périmètre** : spec §8 ; en-tête illustré des scrutins consécutifs (CU-02 étape 2, Q6, T6) — retiré le 2026-11-02, à rediscuter ; RM-07 et le recours au texte débattu de RM-04 — en réserve ; aucune route nouvelle ; pas de harnais Postgres d'intégration (plan d'alignement, lot 5)
- **Mesure avant reprise de la réserve** : requêtes en lecture seule `mesures-h1-h2.sql` (préparées pendant la planification, hors dépôt) — H1 : amendements de séance dont le texte n'est porté par aucun document de dossier ; H2 : scrutins d'amendement sans dossier publié dont le texte débattu mène à un seul dossier ; H3 : préfixes publiés. Les compteurs de F2 donnent la même information sur la base réelle (« sans dossier », « non résolu ») ; un gain net reprend la fiche en réserve correspondante
- **Hypothèses** : questions ouvertes 2 à 5 de la spec : recommandation de la spec retenue, `user decision to validate`, dans la fiche qui la porte ; Q6 sans objet (en-tête retiré). H3 : seuls « I- »/« II- » s'effacent du numéro de référence, tout autre préfixe reste distinctif (RM-01)
- **Règles non applicables** <!-- kit:na-rules --> : DDD-06: N/A — aucun agrégat réhydraté, les liens sont une projection relue telle quelle ; DDD-07: N/A — le dépôt n'a pas d'événements de domaine, la persistance passe par des écritures directes

## 2. Traçabilité <!-- kit:traceability -->

<!-- kit:cols item,owner,batch -->
| RM/CU | Code owner(s) | Batch |
|-------|---------------|-------|
| RM-05 — aperçu mécanique | `SummaryPreview::of` | F1 |
| RM-09 — libellé court | `ShortLabel::from_subject`, `ArticleNumber::in_text` ; `ListScrutins` | F1 |
| RM-10 — couleur du groupe | `ParliamentaryGroup::text_tone`, `AuthorView` | F1 |
| CU-03 — carte d'amendement | `BrowseDossierAmendments` (sort, auteur, aperçu) ; renvoi aux scrutins en F2 | F1, F2 |
| RM-08 — comportements préservés | ordre de la source inchangé, objet officiel conservé | F1, F2 |
| RM-01 — numéro de référence | `AmendmentNumber::reference` → `ReferenceNumber` | F2 |
| RM-02 — antériorité du dépôt | `ScrutinCitations::resolve` | F2 |
| RM-03 — départage par l'article cité | `ArticleNumber::in_text`, `AmendmentTarget::article`, `ScrutinCitations::resolve` | F2 |
| RM-04 — dossier de recherche | `SearchDossier` limité au dossier publié ; texte débattu en réserve | F2 |
| RM-06 — méthode stockée et affichée | `LinkMethod`, stocké par `replace_citations`, libellés du domaine | F2 |
| CU-01 — rapprocher les scrutins | `LinkScrutinsToAmendments` dans `RefreshAll` ; compteurs par `DescribeMethod` | F2 |
| CU-02 — sujet d'un scrutin | libellé court en F1 ; `ListScrutins`, `GetScrutinDetail` enrichis en F2 | F1, F2 |
| T1 — coût du recalcul intégral | ADR-001 — table de citations remplacée en une transaction | F2 |
| T2 — volume de la liste enrichie | aperçu seul en liste, exposé entier au détail, citations bornées à la page | F2 |
| T3 — lecture du numéro d'article | `ArticleNumber` : « 1er »/« premier » → 1, « bis », « ter »… gardés | F1 |
| T4 — branchement de la lecture des numéros cités | `amendment_numbers_in_subject` appelé par `ScrutinCitations::from_subject` | F2 |
| T5 — propagation de la couleur de groupe | ADR-002 ; couleur lue dans `ActorDirectory` à la vue | F1 |
| T6 — vignettes de l'en-tête | hors périmètre (en-tête retiré) | — |

## 3. Conception DDD et architecture <!-- kit:ddd-design -->

| RM/CU | Owning aggregate | Invariant / code consequence | Consistency | Batch |
|-------|------------------|------------------------------|-------------|-------|
| RM-05, RM-09, RM-10 | `SummaryPreview`, `ShortLabel`, `ParliamentaryGroup` | règle fixe et rejouable, aucun texte écrit par le site | lecture seule | F1 |
| RM-01 → RM-04, RM-06 | `ScrutinCitations` | exactement un candidat ou aucun lien ; jamais de « plus probable » | une passe = une transaction de remplacement | F2 |

## 4. Éléments transverses <!-- kit:cross-cutting -->

- **Câblage** : `src/lib.rs` (`AppState`) et `src/main.rs` reçoivent `PgAmendmentCitationRepository` (F2) ; `RefreshAll::new` prend le port en plus (F2) | **Routes** : aucune nouvelle ; `/api/scrutins`, `/api/scrutins/{uid}`, `/api/dossiers/{uid}/scrutins`, `/api/dossiers/{uid}/amendements`, `/api/themes/method` évoluent | **Bornes** (APP-05) : pagination existante ; lectures ajoutées bornées aux identifiants de la page
- **Migrations** : `migrations/` est dans ce dépôt (sqlx) — F2 crée `scrutin_amendment_citations` ; aucune autre évolution de schéma
