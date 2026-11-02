# SUJET-PLAN-F2 — Lot B : rapprochement et sujet des scrutins — ✅ DONE (2026-11-02)

> Lot F2 du plan `SUJET-PLAN.md`. Spec : `todo/sujet-amendements/SPEC-sujet-amendements.md`. Maquettes : rangée « Version 2 » (`v2/Liste`, `v2/Detail`, `v2/Cartes`).
> Lecture : **Intention**, **Conception**, **Décisions** disent pourquoi ; **Éléments de code** le détail des signatures ; **Cycles** les tests, les chemins exacts et l'ordre des cycles TDD.

## Intention <!-- kit:intent -->

À chaque rafraîchissement, chaque scrutin qui cite des amendements est relié, dans son dossier publié, à l'amendement unique que désigne chaque numéro cité ; la méthode est stockée, chaque absence de lien garde sa cause, la page méthode publie les compteurs (CU-01) ; liste, détail et cartes montrent le sujet (CU-02, CU-03). **RM** : RM-01, RM-02, RM-03, RM-04, RM-06, RM-08 | **CU** : CU-01, CU-02, CU-03

## Conception <!-- kit:design -->

| Point | Décision |
|-------|----------|
| Applied rules <!-- kit:applied-rules --> | DDD-01, DDD-02, DDD-03, DDD-04, DDD-05, DDD-08, DDD-09, DDD-10, DDD-11, APP-01, APP-02, APP-03, APP-04, APP-05, PERF-01 |
| Agrégat propriétaire | `ScrutinCitations` (citations d'un scrutin et leur résolution) ; VO `ReferenceNumber`, `SearchDossier` ; `LinkingReport` compte ; les cas d'usage orchestrent |
| Invariants | RM-01 : seuls « I- »/« II- » et la rectification s'effacent ; RM-02 : déposé après le scrutin → jamais candidat, sans date → candidat ; RM-03 : plusieurs candidats → seul l'article cité départage ; exactement un candidat restant ou aucun lien ; RM-04 : dossier publié seul (texte débattu en réserve). Conséquence : aucun score, aucun tri par date, aucune seconde lecture pour départager |
| Cohérence | DDD-08, exception assumée : une passe remplace toutes les citations en une transaction — projection recalculée, sans état propre ; un échec laisse la passe précédente intacte (CU-01 Erreurs) |
| Événements | N/A — aucun événement de domaine dans le dépôt, écriture directe |
| Coût d'accès | passe : 2 lectures (scrutins, candidats des dossiers publiés cités) + 1 écriture transactionnelle, indépendantes du nombre de scrutins ; compteurs : 1 lecture agrégée ; liste : + 1 citations de la page + 1 référentiel ; détail : + 1 citations + 1 référentiel ; cartes : + 1 scrutins des amendements de la page |

## Décisions <!-- kit:decisions -->

### ADR-001 — Citations scrutin–amendement stockées dans une table recalculée intégralement
Choix : table `scrutin_amendment_citations`, une ligne par numéro cité (scrutin, rang de citation, numéro cité, amendement relié ou cause, `by_cited_article`) ; une ligne de rang 0 sans numéro pour « sans numéro » ; amendement présent si et seulement si cause absente ; clés étrangères en cascade vers `scrutins` et `amendments` ; index sur l'amendement. Chaque passe supprime tout puis insère en lot dans une transaction.
Why : T1 — 9 500 scrutins et 100 000 amendements tiennent en 2 lectures et 1 écriture ; CU-01 étape 6 veut un recalcul intégral et les liens précédents servis en cas d'échec.
Rejected : liens calculés à la lecture (coût par page, compteurs impossibles sans tout relire) ; mise à jour incrémentale (liens périmés quand un amendement arrive après coup).
Consequences : une règle de rapprochement change par une nouvelle passe, jamais par une correction en base ; un lecteur des liens lit cette table, jamais l'objet du scrutin ; la reprise de RM-04 ajoute la colonne `through_debated_text` par migration.
Links : T1, CU-01, RM-06

### Compteurs : une seule formule, dans le domaine, sur la page méthode des thèmes
Choix : la base rend des effectifs groupés par (cause, `by_cited_article`) ; `LinkingReport::from_counts` les somme, la passe compte ses résultats par la même fonction. `DescribeMethod` rend le rapport de thématisation et ce rapport ; `get_method` n'appelle que lui (`.claude/rules/api.md` : un handler n'enchaîne jamais deux cas d'usage) ; `ThemeMethodPage.tsx` affiche une section « Rapprochement scrutins–amendements ». `user decision to validate` — la spec dit « la page méthode » sans la nommer ; `/api/dataset` n'est lu par aucune page.
Why : README.md §6 — une seule formule par chiffre, testée ; seule page de méthode servie depuis la base.
Rejected : `COUNT` par compteur en SQL ; `/api/dataset` ; route nouvelle.
Links : CU-01 étape 6

- Q2 : candidat unique dont l'article visé et l'article cité sont connus et diffèrent → pas de lien, cause « non résolu ». `user decision to validate`.
- Q3 : la passe tourne même si l'ingestion des amendements a échoué, sur les amendements en base. `user decision to validate`.
- Q4 : un préfixe cité (« n° II-243 ») doit correspondre ; sans préfixe, il répond aux deux parties, l'article départage. `user decision to validate`.
- Q5 : seul un objet nommant un amendement reçoit une citation (et donc la cause « sans numéro ») — prédicat de `ScrutinCitations::from_subject`, testé ; `citing_scrutins` lit tous les scrutins, sans filtre SQL. `user decision to validate`.
- Q1 (nouvelle lecture) : deux candidats de même numéro et même article restent « ambigus » ; les compteurs mesurent le phénomène.
- RM-04 réduit au dossier publié : `SearchDossier::{Published, Missing}` ; la variante via le texte débattu est en réserve (`reserve/SUJET-RESERVE-RM04.md`). Le compteur « sans dossier » mesure ce qu'elle rapporterait (H2) ; « non résolu » inclut les amendements de séance hors dossier (H1, `reserve/SUJET-RESERVE-RM07.md`) et ceux pas encore ingérés (ingestion plafonnée par passe) — la page méthode le dit sous le compteur.
- `resolve` déduplique les candidats par `AmendmentUid` (`dossier_documents` sans unicité, la jointure peut rendre deux fois un amendement) ; `reference()` et `answers` comparent une forme minuscule (objet lu en minuscules, numéro stocké en casse publiée) ; un seul article cité vaut pour tous les numéros d'un objet (la suite de numéros s'arrête au premier mot non numérique).
- T4 : `amendment_numbers_in_subject` est branché ici et garde « I- »/« II- » ; ses 7 tests actuels restent valides.
- Port dédié `AmendmentCitationRepository` plutôt qu'une extension d'`AmendmentRepository` ou de `ScrutinRepository` : il croise scrutins et amendements, et éviterait de compléter 3 + 4 doubles copiés.
- Doubles : `citation_fakes.rs` (`#[cfg(test)]`, nouveau) et `read_fakes.rs` (F1) ; jamais de double copié.
- Vue citée : `CitedAmendmentView` réutilise `AuthorView` et `SummaryPreview` de F1, porte le `dossier_uid` de l'amendement ; le détail renvoie à `/dossiers/{dossier_uid}?amendement={numéro}#amendement-{numéro}` (ancre posée sur la carte, filtre par numéro pour ouvrir la bonne page). Lien officiel absent tant que AMENDEMENTS H8 n'est pas confirmée.
- Libellés RM-06 et de cause dans le domaine (ADR-002 de F1) ; variante « amendements identiques suivants » non numérotés : `SubjectMentions::unnumbered_identical_amendments`, dite sur le détail.
- `ShortLabel` (F1) reste le titre des scrutins non rapprochés ; un scrutin rapproché prend le sujet, objet officiel en ligne secondaire (RM-08).
- Revue du plan, 2026-11-02 :
  - Méthodes du port par cycle : C3 crée le port avec `citing_scrutins` et `candidates_in`, C4 ajoute `replace_citations`, C6 `citation_counts`, C7 `citations_of`, C9 `scrutins_of_amendments` ; chaque cycle n'implémente que les siennes dans l'adaptateur et `citation_fakes`.
  - `CitedAmendmentView` naît en C7 (`browse_dossier_amendments.rs` dans sa `production`), C8 et C9 le réutilisent.
  - La page d'un dossier (`DossierScrutins.tsx`) rend `ScrutinList` : libellé court (F1) et sujet (C7) s'y affichent sans fichier de plus.
  - Q2 à Q5 restent des décisions marquées `user decision to validate` et non des lignes `## Hypothèses` : ce tableau est réservé au run (gabarit du kit).
- `RefreshAll::new` passe à 13 paramètres (déjà 12 ; structure d'entrée hors lot).
- Aucun cycle `guard` : chaque méthode est rouge avant tout code de production (types absents).
- Écart à la règle d'intégration, gardé : aucun harnais Postgres ; les requêtes sont des projections et une écriture en lot sans règle ; le mapping ligne → `CitingScrutin` est une fonction pure testée inline (`.claude/rules/infrastructure.md`).
- Dépôt Rust, tests inline : le fichier de test est le fichier de production ; `module.fonction` désigne `module::tests::fonction`.

Integration test waiver: aucun harnais Postgres dans le dépôt — `citing_scrutins`, `candidates_in`, `replace_citations`, `citation_counts`, `citations_of`, `scrutins_of_amendments` et le filtre par numéro de `PgAmendmentRepository::by_dossier` sont des projections et une écriture en lot sans règle ; les règles sont testées dans le domaine, le mapping et les cas d'usage.

## Politique de test et périmètres <!-- kit:test-policy -->

**Politique** : domaine testé dans son module ; cas d'usage testés avec `citation_fakes`, `read_fakes`, `theme_fakes` alimentés ; on observe citations stockées, rapports et vues. Ni espion ni compteur d'appels.

**Périmètre de régression ciblé** : `cargo test --locked --lib domain:: infrastructure::persistence::pg_amendment_citation_repository application::use_cases::link_scrutins_to_amendments application::use_cases::refresh_all application::use_cases::describe_method application::use_cases::list_scrutins application::use_cases::get_scrutin_detail application::use_cases::browse_dossier_amendments`.

**Vert global** : `cargo test --locked` (dont `tests/write_routes_guard.rs`, qui construit `AppState`) ; `cargo clippy --all-targets -- -D warnings` ; `cd frontend && npm run lint && npm run build`.

**E2E** : omis — passe puis lecture couvertes par les cas d'usage.

## Éléments de code <!-- kit:code-elements -->

### Domain

**`ReferenceNumber`** — _nouveau_ (`src/domain/amendment.rs`) : `{ part: Option<FinancePart>, number: String }`, `FinancePart { First, Second }` ; `pub fn answers(&self, cited: &ReferenceNumber) -> bool` — même numéro, partie citée absente ou égale ; **`AmendmentNumber::reference`** : forme minuscule, retire « I- »/« II- » et la rectification, garde tout autre préfixe ; **`amendment_numbers_in_subject`** — _modifié_ : accepte « I-243 »/« II-243 » dans la suite ; **`AmendmentTarget::article`** : `pub fn article(&self) -> Option<ArticleNumber>`

**`SearchDossier`** — _nouveau_ (`src/domain/amendment_citation.rs`) : `Published(DossierUid)`, `Missing` ; `pub fn for_scrutin(published: Option<&DossierUid>) -> Self`

**`CandidateAmendment`** : `pub fn new(uid: AmendmentUid, dossier: DossierUid, number: AmendmentNumber, target: &AmendmentTarget, deposited_on: Option<NaiveDate>) -> Self`

**`ScrutinCitations`** : `pub fn from_subject(scrutin: ScrutinUid, date: NaiveDate, subject: &str, search: SearchDossier) -> Option<Self>` — `None` quand l'objet ne nomme aucun amendement ; numéros cités dans l'ordre, article cité par `ArticleNumber::in_text` ; `pub fn resolve(&self, candidates: &[CandidateAmendment]) -> Vec<CitationOutcome>` — candidats du dossier de recherche dédupliqués par uid ; `answers` (RM-01) ; dépôt ≤ date (RM-02) ; plusieurs → article cité (RM-03) ; un seul d'article différent → non résolu (Q2)

**`CitationOutcome`** : `rank: u16`, `cited_number: Option<AmendmentNumber>`, `result: CitationResult` ; `CitationResult::{Linked { amendment: AmendmentUid, method: LinkMethod }, Unlinked(UnlinkedCause)}` ; **`LinkMethod`** `{ by_cited_article: bool }`, `labels() -> Vec<&'static str>` (RM-06) ; **`UnlinkedCause`** `NoNumber`, `NoDossier`, `Unresolved`, `Ambiguous`, `as_str()`, `parse()`, `label()`

**`LinkingReport`** : `linked`, `linked_by_cited_article`, `no_number`, `no_dossier`, `unresolved`, `ambiguous` (`i64`) ; `from_counts(&[CitationCount])`, `from_outcomes(&[CitationOutcome])` via `from_counts` ; `CitationCount { cause: Option<UnlinkedCause>, by_cited_article: bool, count: i64 }`

**`SubjectMentions`** — _nouveau_ (`src/domain/scrutin.rs`) : `pub fn unnumbered_identical_amendments(subject: &str) -> bool`

### Application

**`AmendmentCitationRepository`** — _nouveau_ (`src/application/ports/amendment_citation_repository.rs`), `#[async_trait] pub trait AmendmentCitationRepository: Send + Sync` :
- `citing_scrutins() -> Vec<CitingScrutin>` — `CitingScrutin { uid: ScrutinUid, date: NaiveDate, subject: String, published_dossier: Option<DossierUid> }`
- `candidates_in(&[DossierUid]) -> Vec<CandidateAmendment>` — vide = aucune lecture
- `replace_citations(&[StoredCitation]) -> usize` — `StoredCitation { scrutin: ScrutinUid, outcome: CitationOutcome }`
- `citation_counts() -> Vec<CitationCount>`
- `citations_of(&[ScrutinUid]) -> Vec<CitationRow>` — `CitationRow { scrutin, rank, cited_number: Option<String>, amendment: Option<CitedAmendmentRow>, cause: Option<UnlinkedCause>, method: LinkMethod }`, `CitedAmendmentRow { dossier_uid: DossierUid, summary: AmendmentSummary }`
- `scrutins_of_amendments(&[AmendmentUid]) -> Vec<AmendmentVote>` — `AmendmentVote { amendment, scrutin_uid, scrutin_number, date, tally: VoteTally }`, chronologique
- tous `Result<_, RepositoryError>` (`ports/scrutin_repository.rs`)

**`LinkScrutinsToAmendments`** — _nouveau_ (`link_scrutins_to_amendments.rs`) : `new(citations)` ; `execute() -> Result<LinkingReport, RepositoryError>` : `citing_scrutins` → `SearchDossier::for_scrutin` → `ScrutinCitations::from_subject` → `candidates_in` (dossiers distincts) → `resolve` → `replace_citations` → `LinkingReport::from_outcomes`

**`RefreshAll`** — _modifié_ : `new(.., citation_repository)` ; étape après les amendements, quel que soit leur résultat (Q3) ; `RefreshOutcome.citations: Option<LinkingReport>`, `citations_anomaly: Option<String>`

**`DescribeMethod`** — _nouveau_ (`describe_method.rs`) : `new(themes, citations)` ; `execute() -> Result<MethodOverview, RepositoryError>`, `MethodOverview { report: MethodReport, linking: LinkingReport }`

**`CitedAmendmentView`** — _nouveau_ (`browse_dossier_amendments.rs`) : `rank`, `cited_number`, `dossier_uid`, `number`, `target_title`, `author: AuthorView`, `cosignatory_count`, `preview: SummaryPreview`, `summary: Option<String>` (détail seulement), `method` ou `cause`

**`ListScrutins::execute`** — _modifié_ : `new(repository, citations, actors)` ; `ScrutinListItem` gagne `cited: Vec<CitedAmendmentView>` ; **`GetScrutinDetail`** : `new(repository, actor_repository, citations)`, `ScrutinDetail` gagne `cited` et `unnumbered_identical_amendments: bool` ; **`BrowseDossierAmendments`** : `new(amendments, actors, citations)`, `AmendmentView.votes: Vec<AmendmentVote>` (vide = jamais rapproché), `AmendmentPageRequest` filtre sur un numéro

### Infrastructure

**`PgAmendmentCitationRepository`** — _nouveau_ (`src/infrastructure/persistence/pg_amendment_citation_repository.rs`) : `new(pool)` ; les six méthodes ; `fn citing_scrutin_from_row(..) -> CitingScrutin` pure (`dossier_uid` vide → `None` + `tracing::warn!`) ; `replace_citations` en transaction (suppression puis `UNNEST`) ; modèle `pg_theme_repository.rs`

**Migration** — `migrations/20261103000000_create_scrutin_amendment_citations.sql` : table et contraintes d'ADR-001

### API, câblage, frontend

**`RefreshResponse`** (`src/api/dto.rs:345`) : `citations`, `citations_anomaly` ; **`MethodResponse`** (`src/api/theme_dto.rs:257`) : `amendment_linking`, `From<MethodOverview>` ; **`get_method`** (`theme_handlers.rs:28`) appelle `DescribeMethod` seul ; **`ScrutinSummaryDto`** : `cited` ; **`ScrutinDetailDto`** : `cited` (avec `summary`, `method_labels`, `cause_label`), `unnumbered_identical_amendments` ; **`AmendmentDto`** : `votes` ; **`AppState`** (`src/lib.rs:28`) : `amendment_citation_repository`, construit dans `src/main.rs`
Frontend : `ThemeMethodPage.tsx` (compteurs), `ScrutinList.tsx` (sujet du scrutin rapproché), `ScrutinDetailPage.tsx` (« Ce qui a été mis aux voix », méthode ou cause), `DossierAmendments.tsx` (ancre par numéro, pied « Mis aux voix · scrutin n° X » ou « Aucun scrutin public rapproché ») ; types `themes.ts`, `scrutins.ts`, `amendments.ts`

## Cycles

```json
{
 "declaratives": [
  "src/domain/mod.rs — after amendment; pub mod amendment_citation;",
  "src/application/ports/mod.rs — after amendment_repository; pub mod amendment_citation_repository;",
  "src/infrastructure/persistence/mod.rs — after pg_amendment_repository; pub mod pg_amendment_citation_repository;",
  "src/application/use_cases/mod.rs — after list_scrutins; pub mod link_scrutins_to_amendments;",
  "src/application/use_cases/mod.rs — after describe_dataset; pub mod describe_method;",
  "src/application/use_cases/mod.rs — after read_fakes (#[cfg(test)]); #[cfg(test)] pub mod citation_fakes;"
 ],
 "cycles": [
  {
   "id": "C1", "wave": 1,
   "behaviour": "Le numéro de référence d'un amendement se lit par une règle fixe",
   "rmcu": "RM-01, RM-03 / CU-01",
   "steps": ["1. Un numéro cité désigne l'amendement unique de même numéro de référence, déposé au plus tard le jour du scrutin, départagé par l'article cité"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "amendment", "fixture": "amendment::tests",
   "methods": [
    "amendment.should_drop_finance_part_from_reference_number_when_number_is_prefixed",
    "amendment.should_drop_rectification_from_reference_number_when_number_is_rectified",
    "amendment.should_keep_commission_prefix_in_reference_number_when_number_is_a_commission_one",
    "amendment.should_answer_cited_number_whatever_its_case_when_stored_number_is_upper_case",
    "amendment.should_keep_cited_finance_part_when_subject_cites_a_prefixed_number",
    "amendment.should_keep_reading_number_run_when_it_holds_finance_parts",
    "amendment.should_read_target_article_when_target_title_names_one"
   ],
   "full": ["src/domain/amendment.rs"],
   "bounded": ["src/domain/article.rs"],
   "observation": "ReferenceNumber rendu, answers, numéros rendus par amendment_numbers_in_subject, article visé",
   "scenarios": "« I-1376 » → (First, 1376) ; « 45 rect. » et « 45 rectifié » → (None, 45) ; « CS1376 » ne répond pas à 1376 ; stocké « II-243 » répond à cité « ii-243 » ; « l'amendement n° II-243 de M. X après l'article 15 » → [II-243] ; « n° I-12 et II-45 » → [I-12, II-45] ; « APRÈS L'ARTICLE 12 » → 12. Les 7 tests existants d'amendment_numbers_in_subject restent verts.",
   "outOfScope": "choix du candidat (C2)",
   "toCreate": [],
   "production": ["src/domain/amendment.rs"],
   "ripple": [],
   "boundedGreen": ["src/domain/article.rs"],
   "models": ["src/domain/amendment.rs — AmendmentNumber::key et amendment_numbers_in_subject, avec leurs tests"]
  },
  {
   "id": "C2", "wave": 2,
   "behaviour": "Chaque numéro cité d'un scrutin se résout en un amendement unique ou en une cause d'absence de lien",
   "rmcu": "RM-01, RM-02, RM-03, RM-04, RM-06 / CU-01",
   "steps": ["1. Un numéro cité désigne l'amendement unique de même numéro de référence, déposé au plus tard le jour du scrutin, départagé par l'article cité"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "amendment_citation", "fixture": "amendment_citation::tests",
   "methods": [
    "amendment_citation.should_link_cited_number_when_a_single_candidate_shares_its_reference_number",
    "amendment_citation.should_link_each_cited_number_in_citation_order_when_subject_cites_several",
    "amendment_citation.should_ignore_candidate_when_it_was_deposited_after_the_scrutin",
    "amendment_citation.should_keep_candidate_when_its_deposit_date_is_unknown",
    "amendment_citation.should_link_by_cited_article_when_several_candidates_share_the_number",
    "amendment_citation.should_mark_ambiguous_when_cited_article_does_not_separate_candidates",
    "amendment_citation.should_mark_ambiguous_when_subject_cites_no_article_and_candidates_are_several",
    "amendment_citation.should_mark_unresolved_when_single_candidate_targets_another_article",
    "amendment_citation.should_mark_unresolved_when_no_candidate_shares_the_number",
    "amendment_citation.should_link_single_amendment_when_join_lists_it_twice",
    "amendment_citation.should_mark_no_dossier_when_source_attaches_no_dossier",
    "amendment_citation.should_mark_no_number_when_subject_names_an_amendment_without_number",
    "amendment_citation.should_give_no_citation_when_subject_names_no_amendment",
    "amendment_citation.should_name_cited_number_and_article_method_when_link_used_them",
    "amendment_citation.should_name_each_cause_when_number_is_not_linked",
    "amendment_citation.should_count_links_by_method_and_absences_by_cause_when_outcomes_are_summed",
    "scrutin.should_detect_unnumbered_identical_amendments_when_subject_announces_them"
   ],
   "full": ["src/domain/amendment_citation.rs (to create)", "src/domain/scrutin.rs"],
   "bounded": ["src/domain/amendment.rs", "src/domain/article.rs", "src/domain/dossier.rs"],
   "observation": "Option<ScrutinCitations>, Vec<CitationOutcome>, libellés, LinkingReport",
   "scenarios": "Cas de la spec : « n° 243 … après l'article 15 », candidats I-243 (« Après l'article 15 ») et II-243 (« Article 52 ») → lien I-243, by_cited_article. Candidat d'un autre dossier : jamais candidat. « amendements identiques n° 12 et 45 … après l'article 3 » : l'article 3 vaut pour les deux.",
   "outOfScope": "lecture et écriture des citations (C3, C4)",
   "toCreate": ["src/domain/amendment_citation.rs"],
   "production": ["src/domain/amendment_citation.rs", "src/domain/scrutin.rs"],
   "ripple": [],
   "boundedGreen": [],
   "models": ["src/domain/theme.rs — VO à variantes nommées et tests inline"]
  },
  {
   "id": "C3", "wave": 3,
   "behaviour": "Une ligne lue en base devient un scrutin citant, dossier vide écarté sans erreur",
   "rmcu": "RM-04 / CU-01",
   "steps": ["2. Chaque passe recalcule tous les liens et note la cause de chaque absence de lien"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "pg_amendment_citation_repository", "fixture": "pg_amendment_citation_repository::tests",
   "methods": [
    "pg_amendment_citation_repository.should_keep_published_dossier_when_row_carries_one",
    "pg_amendment_citation_repository.should_read_no_published_dossier_when_row_carries_an_empty_one"
   ],
   "full": ["src/infrastructure/persistence/pg_amendment_citation_repository.rs (to create)"],
   "bounded": ["src/domain/dossier.rs"],
   "observation": "CitingScrutin rendu par citing_scrutin_from_row",
   "outOfScope": "requêtes SQL (waiver)",
   "toCreate": ["src/application/ports/amendment_citation_repository.rs", "src/infrastructure/persistence/pg_amendment_citation_repository.rs"],
   "production": ["src/application/ports/amendment_citation_repository.rs", "src/infrastructure/persistence/pg_amendment_citation_repository.rs"],
   "ripple": [],
   "boundedGreen": ["src/infrastructure/persistence/pg_theme_repository.rs"],
   "models": ["src/infrastructure/persistence/pg_theme_repository.rs — lecture agrégée et mapping d'erreur db(e)"]
  },
  {
   "id": "C4", "wave": 4,
   "behaviour": "Une passe de rapprochement remplace toutes les citations et rend ses compteurs",
   "rmcu": "RM-06 / CU-01",
   "steps": ["2. Chaque passe recalcule tous les liens et note la cause de chaque absence de lien"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "link_scrutins_to_amendments", "fixture": "citation_fakes::InMemoryCitationRepository",
   "methods": [
    "link_scrutins_to_amendments.should_replace_all_citations_with_this_pass_outcomes_when_pass_completes",
    "link_scrutins_to_amendments.should_search_candidates_only_in_published_dossiers_when_pass_reads_amendments",
    "link_scrutins_to_amendments.should_keep_previous_citations_when_candidates_cannot_be_read",
    "link_scrutins_to_amendments.should_report_links_by_method_and_absences_by_cause_when_pass_completes"
   ],
   "full": ["src/application/use_cases/link_scrutins_to_amendments.rs (to create)", "src/application/use_cases/citation_fakes.rs (to create)"],
   "bounded": ["src/application/use_cases/theme_fakes.rs", "src/domain/amendment_citation.rs (nouveau, créé en C2)"],
   "observation": "citations stockées dans le double après la passe ; LinkingReport ; citations antérieures intactes sur erreur",
   "outOfScope": "étape du rafraîchissement global (C5)",
   "toCreate": ["src/application/use_cases/link_scrutins_to_amendments.rs", "src/application/use_cases/citation_fakes.rs", "migrations/20261103000000_create_scrutin_amendment_citations.sql"],
   "production": ["src/application/use_cases/link_scrutins_to_amendments.rs", "src/application/use_cases/citation_fakes.rs", "src/application/ports/amendment_citation_repository.rs (nouveau, créé en C3 ; replace_citations ajouté)", "src/infrastructure/persistence/pg_amendment_citation_repository.rs (nouveau, créé en C3 ; replace_citations en transaction)", "migrations/20261103000000_create_scrutin_amendment_citations.sql"],
   "ripple": [],
   "boundedGreen": ["migrations/20260810000000_create_amendments.sql"],
   "models": ["src/application/use_cases/extract_debated_texts.rs — passe déterministe sur tous les scrutins, avec ses tests", "src/application/use_cases/theme_fakes.rs — double en mémoire partagé"]
  },
  {
   "id": "C5", "wave": 5,
   "behaviour": "Le rafraîchissement relie les scrutins aux amendements après l'ingestion, même quand elle échoue",
   "rmcu": "CU-01",
   "steps": ["2. Chaque passe recalcule tous les liens et note la cause de chaque absence de lien"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "refresh_all", "fixture": "refresh_all::tests",
   "methods": [
    "refresh_all.should_link_scrutins_to_amendments_when_amendment_ingestion_failed",
    "refresh_all.should_report_linking_anomaly_without_failing_refresh_when_linking_fails"
   ],
   "full": ["src/application/use_cases/refresh_all.rs"],
   "bounded": ["src/application/use_cases/citation_fakes.rs (nouveau, créé en C4)", "src/application/use_cases/link_scrutins_to_amendments.rs (nouveau, créé en C4)"],
   "observation": "RefreshOutcome.citations et citations_anomaly",
   "outOfScope": "affichage des compteurs (C6)",
   "toCreate": [],
   "production": ["src/application/use_cases/refresh_all.rs", "src/api/dto.rs", "src/api/handlers/dossier_handlers.rs", "src/lib.rs", "src/main.rs"],
   "ripple": ["src/application/use_cases/refresh_all.rs — appel RefreshAll::new des tests (ligne 456) : citation_fakes en 13e argument", "tests/write_routes_guard.rs — AppState gagne amendment_citation_repository (PgAmendmentCitationRepository sur le pool paresseux)"],
   "boundedGreen": ["src/api/routes.rs"],
   "models": ["src/application/use_cases/refresh_all.rs — étape des amendements (lignes 185-206) et son anomalie"]
  },
  {
   "id": "C6", "wave": 6,
   "behaviour": "La page méthode publie les liens établis par méthode et les absences de lien par cause",
   "rmcu": "CU-01",
   "steps": ["3. La page méthode publie les compteurs du rapprochement"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "describe_method", "fixture": "citation_fakes::InMemoryCitationRepository",
   "methods": ["describe_method.should_publish_links_by_method_and_absences_by_cause_beside_theme_report_when_citations_are_stored"],
   "full": ["src/application/use_cases/describe_method.rs (to create)"],
   "bounded": ["src/application/use_cases/citation_fakes.rs (nouveau, créé en C4)", "src/application/use_cases/theme_fakes.rs"],
   "observation": "MethodOverview : rapport de thématisation inchangé et LinkingReport",
   "outOfScope": "—",
   "toCreate": ["src/application/use_cases/describe_method.rs"],
   "production": ["src/application/use_cases/describe_method.rs", "src/api/theme_dto.rs", "src/api/handlers/theme_handlers.rs", "frontend/src/types/themes.ts", "frontend/src/pages/ThemeMethodPage.tsx"],
   "ripple": ["src/application/ports/amendment_citation_repository.rs (nouveau, créé en C3) — citation_counts ajouté au trait", "src/infrastructure/persistence/pg_amendment_citation_repository.rs (nouveau, créé en C3) — citation_counts : effectifs groupés par (cause, by_cited_article)", "src/application/use_cases/citation_fakes.rs (nouveau, créé en C4) — citation_counts servi depuis les citations stockées"],
   "boundedGreen": ["src/application/use_cases/browse_themes.rs", "src/domain/amendment_citation.rs (nouveau, créé en C2)"],
   "models": ["src/application/use_cases/describe_dataset.rs — cas d'usage de chiffres publiés, avec ses tests"]
  },
  {
   "id": "C7", "wave": 7,
   "behaviour": "La liste des scrutins donne le sujet d'un scrutin rapproché, sinon son libellé court, dans l'ordre de la source",
   "rmcu": "RM-05, RM-08, RM-10 / CU-02",
   "steps": ["4. La liste et le détail d'un scrutin rapproché disent ce qui a été mis aux voix"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "list_scrutins", "fixture": "read_fakes",
   "methods": [
    "list_scrutins.should_present_cited_amendment_with_author_article_and_preview_when_scrutin_is_linked",
    "list_scrutins.should_list_cited_amendments_in_citation_order_when_subject_cites_several",
    "list_scrutins.should_give_no_cited_amendment_when_scrutin_is_not_linked",
    "list_scrutins.should_keep_source_order_when_some_scrutins_are_linked_and_others_not"
   ],
   "full": ["src/application/use_cases/list_scrutins.rs"],
   "bounded": ["src/application/use_cases/read_fakes.rs", "src/application/use_cases/citation_fakes.rs (nouveau, créé en C4)", "src/application/use_cases/browse_dossier_amendments.rs"],
   "observation": "ScrutinListItem.cited et short_label, ordre",
   "outOfScope": "détail (C8)",
   "toCreate": [],
   "production": ["src/application/use_cases/list_scrutins.rs", "src/application/use_cases/browse_dossier_amendments.rs", "src/application/ports/amendment_citation_repository.rs (nouveau, créé en C3 ; citations_of ajouté)", "src/infrastructure/persistence/pg_amendment_citation_repository.rs (nouveau, créé en C3 ; citations_of)", "src/api/scrutin_dto.rs", "src/api/handlers/scrutin_handlers.rs", "frontend/src/types/scrutins.ts", "frontend/src/components/ScrutinList.tsx"],
   "ripple": ["src/application/use_cases/citation_fakes.rs (nouveau, créé en C4) — citations_of servi depuis les données semées"],
   "boundedGreen": ["frontend/src/components/AmendmentAuthor.tsx", "frontend/src/components/DossierScrutins.tsx"],
   "models": ["src/application/use_cases/browse_dossier_amendments.rs — référentiel chargé pour les seuls auteurs de la page"]
  },
  {
   "id": "C8", "wave": 8,
   "behaviour": "Le détail d'un scrutin dit ce qui a été mis aux voix, avec sa méthode ou la cause de chaque absence de lien",
   "rmcu": "RM-06, RM-08 / CU-02",
   "steps": ["4. La liste et le détail d'un scrutin rapproché disent ce qui a été mis aux voix"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "get_scrutin_detail", "fixture": "get_scrutin_detail::tests",
   "methods": [
    "get_scrutin_detail.should_tell_what_was_put_to_the_vote_with_whole_summary_and_method_when_scrutin_is_linked",
    "get_scrutin_detail.should_give_each_unlinked_number_its_cause_when_subject_cites_several",
    "get_scrutin_detail.should_give_no_cited_amendment_when_subject_names_no_amendment",
    "get_scrutin_detail.should_say_identical_amendments_are_unnumbered_when_subject_announces_them_without_numbers"
   ],
   "full": ["src/application/use_cases/get_scrutin_detail.rs"],
   "bounded": ["src/application/use_cases/citation_fakes.rs (nouveau, créé en C4)", "src/application/use_cases/read_fakes.rs", "src/application/use_cases/browse_dossier_amendments.rs"],
   "observation": "ScrutinDetail.cited (exposé entier, dossier de l'amendement, libellés), unnumbered_identical_amendments",
   "outOfScope": "—",
   "toCreate": [],
   "production": ["src/application/use_cases/get_scrutin_detail.rs", "src/api/scrutin_dto.rs", "src/api/handlers/scrutin_handlers.rs", "frontend/src/types/scrutins.ts", "frontend/src/pages/ScrutinDetailPage.tsx"],
   "ripple": ["src/application/use_cases/get_scrutin_detail.rs — appels GetScrutinDetail::new des tests (lignes 381, 394) : citations en 3e argument"],
   "boundedGreen": ["frontend/src/components/AmendmentAuthor.tsx", "frontend/src/components/OutcomeBadge.tsx"],
   "models": ["src/application/use_cases/get_scrutin_detail.rs — tests existants du détail"]
  },
  {
   "id": "C9", "wave": 9,
   "behaviour": "Une carte d'amendement renvoie aux scrutins qui l'ont mis aux voix, dans l'ordre chronologique",
   "rmcu": "RM-08 / CU-03",
   "steps": ["5. Une carte d'amendement renvoie aux scrutins qui l'ont mis aux voix"],
   "level": "UT", "skill": "tests-unit-tests",
   "testClass": "browse_dossier_amendments", "fixture": "read_fakes",
   "methods": [
    "browse_dossier_amendments.should_list_scrutins_in_chronological_order_when_amendment_was_put_to_the_vote",
    "browse_dossier_amendments.should_list_no_scrutin_when_amendment_was_never_linked",
    "browse_dossier_amendments.should_open_page_holding_the_number_when_card_is_reached_from_a_scrutin"
   ],
   "full": ["src/application/use_cases/browse_dossier_amendments.rs"],
   "bounded": ["src/application/use_cases/citation_fakes.rs (nouveau, créé en C4)", "src/application/use_cases/read_fakes.rs"],
   "observation": "AmendmentView.votes ordonnés ; page filtrée sur le numéro",
   "outOfScope": "—",
   "toCreate": [],
   "production": ["src/application/use_cases/browse_dossier_amendments.rs", "src/application/ports/amendment_repository.rs", "src/infrastructure/persistence/pg_amendment_repository.rs", "src/api/amendment_dto.rs", "src/api/handlers/amendment_handlers.rs", "frontend/src/types/amendments.ts", "frontend/src/components/DossierAmendments.tsx"],
   "ripple": ["src/application/use_cases/citation_fakes.rs (nouveau, créé en C4) — scrutins_of_amendments servi depuis les données semées", "src/application/ports/amendment_citation_repository.rs (nouveau, créé en C3) — scrutins_of_amendments ajouté au trait", "src/infrastructure/persistence/pg_amendment_citation_repository.rs (nouveau, créé en C3) — scrutins_of_amendments : votes des amendements, ordre chronologique", "src/application/use_cases/read_fakes.rs — filtre par numéro du double d'amendements"],
   "boundedGreen": ["src/infrastructure/persistence/pg_theme_repository.rs"],
   "models": ["src/application/use_cases/browse_dossier_amendments.rs — tests de F1"]
  }
 ]
}
```

## Hypothèses <!-- kit:assumptions -->

_Vide à la rédaction du plan._

| # | Assumption | To be validated by |
|---|------------|--------------------|
