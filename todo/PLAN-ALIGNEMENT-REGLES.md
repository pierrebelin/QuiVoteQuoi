# Plan — aligner le code sur les règles d'architecture

Audit du 2026-11-01 : `src/` (31 k lignes Rust) confronté à `.claude/rules/{domain,application,infrastructure,api}.md` et à `AGENTS.md`. Base de départ verte : `cargo test --lib` = 400 passés, 3 ignorés.

## Constat

- **Aucun écart critique.** Les dépendances pointent bien vers l'intérieur : zéro `sqlx`/`axum`/`infrastructure` dans le domaine ou l'application, zéro `anyhow`, zéro `mockall`, tous les ports sont `#[async_trait] pub trait X: Send + Sync`, tous les tests sont inline.
- **Les règles sont des brouillons** (`kit-init`, « review before trusting ») : une bonne part des écarts relevés sont des erreurs de la règle, pas du code. Elles se corrigent en premier.
- **La vraie dette** : de la logique métier logée hors du domaine (use cases, adapters, DTO), des handlers épais, des agrégats à champs publics, des fakes de test copiés jusqu'à 7 fois.

Volume relevé : domaine 43, application 33, infrastructure ~70 sites, API/bin 68, plus 25 lignes de vocabulaire éditorial.

## Lots

Un lot = une branche `feature/…` = une PR. Chaque lot laisse `cargo test` vert et ne change pas le comportement observable, sauf mention.

### Lot 0 — Corriger les règles et `AGENTS.md` (docs seules) — ✅ fait le 2026-11-01

Fichiers ignorés par Git (`.gitignore` : `.claude/`, `**/AGENTS.md`) : la correction est locale, sans PR.

- `AGENTS.md` : `use_cases/{uc}/ → command, handler, tests` est faux (fichiers plats, la command vit dans le fichier du use case) ; « mockall pour les ports d'effet » est faux (fakes écrits à la main partout) ; `.Codex/skills/implement.md` n'existe pas ; l'arbre omet `lib.rs`, `bin/`, `tests/`, les sous-dossiers d'infrastructure ; « config jamais en dur » à restreindre aux secrets et à l'infrastructure.
- `domain.md` : exemple d'agrégat `LegislativeDossier` = contre-exemple (17 champs publics) → citer `Amendment`/`Scrutin` ; « champs privés » limité aux types porteurs d'invariant ; `parse -> Option<Self>` (6 sur 7) ; erreurs `Debug, PartialEq, Eq, Error` et variantes `Empty*`/`Unknown*`/`Missing*` ; `parse` exigé seulement si l'infra relit la valeur ; références de spec étrangère `SPEC-<nom> RM-xx`.
- `application.md` : autoriser `tracing`, `thiserror`, `async_trait`, `std::collections` ; « pas de génériques de type » (les durées de vie `<'a>` sont partout) ; entrées `XCommand` et `execute_with` ; erreurs par port (`ClassifierError`, `SummaryGeneratorError`) ; `#[test]` sync pour les fonctions pures ; ne plus citer `fetch_recent_dossiers.rs` en modèle (il lit l'horloge).
- `infrastructure.md` : `<Techno>Client` pour les ports `*Source` ; `new(pool)` ou `from_env() -> Option<Self>` ; helper `db(e)` (majoritaire) ; `connect_timeout + read_timeout` accepté ; section pour les modules non-adapters (`config`, `security`, `provider`, registre, `Raw*`) ; tests `#[ignore]` sur archive réelle.
- `api.md` : des tests existent (`tests/write_routes_guard.rs`) ; suffixe `Request` ; un seul style de mapping (`From`, y compris sur tuple) ; ne plus citer `dossier_handlers.rs:28-227` en modèle.

### Lot 1 — Hygiène mécanique (S, sans risque) — ✅ fait le 2026-11-01

`cargo test --locked` vert : lib 404 passés (+4 round-trip), 3 ignorés ; intégration 7 + 3 + 2. Restes reportés au lot 5 : `MAX_ATTEMPTS`, `TOKENS_PER_LABEL`, `TOKENS_FLOOR` encore dupliqués dans les adapters LLM ; `SUMMARY_MAX_OUTPUT_TOKENS` importé depuis `anthropic_dossier_summary` par les adapters frères.

- Tests : 23 noms français → anglais (`api/security.rs`, `infrastructure/security.rs`, `tests/*.rs`) ; noms faibles (`chamber_assignment`, `*_scores_N`) ; 22 `is_err()` → variante exacte ; tests round-trip manquants (`ActorRole`, `VotePosition`, `TallyOrigin`, `LegislativeStage`).
- Domaine : `PartialEq, Eq` sur les enums d'erreur restants, `Display` sur `CandidateId`, `super::` au lieu de `crate::domain::`, doc RM sur `compute_score`.
- Infrastructure : un seul `persistence::db(e)` au lieu de 8 copies ; constantes nommées (timeouts 180 s ×6, `max_tokens` ×3, `clamp(1, 200)`, pool) et `llm/http_retry.rs` partagé (6 `backoff` dupliqués) ; `UnavailableClassifier` → `UnavailableThemeClassifier`.
- Application : log français `propose_theme_families.rs:161`.

### Lot 2 — Robustesse en production (S/M, change le comportement en cas d'erreur) — ✅ fait le 2026-11-01

`cargo test --locked` vert : lib 416 passés (+12), 3 ignorés ; intégration 7 + 3 + 2.

- `unwrap`/`expect` retirés du code de prod :
  - `archive_fetcher.rs` : un client HTTP impossible à construire est journalisé au démarrage et rend `SourceError::Download` à chaque téléchargement, au lieu de paniquer ; les 5 `lock().unwrap()` passent par `cache()`, qui récupère un verrou empoisonné (`PoisonError::into_inner`).
  - `pg_dossier_repository.rs` : `into_dossier` / `into_initiator` rendent `Result` ; une ligne refusée par le domaine devient `RepositoryError` via `persistence::invalid_row`, au lieu de faire tomber la requête.
  - `browse_final_votes.rs` : `build_entry` / `build_stance` faillibles, variantes `Scrutin` et `Actor` (`#[from]`) ; l'API les traite en 500, pas en 400.
  - `client.rs:127` : `Initiator::unresolved(..).into_iter().collect()`.
- `client.rs` : une entrée de zip illisible (`by_index`, `read_to_string`) est ignorée avec `tracing::warn!` dans `parse_dossiers` et `find_dossier`, au lieu d'interrompre l'ingestion.
- API : `api/error.rs` — `ApiError { BadRequest, NotFound, Internal }: IntoResponse` ; `Internal` est journalisé (`tracing::error!`) et répond `internal server error`. Tous les handlers et `status_of` y passent ; plus aucun `(StatusCode, String)`.
- Santé : port `HealthProbe`, use case `CheckHealth`, adapter `PgHealthProbe`, DTO `HealthResponse` (réponse JSON inchangée) ; `AppState.db` remplacé par `health_probe`. `ALLOWED_ORIGINS` lu par `config::allowed_origins()`.
- **TLS — option retirée.** Chaîne de `data.assemblee-nationale.fr` saine le 2026-11-01 : feuille `CN=www.assemblee-nationale.fr` (SAN couvre `data.assemblee-nationale.fr`, valide jusqu'au 2027-01-31) → intermédiaire GandiCert servi → racine DigiCert Global Root G2 (présente dans webpki-roots, utilisé par `rustls-tls`) ; `openssl s_client` : `Verify return code: 0 (ok)`, `curl -I` sans `-k` : 200. `danger_accept_invalid_certs(true)` retiré : la vérification du certificat est active.

### Lot 3 — Logique métier vers le domaine (M/L, plus forte valeur) — ✅ fait le 2026-11-01

`cargo test --locked` vert : lib 486 passés (+70), 3 ignorés ; intégration 7 + 3 + 2. Comportement observable inchangé ; chaque règle déplacée a ses tests dans le domaine.

- **3a — déplacements simples.**
  - `FetchRecentDossiers::execute(days, today)` : l'horloge est lue dans le handler ; le test ne dépend plus de la date du jour.
  - `CURRENT_LEGISLATURE` → `domain/legislature.rs` (plus d'import d'un use case par l'infrastructure) ; `client.rs` remplace son `unwrap_or(17)`.
  - `Initiator::default_for(Option<Initiative>, has_initiator_refs)` et `GOVERNMENT_INITIATOR_NAME` dans `domain/dossier.rs`.
  - Un seul `FINAL_VOTE_PREDICATE` dans `persistence/mod.rs`, un test le lie à `FINAL_VOTE_MARKERS`.
  - `DatasetShape` et `CodeCount` → `domain/dataset.rs`, avec `scrutins_without_dossier`, `share_without_dossier_percent` et `scrutins_per_text`.
- **3b — scrutin.**
  - `Scrutin::attach_groups(&ActorDirectory) -> ReconstructionReport` ; `nominal_votes_mut` supprimé.
  - `scrutin_official_url(legislature, number)` : une seule construction de l'URL AN d'un scrutin, partagée par `Scrutin::official_url` et les deux DTO.
- **3c — lignées.**
  - `domain/group_lineage` porte `group_by_lineage` / `LineageGroup` (représentant : la canonique, sinon la dernière vue), `displayed_group`, `designates`, `same_group` et `uids_of`.
  - `VotingWindow::span` remplace `widen`.
  - `browse_groups` et `browse_final_votes` ne gardent que le repli de chaque groupe et leurs tris.
  - Écart toléré : `GroupSummary::designated_by` suit désormais la lignée déclarée et non plus les seuls `uids` présents dans les lignes. La différence n'est atteignable que si un identifiant de lignée manque de `parliamentary_groups`.
- **3d — dossiers.**
  - `StoredDossierState` passe dans le domaine, avec `WriteDecision` et `LegislativeDossier::write_decision`. Les compteurs restent dans le use case.
  - `AssemblySource::fetch_dossiers_since{,_with_refs}(Option<NaiveDate>)` : `None` signifie qu'il n'y a pas de borne. La date `2000-01-01` disparaît.
  - `DossierPeriod::from_dates` porte la règle de début et de fin de période. Les `min()` et le filtre de recouvrement des groupes restent en SQL.
- **3e — synthèses.**
  - `DossierGroupFacts` et ses sous-structures → `domain/dossier_group_facts.rs`, à l'identique. `fingerprint_is_stable` fige l'empreinte SHA-256 d'un jeu de faits.
  - `domain/dossier_summary.rs` reçoit `FactsFingerprint`, `SummaryStatus`, `SummarySource` et `StoredGroupSummary`, avec deux règles distinctes :
    - `is_shown_for`, en lecture : statut `Ready` et même empreinte ;
    - `is_up_to_date`, en écriture : en plus, même modèle et même version de prompt.
  - `GroupFacts::has_actions` et `GroupSummaryState` (`as_str` donne les chaînes du contrat JSON).
  - Le hachage (`sha2`, `serde_json`) reste dans l'application, dans une seule fonction : `domain.md` n'autorise que `chrono`, `serde` et `thiserror`.

### Lot 4 — Encapsulation du domaine (M/L)

- Champs publics → privés + constructeur validant : `MemberCountRange`, `VotingWindow`, `Signatory`, `AmendmentTarget`, `VoterShare`, `GroupStance`, `FinalVote`.
- Identifiants `String` bruts → newtypes existants (`ScrutinUid`, `TextKey`, `DossierUid`…) : 6 sites.
- `is_empty()` sans `trim()` dans 6 constructeurs (**accepte aujourd'hui des blancs** : léger changement de comportement).
- `DomainError` → `DossierError` ; `Scrutin::new` à 16 arguments → struct `NewScrutin` (comme `NewAmendment`).
- `LegislativeDossier` (17 champs publics, construit en littéral dans 3 use cases) : sous-lot à part, le plus coûteux.

### Lot 5 — Couche application (M/L)

- `browse_themes.rs` : 7 méthodes, pas d'`execute`, fusionne trois use cases → découper ; `list_scrutins::for_dossier` → use case dédié ; `refresh_all` 12 arguments → struct de configuration.
- `propose_theme_families` : erreur domaine écrasée en `RepositoryError::Database` → enum d'erreur avec `#[from] ThemeError`.
- Ports : deux traits dans `dossier_group_actions_repository.rs` ; `tokio::sync::mpsc::Receiver` et logique `ArchiveScan` dans `amendment_source.rs` ; `serde::Serialize` sur des types de port ; `AttemptOutcome::as_str` rend des codes DB.
- Tests : module de fakes partagé `#[cfg(test)]` (`InMemoryDossierRepository` ×7, `FakeSource` ×5) à la place de `theme_fakes.rs` et des `pub(crate) mod tests` importés ; tests absents sur `browse_dossier_amendments`, `browse_themes`, `list_scrutins`.
- `system_prompt` de thématisation importé d'`anthropic_classifier` par deux adapters frères → `llm/theme_prompt.rs`.

### Lot 6 — Couche API et binaires (M/L)

- Handlers minces : calculs de pagination, bornage, parsing, assemblage de réponse et appel direct d'un port (`theme_handlers.rs:86`) sortent du handler ; deux handlers enchaînent deux use cases avec une politique d'erreur propre → un use case.
- Mapping : `from_result`, `from_parts`, `::new`, `.map(|x| Dto { .. })` inline → `impl From` ; `TallyDto` ×3 et `FinalVoteDto` ×2 → un seul chacun.
- Champs de requête `candidats`, `groupes` → anglais avec `serde(rename)` (contrat HTTP inchangé).
- `main.rs:95-161` : six bras de sélection du fournisseur LLM → fabrique dans `infrastructure::llm` ; `AdminGuard::from_env()`.
- `bin/sync_official_text_versions.rs` : SQL brut dans le binaire → port + adapter Pg + use case.

### Lot 7 — Vocabulaire éditorial (curation, suggestions)

`CurateDossier`, `SuggestDossiers`, `CurationStatus`, routes `/api/suggestions` et `/curate`, 25 lignes en tout. **Décision** : supprimer ces fonctionnalités (héritées du pivot) ou les renommer. Touche le frontend et le contrat HTTP.

## Hors périmètre

- Frontend : aucune règle écrite pour `frontend/`.
- `row.get` (~277) et casts `as u8/u16` (~66) en persistence : panique sur un type inattendu, mais le schéma est sous contrôle des migrations ; coût élevé pour un gain faible.
- `src/bin/measure_phase5.rs`, `capture_official_text_versions.rs` : non audités (lecture refusée par les permissions).

## Décisions ouvertes

| # | Question | Recommandation |
|---|---|---|
| 1 | `danger_accept_invalid_certs` | ✅ Tranché : chaîne saine, option retirée (lot 2) |
| 2 | Curation / suggestions | Supprimer si le frontend actuel ne s'en sert plus |
| 3 | Ordre des lots | 0 → 1 → 2 → 3, puis 4–6 selon le temps |
| 4 | Vote déjà sous la sentinelle `PO0` explicite : `Scrutin::attach_groups` ne le rattache pas et le compte non résolu (comportement d'avant le lot 3, documenté) | Le rattacher comme un vote sans groupe, si la source publie de tels votes |
| 5 | `is_final_vote_subject` ignore casse et espaces de tête, `FINAL_VOTE_PREDICATE` (`LIKE`) non | Aligner la fonction du domaine sur le SQL, qui est ce que le site affiche (aucun appelant en production) |
| 6 | `pg_dossier_repository.rs` recalcule `outcome_is_final` par chaînes (`promulgated`, `withdrawn`, `merged_into`), doublon de `DossierOutcome::is_final` | Passer par `DossierOutcome::parse(..).is_final()` (lot 4) |
| 7 | `infrastructure/llm/anthropic_dossier_summary.rs` construit encore l'URL AN d'un scrutin à la main | Utiliser `scrutin_official_url` |
| 8 | `FactsFingerprint::new` est infaillible (valeur opaque), `StoredGroupSummary.facts_fingerprint` reste `String` | Garder ; contrôle 64 hex seulement si une empreinte d'une autre forme apparaît |
