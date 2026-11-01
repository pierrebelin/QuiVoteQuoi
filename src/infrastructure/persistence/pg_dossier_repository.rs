use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{PgPool, Postgres, QueryBuilder};

use super::db;
use crate::application::ports::dossier_repository::{
    DossierCriteria, DossierPage, DossierRepository, RepositoryError, StoredDossierState,
};
use crate::domain::actor::{ActorRole, ActorUid, GroupUid, MembershipQuality};
use crate::domain::dossier::{
    Committee, CurationStatus, DossierOutcome, DossierUid, Initiator, InitiatorGroup,
    LawPublication, LegislativeAct, LegislativeDocument, LegislativeDossier, LegislativeStage,
    Score,
};

pub struct PgDossierRepository {
    pool: PgPool,
}

impl PgDossierRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn fetch_acts(&self, dossier_uid: &str) -> Result<Vec<LegislativeAct>, RepositoryError> {
        let rows = sqlx::query_as::<_, ActRow>(
            "SELECT act_date, label, act_code FROM legislative_acts WHERE dossier_uid = $1 ORDER BY act_date",
        )
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .into_iter()
            .map(|r| LegislativeAct {
                date: r.act_date,
                label: r.label,
                code: r.act_code,
            })
            .collect())
    }

    async fn fetch_documents(
        &self,
        dossier_uid: &str,
    ) -> Result<Vec<LegislativeDocument>, RepositoryError> {
        let rows = sqlx::query_as::<_, DocumentRow>(
            "SELECT document_uid, title, short_title, doc_type, doc_date,
                    official_url, source_archive_url, source_license,
                    source_metadata_fingerprint, source_retrieved_at
             FROM dossier_documents WHERE dossier_uid = $1 ORDER BY id",
        )
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .into_iter()
            .map(|r| LegislativeDocument {
                document_uid: r.document_uid,
                title: r.title,
                short_title: r.short_title,
                doc_type: r.doc_type,
                date: r.doc_date,
                official_url: r.official_url,
                source_archive_url: r.source_archive_url,
                source_license: r.source_license,
                source_metadata_fingerprint: r.source_metadata_fingerprint,
                source_retrieved_at: r.source_retrieved_at,
            })
            .collect())
    }

    async fn save_batch(&self, dossiers: &[LegislativeDossier]) -> Result<(), RepositoryError> {
        let mut tx = self.pool.begin().await.map_err(db)?;

        for dossier in dossiers {
            let stage_code = dossier.current_stage.map(|s| s.to_code().to_string());

            let outcome = &dossier.outcome;
            let (law_code, law_jo_date, law_url) = match outcome {
                DossierOutcome::Promulgated { publication, .. } => (
                    publication.law_code.clone(),
                    publication.jo_date,
                    publication.legifrance_url.clone(),
                ),
                _ => (None, None, None),
            };
            let (merged_into_uid, merge_cause) = match outcome {
                DossierOutcome::MergedInto { dossier_uid, cause } => {
                    (Some(dossier_uid.as_str().to_string()), cause.clone())
                }
                _ => (None, None),
            };
            let outcome_label = match outcome {
                DossierOutcome::Rejected { label, .. } => Some(label.clone()),
                _ => None,
            };

            sqlx::query(
                "INSERT INTO legislative_dossiers (uid, title, procedure_label, last_activity_date, last_activity_label, score_progress, score_magnitude, score_momentum, score_total, current_stage_code, committee, curation_status, legislature, url, summary, deposit_date, outcome_kind, outcome_date, outcome_label, law_code, law_jo_date, law_legifrance_url, merged_into_uid, merge_cause)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
                 ON CONFLICT (uid) DO UPDATE SET
                    outcome_kind = EXCLUDED.outcome_kind,
                    outcome_date = EXCLUDED.outcome_date,
                    outcome_label = EXCLUDED.outcome_label,
                    law_code = EXCLUDED.law_code,
                    law_jo_date = EXCLUDED.law_jo_date,
                    law_legifrance_url = EXCLUDED.law_legifrance_url,
                    merged_into_uid = EXCLUDED.merged_into_uid,
                    merge_cause = EXCLUDED.merge_cause,
                    title = EXCLUDED.title,
                    deposit_date = EXCLUDED.deposit_date,
                    procedure_label = EXCLUDED.procedure_label,
                    last_activity_date = EXCLUDED.last_activity_date,
                    last_activity_label = EXCLUDED.last_activity_label,
                    score_progress = EXCLUDED.score_progress,
                    score_magnitude = EXCLUDED.score_magnitude,
                    score_momentum = EXCLUDED.score_momentum,
                    score_total = EXCLUDED.score_total,
                    current_stage_code = EXCLUDED.current_stage_code,
                    committee = EXCLUDED.committee,
                    legislature = EXCLUDED.legislature,
                    url = EXCLUDED.url,
                    summary = COALESCE(EXCLUDED.summary, legislative_dossiers.summary),
                    updated_at = NOW()",
            )
            .bind(dossier.uid.as_str())
            .bind(&dossier.title)
            .bind(&dossier.procedure)
            .bind(dossier.last_activity_date)
            .bind(&dossier.last_activity_label)
            .bind(dossier.score.progress() as i16)
            .bind(dossier.score.magnitude() as i16)
            .bind(dossier.score.momentum() as i16)
            .bind(dossier.score.total() as i16)
            .bind(&stage_code)
            .bind(dossier.committee.as_ref().map(|c| c.as_str()))
            .bind(dossier.curation_status.as_str())
            .bind(dossier.legislature as i16)
            .bind(&dossier.url)
            .bind(&dossier.summary)
            .bind(dossier.deposit_date)
            .bind(outcome.kind())
            .bind(outcome.date())
            .bind(&outcome_label)
            .bind(&law_code)
            .bind(law_jo_date)
            .bind(&law_url)
            .bind(&merged_into_uid)
            .bind(&merge_cause)
            .execute(&mut *tx)
            .await
            .map_err(db)?;

            sqlx::query("DELETE FROM legislative_acts WHERE dossier_uid = $1")
                .bind(dossier.uid.as_str())
                .execute(&mut *tx)
                .await
                .map_err(db)?;

            for act in &dossier.acts {
                sqlx::query(
                    "INSERT INTO legislative_acts (dossier_uid, act_date, label, act_code) VALUES ($1, $2, $3, $4)",
                )
                .bind(dossier.uid.as_str())
                .bind(act.date)
                .bind(&act.label)
                .bind(&act.code)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }

            sqlx::query("DELETE FROM dossier_initiators WHERE dossier_uid = $1")
                .bind(dossier.uid.as_str())
                .execute(&mut *tx)
                .await
                .map_err(db)?;

            for initiator in &dossier.initiators {
                let group = initiator.group();
                sqlx::query(
                    "INSERT INTO dossier_initiators (dossier_uid, full_name, actor_uid, actor_role, group_uid, group_abbrev, group_label, membership_quality, reference_date, official_url)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(dossier.uid.as_str())
                .bind(initiator.full_name())
                .bind(initiator.actor_uid().map(|u| u.as_str()))
                .bind(initiator.role().map(|r| r.as_str()))
                .bind(group.map(|g| g.uid.as_str()))
                .bind(group.map(|g| g.abbrev.as_str()))
                .bind(group.map(|g| g.label.as_str()))
                .bind(group.and_then(|g| g.quality.as_ref()).map(|q| q.as_str()))
                .bind(initiator.reference_date())
                .bind(initiator.official_url())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }

            sqlx::query("DELETE FROM dossier_documents WHERE dossier_uid = $1")
                .bind(dossier.uid.as_str())
                .execute(&mut *tx)
                .await
                .map_err(db)?;

            for doc in &dossier.documents {
                sqlx::query(
                    "INSERT INTO dossier_documents (
                        dossier_uid, document_uid, title, short_title, doc_type, doc_date,
                        official_url, source_archive_url, source_license,
                        source_metadata_fingerprint, source_retrieved_at
                    ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())",
                )
                .bind(dossier.uid.as_str())
                .bind(&doc.document_uid)
                .bind(&doc.title)
                .bind(&doc.short_title)
                .bind(&doc.doc_type)
                .bind(doc.date)
                .bind(&doc.official_url)
                .bind(&doc.source_archive_url)
                .bind(&doc.source_license)
                .bind(&doc.source_metadata_fingerprint)
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            }
        }

        tx.commit().await.map_err(db)?;
        Ok(())
    }

    async fn fetch_initiators(&self, dossier_uid: &str) -> Result<Vec<Initiator>, RepositoryError> {
        let rows = sqlx::query_as::<_, InitiatorRow>(
            "SELECT full_name, actor_uid, actor_role, group_uid, group_abbrev, group_label, membership_quality, reference_date, official_url
             FROM dossier_initiators WHERE dossier_uid = $1 ORDER BY id",
        )
        .bind(dossier_uid)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows.into_iter().map(InitiatorRow::into_initiator).collect())
    }
}

const BATCH_SIZE: usize = 50;

/// Colonnes de la ligne de liste : ni actes, ni initiateurs, ni documents, que
/// la page ne montre pas et qui coûteraient une requête par dossier.
const PAGE_SELECT: &str =
    "SELECT uid, title, procedure_label, last_activity_date, last_activity_label,
            score_progress, score_magnitude, score_momentum, score_total,
            current_stage_code, committee, curation_status,
            legislature, url, summary, deposit_date,
            outcome_kind, outcome_date, outcome_label,
            law_code, law_jo_date, law_legifrance_url,
            merged_into_uid, merge_cause
     FROM legislative_dossiers";

/// Traduit les critères du visiteur en conditions SQL, la même clause servant
/// au décompte et à la tranche : sinon le total ne dirait pas ce qui est listé.
fn push_criteria(query: &mut QueryBuilder<'_, Postgres>, criteria: &DossierCriteria) {
    let mut separated = false;
    let open = |q: &mut QueryBuilder<'_, Postgres>, separated: &mut bool| {
        q.push(if *separated { " AND " } else { " WHERE " });
        *separated = true;
    };

    if let Some(search) = criteria.search.as_ref() {
        open(query, &mut separated);
        query.push("title ILIKE ");
        query.push_bind(format!("%{}%", escape_like(search)));
    }
    if let Some(kind) = criteria.outcome_kind.as_ref() {
        open(query, &mut separated);
        query.push("outcome_kind = ");
        query.push_bind(kind.clone());
    }
    // L'initiative se lit dans le libellé de procédure, avec la règle que porte
    // le domaine : « Projet de loi ... » vient du gouvernement.
    if let Some(initiative) = criteria.initiative {
        open(query, &mut separated);
        query.push("procedure_label LIKE ");
        query.push_bind(format!("{}%", escape_like(initiative.procedure_prefix())));
    }
}

/// Neutralise les jokers SQL d'une saisie libre : sans cela un titre contenant
/// `%` élargirait la recherche au lieu de la restreindre.
fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[async_trait]
impl DossierRepository for PgDossierRepository {
    async fn save_all(&self, dossiers: &[LegislativeDossier]) -> Result<usize, RepositoryError> {
        let mut saved = 0;
        for chunk in dossiers.chunks(BATCH_SIZE) {
            self.save_batch(chunk).await?;
            saved += chunk.len();
            tracing::debug!("Saved {saved}/{} dossiers", dossiers.len());
        }
        Ok(saved)
    }

    /// Une seule requete pour toute la base: 3 035 lignes courtes valent mieux
    /// que 3 035 dossiers reecrits.
    async fn load_states(&self) -> Result<HashMap<String, StoredDossierState>, RepositoryError> {
        let rows = sqlx::query_as::<_, StateRow>(
            "SELECT d.uid, d.last_activity_date, d.outcome_kind, COUNT(a.id) AS act_count
             FROM legislative_dossiers d
             LEFT JOIN legislative_acts a ON a.dossier_uid = d.uid
             GROUP BY d.uid, d.last_activity_date, d.outcome_kind",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .into_iter()
            .map(|r| {
                // Le sort est reconstruit avec des champs vides: seule la
                // variante compte ici, pas son contenu.
                let outcome_is_final = matches!(
                    r.outcome_kind.as_str(),
                    "promulgated" | "withdrawn" | "merged_into"
                );
                (
                    r.uid,
                    StoredDossierState {
                        last_activity_date: r.last_activity_date,
                        act_count: r.act_count.max(0) as usize,
                        outcome_is_final,
                    },
                )
            })
            .collect())
    }

    async fn find_recent(
        &self,
        since: NaiveDate,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        let rows = sqlx::query_as::<_, DossierRow>(
            "SELECT uid, title, procedure_label, last_activity_date, last_activity_label,
                    score_progress, score_magnitude, score_momentum, score_total,
                    current_stage_code, committee, curation_status,
                    legislature, url, summary, deposit_date,
                    outcome_kind, outcome_date, outcome_label,
                    law_code, law_jo_date, law_legifrance_url,
                    merged_into_uid, merge_cause
             FROM legislative_dossiers
             WHERE last_activity_date >= $1
             ORDER BY last_activity_date DESC",
        )
        .bind(since)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .into_iter()
            .map(|r| r.into_dossier(vec![], vec![], vec![]))
            .collect())
    }

    async fn find_page(
        &self,
        criteria: &DossierCriteria,
        limit: i64,
        offset: i64,
    ) -> Result<DossierPage, RepositoryError> {
        let mut count_query: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(*) FROM legislative_dossiers");
        push_criteria(&mut count_query, criteria);
        let total: i64 = count_query
            .build_query_scalar()
            .persistent(false)
            .fetch_one(&self.pool)
            .await
            .map_err(db)?;

        let mut query: QueryBuilder<Postgres> = QueryBuilder::new(PAGE_SELECT);
        push_criteria(&mut query, criteria);
        // `uid` départage les dossiers d'une même journée : sans lui, deux pages
        // successives peuvent renvoyer deux fois la même ligne.
        query.push(" ORDER BY last_activity_date DESC, uid DESC LIMIT ");
        query.push_bind(limit);
        query.push(" OFFSET ");
        query.push_bind(offset);

        let rows = query
            .build_query_as::<DossierRow>()
            .persistent(false)
            .fetch_all(&self.pool)
            .await
            .map_err(db)?;

        Ok(DossierPage {
            items: rows
                .into_iter()
                .map(|r| r.into_dossier(vec![], vec![], vec![]))
                .collect(),
            total,
        })
    }

    async fn find_by_uid(
        &self,
        uid: &DossierUid,
    ) -> Result<Option<LegislativeDossier>, RepositoryError> {
        let row = sqlx::query_as::<_, DossierRow>(
            "SELECT uid, title, procedure_label, last_activity_date, last_activity_label,
                    score_progress, score_magnitude, score_momentum, score_total,
                    current_stage_code, committee, curation_status,
                    legislature, url, summary, deposit_date,
                    outcome_kind, outcome_date, outcome_label,
                    law_code, law_jo_date, law_legifrance_url,
                    merged_into_uid, merge_cause
             FROM legislative_dossiers
             WHERE uid = $1",
        )
        .bind(uid.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(db)?;

        match row {
            Some(r) => {
                let uid_str = r.uid.clone();
                let acts = self.fetch_acts(&uid_str).await?;
                let initiators = self.fetch_initiators(&uid_str).await?;
                let documents = self.fetch_documents(&uid_str).await?;
                Ok(Some(r.into_dossier(acts, initiators, documents)))
            }
            None => Ok(None),
        }
    }

    async fn find_suggestions(
        &self,
        count: usize,
    ) -> Result<Vec<LegislativeDossier>, RepositoryError> {
        let rows = sqlx::query_as::<_, DossierRow>(
            "SELECT uid, title, procedure_label, last_activity_date, last_activity_label,
                    score_progress, score_magnitude, score_momentum, score_total,
                    current_stage_code, committee, curation_status,
                    legislature, url, summary, deposit_date,
                    outcome_kind, outcome_date, outcome_label,
                    law_code, law_jo_date, law_legifrance_url,
                    merged_into_uid, merge_cause
             FROM legislative_dossiers
             WHERE curation_status = 'new'
             ORDER BY score_total DESC
             LIMIT $1",
        )
        .bind(count as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(db)?;

        Ok(rows
            .into_iter()
            .map(|r| r.into_dossier(vec![], vec![], vec![]))
            .collect())
    }

    async fn update_curation_status(
        &self,
        uid: &DossierUid,
        status: CurationStatus,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query(
            "UPDATE legislative_dossiers SET curation_status = $1, updated_at = NOW() WHERE uid = $2",
        )
        .bind(status.as_str())
        .bind(uid.as_str())
        .execute(&self.pool)
        .await
        .map_err(db)?;

        Ok(result.rows_affected() > 0)
    }
}

#[derive(sqlx::FromRow)]
struct StateRow {
    uid: String,
    last_activity_date: NaiveDate,
    outcome_kind: String,
    act_count: i64,
}

#[derive(sqlx::FromRow)]
struct DossierRow {
    uid: String,
    title: String,
    procedure_label: String,
    last_activity_date: NaiveDate,
    last_activity_label: String,
    score_progress: i16,
    score_magnitude: i16,
    score_momentum: i16,
    score_total: i16,
    current_stage_code: Option<String>,
    committee: Option<String>,
    curation_status: String,
    legislature: i16,
    url: Option<String>,
    summary: Option<String>,
    deposit_date: Option<NaiveDate>,
    outcome_kind: String,
    outcome_date: Option<NaiveDate>,
    outcome_label: Option<String>,
    law_code: Option<String>,
    law_jo_date: Option<NaiveDate>,
    law_legifrance_url: Option<String>,
    merged_into_uid: Option<String>,
    merge_cause: Option<String>,
}

impl DossierRow {
    /// Un sort date sans date, ou une fusion sans dossier absorbant, est une
    /// ligne incoherente: on retombe sur l'absence de conclusion plutot que
    /// d'inventer la partie manquante.
    fn outcome(&self) -> DossierOutcome {
        match self.outcome_kind.as_str() {
            "promulgated" => self
                .outcome_date
                .map(|date| DossierOutcome::Promulgated {
                    date,
                    publication: LawPublication {
                        law_code: self.law_code.clone(),
                        jo_date: self.law_jo_date,
                        legifrance_url: self.law_legifrance_url.clone(),
                    },
                })
                .unwrap_or(DossierOutcome::NoRecordedConclusion),
            "withdrawn" => self
                .outcome_date
                .map(|date| DossierOutcome::Withdrawn { date })
                .unwrap_or(DossierOutcome::NoRecordedConclusion),
            "merged_into" => self
                .merged_into_uid
                .clone()
                .and_then(|uid| DossierUid::new(uid).ok())
                .map(|dossier_uid| DossierOutcome::MergedInto {
                    dossier_uid,
                    cause: self.merge_cause.clone(),
                })
                .unwrap_or(DossierOutcome::NoRecordedConclusion),
            "rejected" => match (self.outcome_date, self.outcome_label.clone()) {
                (Some(date), Some(label)) => DossierOutcome::Rejected { date, label },
                _ => DossierOutcome::NoRecordedConclusion,
            },
            _ => DossierOutcome::NoRecordedConclusion,
        }
    }

    fn into_dossier(
        self,
        acts: Vec<LegislativeAct>,
        initiators: Vec<Initiator>,
        documents: Vec<LegislativeDocument>,
    ) -> LegislativeDossier {
        let outcome = self.outcome();
        LegislativeDossier {
            uid: DossierUid::new(self.uid).expect("DB uid is non-empty"),
            title: self.title,
            procedure: self.procedure_label,
            legislature: self.legislature as u16,
            url: self.url,
            summary: self.summary,
            deposit_date: self.deposit_date,
            last_activity_date: self.last_activity_date,
            last_activity_label: self.last_activity_label,
            acts,
            documents,
            score: Score::new(
                self.score_progress as u8,
                self.score_magnitude as u8,
                self.score_momentum as u8,
                self.score_total as u8,
            )
            .expect("DB scores are in valid range"),
            current_stage: self
                .current_stage_code
                .as_deref()
                .and_then(LegislativeStage::from_code),
            initiators,
            committee: self
                .committee
                .map(|c| Committee::new(c).expect("DB committee is non-empty")),
            curation_status: CurationStatus::parse(&self.curation_status)
                .unwrap_or(CurationStatus::New),
            outcome,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ActRow {
    act_date: NaiveDate,
    label: String,
    act_code: Option<String>,
}

#[derive(sqlx::FromRow)]
struct InitiatorRow {
    full_name: String,
    actor_uid: Option<String>,
    actor_role: Option<String>,
    group_uid: Option<String>,
    group_abbrev: Option<String>,
    group_label: Option<String>,
    membership_quality: Option<String>,
    reference_date: Option<NaiveDate>,
    official_url: Option<String>,
}

impl InitiatorRow {
    fn into_initiator(self) -> Initiator {
        let name = self.full_name.clone();
        let unresolved =
            || Initiator::unresolved(name.clone()).expect("DB initiator name is non-empty");

        let Some(actor_uid) = self.actor_uid.and_then(|u| ActorUid::new(u).ok()) else {
            return unresolved();
        };
        let role = self
            .actor_role
            .as_deref()
            .and_then(ActorRole::parse)
            .unwrap_or(ActorRole::Other);

        // RM-01: la contrainte SQL garantit deja qu'un groupe stocke porte sa
        // date de reference; la construction du domaine la revalide.
        let group = match (self.group_uid, self.group_abbrev, self.group_label) {
            (Some(uid), Some(abbrev), Some(label)) => {
                GroupUid::new(uid).ok().map(|uid| InitiatorGroup {
                    uid,
                    abbrev,
                    label,
                    quality: self
                        .membership_quality
                        .and_then(|q| MembershipQuality::new(q).ok()),
                })
            }
            _ => None,
        };

        Initiator::resolved(
            self.full_name,
            actor_uid,
            role,
            group,
            self.reference_date,
            self.official_url,
        )
        .unwrap_or_else(|_| unresolved())
    }
}

#[derive(sqlx::FromRow)]
struct DocumentRow {
    document_uid: String,
    title: String,
    short_title: Option<String>,
    doc_type: String,
    doc_date: Option<NaiveDate>,
    official_url: Option<String>,
    source_archive_url: Option<String>,
    source_license: Option<String>,
    source_metadata_fingerprint: Option<String>,
    source_retrieved_at: Option<DateTime<Utc>>,
}
