# SUJET-PLAN-F1 — Lot A : cartes d'amendement et libellé court — ✅ DONE (2026-11-02)

> Lot F1 du plan `SUJET-PLAN.md`. Spec : `todo/sujet-amendements/SPEC-sujet-amendements.md`. Maquettes : rangée « Version 2 » de https://claude.ai/artifact/Dstm7rJCCA16zKqJg48p8H (`v2/Cartes`, `v2/Liste`, `v2/Mobile`).
> Lecture : **Intention**, **Conception**, **Décisions** disent pourquoi ; **Éléments de code** le détail des signatures ; **Cycles** les tests, les chemins exacts et l'ordre des cycles TDD.

## Intention <!-- kit:intent -->

Sans rapprochement, avec les données déjà en base : la carte d'amendement montre son sort, son auteur aux couleurs du groupe et l'aperçu de son exposé (CU-03 étapes 1 à 3 et 5), et chaque scrutin de la liste prend son libellé court (CU-02 étape 5). **RM** : RM-05, RM-08, RM-09, RM-10 | **CU** : CU-02, CU-03 | **Point technique** : T3 (lecture du numéro d'article)

## Conception <!-- kit:design -->

| Point | Décision |
|-------|----------|
| Applied rules <!-- kit:applied-rules --> | DDD-01, DDD-04, DDD-09, DDD-10, DDD-11, APP-01, APP-02, APP-04, PERF-01 |
| Agrégat propriétaire | VO `SummaryPreview`, `FateBanner` (`amendment.rs`), `ShortLabel` (`scrutin.rs`), `ArticleNumber` (`article.rs`) ; `ParliamentaryGroup::text_tone` (`actor.rs`) ; les cas d'usage assemblent, aucune règle dans les DTO ni dans l'interface |
| Invariants | RM-05 : 300 premiers caractères, coupure au milieu d'un mot comprise, « … » seulement si coupé ; RM-09 : quatre formes fixes, sinon l'objet tel quel ; RM-10 : couleur publiée du groupe au dépôt, jamais sur un élément de vote ; RM-08 : ordre de la source. Conséquence : aucun tri ni filtre ajouté |
| Cohérence | lectures seules |
| Événements | N/A — lectures seules |
| Coût d'accès | cartes : inchangé (page + couverture + groupes + référentiel des auteurs de la page) ; liste des scrutins : inchangé (1 page) |

## Décisions <!-- kit:decisions -->

### ADR-002 — Règles d'affichage d'un contenu publié calculées dans le domaine, servies par l'API
Choix : `SummaryPreview`, `FateBanner`, `ShortLabel` et `TextTone` sont calculés côté serveur et sérialisés ; l'interface affiche, elle ne coupe, ne reformule ni ne calcule de contraste.
Why : RM-05/RM-09/RM-10 sont des règles fixes et rejouables (README.md §5, §9) ; l'interface n'a aucun lanceur de tests : une règle y serait invérifiable.
Rejected : coupe côté interface (`displaySummary` existant, gardé seulement pour l'exposé entier déplié) ; contraste calculé en CSS.
Consequences : toute règle d'affichage d'un contenu publié se teste dans le domaine et arrive par le DTO ; F2 y ajoute les libellés de méthode et de cause.
Links : RM-05, RM-09, RM-10, T5

- T5 : couleur lue dans `ActorDirectory` à la construction de la vue, pour le groupe stocké au dépôt (AMENDEMENTS RM-02), jamais dénormalisée. La partie auteur de `view_of` (`browse_dossier_amendments.rs:76`) devient `pub(crate) fn author_view_of(..) -> AuthorView`, que F2 réutilise pour les scrutins.
- Bandeau de sort : `FateBanner` — libellé publié, ou « Sort non publié » suivi de l'état publié (CU-03 étape 3).
- Libellé court : appliqué à tous les scrutins tant qu'aucun lien n'existe (tous sont « non rapprochés », RM-09) ; F2 le remplace par le sujet pour les scrutins rapprochés. Le détail garde l'objet officiel entier.
- Doubles partagés (`.claude/rules/application.md` : jamais de double copié dans un autre fichier) : `read_fakes.rs` (`#[cfg(test)]`) crée `InMemoryScrutinReadRepository`, `InMemoryAmendmentReadRepository`, `StubActorRepository` ; les copies existantes des autres fichiers restent (refactor hors lot).
- Pastille et badge : composant partagé `AmendmentAuthor.tsx` (initiales, groupe, couleur ou gris neutre), forme de `GroupDot` (`pages/GroupListPage.tsx:14`).
- La page d'un dossier (`DossierScrutins.tsx`) rend `ScrutinList` : le libellé court s'y affiche sans fichier de plus (revue du plan, 2026-11-02).
- Interface sans tests (aucun lanceur) : vérifiée par `npm run lint`, `npm run build` et confrontation aux maquettes « Version 2 ».
- Dépôt Rust, tests inline : le fichier de test est le fichier de production ; `module.fonction` désigne `module::tests::fonction`. `list_scrutins.rs` et `browse_dossier_amendments.rs` reçoivent leur premier module `tests`. Aucun skill `tests-*` dans `.claude/skills/` : `skill` nomme `tests-unit-tests` par convention du kit.

Integration test waiver: aucun effet de persistance — aucune requête ni écriture ajoutée, seules les vues et les DTO changent.

## Politique de test et périmètres <!-- kit:test-policy -->

**Politique** : domaine testé dans son module ; requêtes testées avec les doubles partagés `read_fakes` alimentés, on observe la vue rendue. Ni espion ni compteur d'appels.

**Périmètre de régression ciblé** : `cargo test --locked --lib domain::amendment domain::scrutin domain::actor domain::article application::use_cases::browse_dossier_amendments application::use_cases::list_scrutins`.

**Vert global** : `cargo test --locked` ; `cargo clippy --all-targets -- -D warnings` ; `cd frontend && npm run lint && npm run build`.

**E2E** : omis — lectures sans cycle de vie.

## Éléments de code <!-- kit:code-elements -->

### Domain

**`SummaryPreview`** — _nouveau_ (`src/domain/amendment.rs`) : `pub enum SummaryPreview { Unpublished, Text { text: String, truncated: bool } }` ; `pub fn of(summary: Option<&str>) -> Self` ; `PREVIEW_LENGTH: usize = 300` ; `text` finit par « … » si et seulement si `truncated`

**`FateBanner`** — _nouveau_ (`src/domain/amendment.rs`) : `pub fn of(fate: FateCode, fate_label: &str, state_label: Option<&str>) -> Self` ; `pub fn label(&self) -> &str`

**`ShortLabel`** — _nouveau_ (`src/domain/scrutin.rs`) : `pub fn from_subject(subject: &str) -> Self` ; `as_str()` — « Vote sur l'article N », « Amendement n° N de [nom cité], article N », « Vote sur l'ensemble du texte », « Motion de [type] », sinon l'objet tel quel ; article lu par `ArticleNumber::in_text`

**`ArticleNumber`** — _nouveau_ (`src/domain/article.rs`) : `pub struct ArticleNumber(String)` + `Display` ; `pub fn in_text(text: &str) -> Option<Self>` — premier « article N » ; « 1er »/« premier » → « 1 » ; mention latine suivante gardée (« 15 bis ») ; la position (avant, après) n'est pas lue. Réutilisé par F2 pour l'article cité et l'article visé (T3)

**`TextTone`** — _nouveau_ (`src/domain/actor.rs`) : `Light`, `Dark`, `as_str()` (« white », « black ») ; **`ParliamentaryGroup::text_tone`** : `pub fn text_tone(&self) -> Option<TextTone>` — contraste sur `#RRGGBB` ; `None` sans couleur

### Application

**`AuthorView`** — _nouveau_ (`src/application/use_cases/browse_dossier_amendments.rs`) : `name`, `official_url`, `group_label`, `group_abbrev`, `group_color: Option<String>`, `text_tone: Option<TextTone>` ; `pub(crate) fn author_view_of(summary: &AmendmentSummary, directory: &ActorDirectory) -> AuthorView`

**`AmendmentView`** — _modifié_ : `summary`, `author: AuthorView`, `preview: SummaryPreview`, `fate_banner: FateBanner` ; `BrowseDossierAmendments::new` inchangé

**`ListScrutins::execute`** — _modifié_ : rend `ScrutinListPage { items: Vec<ScrutinListItem>, total }`, `ScrutinListItem { summary: ScrutinSummary, short_label: ShortLabel }`, ordre de la source ; `for_dossier` (`list_scrutins.rs:24`, sans appelant) inchangé

### API

**`AmendmentDto`** (`src/api/amendment_dto.rs:26`) : `author_group_color`, `author_text_tone`, `preview`, `preview_truncated`, `fate_banner` ; **`ScrutinSummaryDto`** (`src/api/scrutin_dto.rs:73`) : `short_label` ; `subject` conservé (RM-08)

### Frontend

**`AmendmentAuthor`** — _nouveau_ (`frontend/src/components/AmendmentAuthor.tsx`) ; **`DossierAmendments`** : bandeau de sort, auteur, aperçu, « Lire la suite » qui déplie exposé entier et signataires ; **`ScrutinList`** : libellé court en titre, objet officiel en ligne secondaire ; types `frontend/src/types/amendments.ts`, `frontend/src/types/scrutins.ts`

## Cycles

```json
{
 "declaratives": [
  "src/domain/mod.rs — after amendment; pub mod article;",
  "src/application/use_cases/mod.rs — after theme_fakes (#[cfg(test)]); #[cfg(test)] pub mod read_fakes;"
 ],
 "cycles": [
  {
   "id": "C1", "wave": 1,
   "behaviour": "L'aperçu, le bandeau de sort, le libellé court et le ton du texte se calculent par des règles fixes",
   "rmcu": "RM-05, RM-09, RM-10 / CU-02, CU-03",
   "steps": ["1. Une carte d'amendement montre son sort, son auteur aux couleurs de son groupe et l'aperçu de son exposé", "2. Un scrutin de la liste est intitulé par son libellé court"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "amendment", "fixture": "amendment::tests",
   "methods": [
    "amendment.should_show_whole_summary_without_ellipsis_when_it_has_300_characters_or_less",
    "amendment.should_cut_summary_at_300_characters_mid_word_and_mark_ellipsis_when_it_is_longer",
    "amendment.should_count_characters_not_bytes_when_summary_has_accents",
    "amendment.should_mark_summary_unpublished_when_amendment_has_none",
    "amendment.should_announce_unpublished_fate_with_published_state_when_source_gives_no_fate",
    "scrutin.should_label_article_vote_when_subject_puts_an_article_to_the_vote",
    "scrutin.should_label_amendment_with_cited_author_and_article_when_subject_names_them",
    "scrutin.should_label_whole_text_vote_when_subject_puts_the_whole_text_to_the_vote",
    "scrutin.should_label_motion_with_its_type_when_subject_is_a_motion",
    "scrutin.should_keep_subject_as_is_when_no_form_recognises_it",
    "article.should_read_cited_article_when_text_places_amendment_after_an_article",
    "article.should_read_article_one_when_text_writes_premier_or_1er",
    "article.should_keep_latin_mention_when_article_is_a_bis_one",
    "actor.should_write_white_on_group_color_when_color_is_dark",
    "actor.should_write_black_on_group_color_when_color_is_light",
    "actor.should_give_no_tone_when_group_publishes_no_color"
   ],
   "full": ["src/domain/amendment.rs", "src/domain/scrutin.rs", "src/domain/actor.rs", "src/domain/article.rs (to create)"],
   "bounded": ["src/domain/theme.rs"],
   "observation": "SummaryPreview, FateBanner.label(), ShortLabel.as_str(), ArticleNumber, Option<TextTone>",
   "scenarios": "Objet « l'amendement n° 1376 de Mme Thomin après l'article 15 du projet de loi … » → « Amendement n° 1376 de Mme Thomin, article 15 ». Exposé de 301 caractères avec accents : 300 caractères + « … ». « après l'article 15 » → 15 ; « article premier », « article 1er » → 1 ; « article 15 bis » → « 15 bis ».",
   "outOfScope": "assemblage des vues (C2, C3)",
   "toCreate": ["src/domain/article.rs"],
   "production": ["src/domain/amendment.rs", "src/domain/scrutin.rs", "src/domain/actor.rs", "src/domain/article.rs"],
   "ripple": [],
   "boundedGreen": ["src/domain/theme.rs"],
   "models": ["src/domain/amendment.rs — Fate et FateCode (lignes 134-230) avec leurs tests"]
  },
  {
   "id": "C2", "wave": 2,
   "behaviour": "Une carte d'amendement porte son sort, son auteur aux couleurs du groupe et l'aperçu de son exposé",
   "rmcu": "RM-05, RM-08, RM-10 / CU-03",
   "steps": ["1. Une carte d'amendement montre son sort, son auteur aux couleurs de son groupe et l'aperçu de son exposé"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "browse_dossier_amendments", "fixture": "read_fakes",
   "methods": [
    "browse_dossier_amendments.should_paint_author_in_group_color_when_source_publishes_one",
    "browse_dossier_amendments.should_give_no_color_when_group_publishes_none",
    "browse_dossier_amendments.should_give_summary_preview_when_amendment_has_a_summary",
    "browse_dossier_amendments.should_announce_unpublished_fate_when_source_gives_no_fate"
   ],
   "full": ["src/application/use_cases/browse_dossier_amendments.rs", "src/application/use_cases/read_fakes.rs (to create)"],
   "bounded": ["src/application/use_cases/get_scrutin_detail.rs", "src/application/ports/amendment_repository.rs"],
   "observation": "AmendmentView : AuthorView (couleur, ton), aperçu, bandeau de sort",
   "outOfScope": "renvoi aux scrutins (F2)",
   "toCreate": ["src/application/use_cases/read_fakes.rs", "frontend/src/components/AmendmentAuthor.tsx"],
   "production": [
    "src/application/use_cases/browse_dossier_amendments.rs",
    "src/application/use_cases/read_fakes.rs",
    "src/api/amendment_dto.rs",
    "frontend/src/types/amendments.ts",
    "frontend/src/components/DossierAmendments.tsx",
    "frontend/src/components/AmendmentAuthor.tsx"
   ],
   "ripple": [],
   "boundedGreen": ["src/domain/actor.rs", "frontend/src/pages/GroupListPage.tsx"],
   "models": ["src/application/use_cases/get_scrutin_detail.rs — doubles d'acteurs et tests de vue (lignes 230-400), dont la forme va dans read_fakes"]
  },
  {
   "id": "C3", "wave": 3,
   "behaviour": "Chaque scrutin de la liste porte son libellé court, dans l'ordre de la source",
   "rmcu": "RM-08, RM-09 / CU-02",
   "steps": ["2. Un scrutin de la liste est intitulé par son libellé court"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "list_scrutins", "fixture": "read_fakes",
   "methods": [
    "list_scrutins.should_give_each_scrutin_its_short_label_when_list_is_read",
    "list_scrutins.should_keep_source_order_and_total_when_list_is_read"
   ],
   "full": ["src/application/use_cases/list_scrutins.rs"],
   "bounded": ["src/application/use_cases/read_fakes.rs (to create in C2)", "src/application/ports/scrutin_repository.rs"],
   "observation": "ScrutinListPage : items dans l'ordre semé, short_label de chacun, total",
   "outOfScope": "sujet des scrutins rapprochés (F2)",
   "toCreate": [],
   "production": [
    "src/application/use_cases/list_scrutins.rs",
    "src/api/scrutin_dto.rs",
    "src/api/handlers/scrutin_handlers.rs",
    "frontend/src/types/scrutins.ts",
    "frontend/src/components/ScrutinList.tsx"
   ],
   "ripple": ["src/application/use_cases/read_fakes.rs (to create in C2) — page de scrutins semée"],
   "boundedGreen": ["frontend/src/components/DossierScrutins.tsx", "frontend/src/pages/ScrutinListPage.tsx"],
   "models": ["src/application/use_cases/browse_dossier_amendments.rs — vue assemblée par le cas d'usage, tests de C2"]
  }
 ]
}
```

## Hypothèses <!-- kit:assumptions -->

_Vide à la rédaction du plan._

| # | Assumption | To be validated by |
|---|------------|--------------------|
