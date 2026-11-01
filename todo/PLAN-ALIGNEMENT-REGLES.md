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

### Lot 2 — Robustesse en production (S/M, change le comportement en cas d'erreur)

- `unwrap`/`expect` en code de prod : `archive_fetcher.rs` (construction du client, 5 `lock().unwrap()`), `pg_dossier_repository.rs:584-641` (panique sur une ligne DB), `browse_final_votes.rs:240-280`, `client.rs:127`.
- Une entrée de zip malformée interrompt toute l'ingestion des dossiers (`client.rs:241-252`) → ignorer + `tracing::warn!`, comme les trois autres clients.
- API : 16 `map_err(|e| (500, e.to_string()))` renvoient le texte d'erreur interne au client → `ApiError: IntoResponse` partagé, journalisé, message générique.
- `routes.rs:159` `sqlx::query("SELECT 1")` dans l'API → port de santé ; `ALLOWED_ORIGINS` via `config`.
- `archive_fetcher.rs:121` `danger_accept_invalid_certs(true)` → **décision** : vérifier la chaîne TLS de la source et retirer l'option si elle est saine.

### Lot 3 — Logique métier vers le domaine (M/L, plus forte valeur)

- Fusion des lignées de groupes : `browse_groups.rs:116-222` et sa copie `browse_final_votes.rs:138-219` → `domain/group_lineage`.
- Fraîcheur des synthèses (empreinte + modèle + version de prompt), dupliquée dans deux use cases ; `fingerprint` (`sha2`, `serde_json`) → `domain/dossier_summary`.
- `refresh_scrutins.rs:110-135` mute `vote.group_uid` en boucle → `Scrutin::attach_groups`, suppression de `nominal_votes_mut`.
- `refresh_dossiers.rs` : règle `should_write` et date magique `2000-01-01`.
- `pg_dossier_group_actions_repository.rs:62-76` : règle de début de période calculée en SQL → domaine.
- `FINAL_VOTE_PREDICATE` dupliqué dans deux repositories, miroir de `FINAL_VOTE_MARKERS`.
- Initiateur « Gouvernement » par défaut (`client.rs:121-129`), `CURRENT_LEGISLATURE` importé d'un use case par deux autres, calculs de part dans `describe_dataset.rs`, état `no_data/ready/summary_pending` et gabarit d'URL AN calculés dans les DTO.
- `fetch_recent_dossiers.rs` reçoit `today` au lieu d'appeler `Utc::now()`.

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
| 1 | `danger_accept_invalid_certs` | Vérifier la chaîne TLS, retirer si saine |
| 2 | Curation / suggestions | Supprimer si le frontend actuel ne s'en sert plus |
| 3 | Ordre des lots | 0 → 1 → 2 → 3, puis 4–6 selon le temps |
