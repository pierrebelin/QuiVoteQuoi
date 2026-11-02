use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;

use crate::api::amendment_dto::{AmendmentsRefreshResponse, DossierAmendmentsResponse};
use crate::api::error::ApiError;
use crate::application::ports::amendment_repository::AmendmentPageRequest;
use crate::application::use_cases::browse_dossier_amendments::BrowseDossierAmendments;
use crate::application::use_cases::refresh_amendments::RefreshAmendments;
use crate::application::use_cases::refresh_dossier_group_summaries::RefreshDossierGroupSummaries;
use crate::infrastructure::config;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct AmendmentPageQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub fate: Option<String>,
    pub group: Option<String>,
    /// Numero exact: ouvre la page qui porte la carte (depuis un scrutin).
    pub number: Option<String>,
}

/// CU-02 — Amendements d'un dossier.
///
/// Route fille plutot qu'un gonflement de `DossierDetailDto`: un dossier
/// budgetaire porte des milliers d'amendements, et la page dossier doit
/// s'afficher sans les attendre. Meme forme que `/api/dossiers/{uid}/scrutins`.
pub async fn get_dossier_amendments(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Query(query): Query<AmendmentPageQuery>,
) -> Result<Json<DossierAmendmentsResponse>, ApiError> {
    let page = AmendmentPageRequest::new(query.limit, query.offset);
    let page = AmendmentPageRequest {
        search: query.search.filter(|s| !s.trim().is_empty()),
        fate_code: query.fate.filter(|s| !s.trim().is_empty()),
        author_group_uid: query.group.filter(|s| !s.trim().is_empty()),
        number: query.number.filter(|s| !s.trim().is_empty()),
        ..page
    };

    let amendments = BrowseDossierAmendments::new(
        state.amendment_repository.as_ref(),
        state.actor_repository.as_ref(),
        state.amendment_citation_repository.as_ref(),
    )
    .execute(&uid, &page)
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(DossierAmendmentsResponse::new(
        amendments,
        page.offset,
        page.limit,
    )))
}

/// CU-01 — Ingestion des amendements.
pub async fn refresh_amendments(
    State(state): State<AppState>,
) -> Result<Json<AmendmentsRefreshResponse>, ApiError> {
    let summary = RefreshAmendments::new(
        state.amendment_source.as_ref(),
        state.amendment_repository.as_ref(),
        state.actor_repository.as_ref(),
        config::amendment_batch_per_refresh(),
    )
    .execute()
    .await
    .map_err(ApiError::internal)?;

    let mut response = AmendmentsRefreshResponse::from(summary);
    match RefreshDossierGroupSummaries::new(
        state.dossier_group_actions_repository.as_ref(),
        state.dossier_summary_repository.as_ref(),
        state.dossier_summary_generator.as_ref(),
    )
    .execute(config::dossier_summary_batch_per_refresh())
    .await
    {
        Ok(report) => {
            response.dossier_summaries = Some(crate::api::dto::DossierSummaryRefreshResponse {
                dossiers_seen: report.dossiers_seen,
                dossiers_refreshed: report.dossiers_refreshed,
                summaries_ready: report.summaries_ready,
                summaries_pending: report.summaries_pending,
                anomaly: report.anomaly,
            });
        }
        Err(error) => {
            tracing::warn!(%error, "Dossier summaries refresh failed after amendments");
        }
    }

    Ok(Json(response))
}
