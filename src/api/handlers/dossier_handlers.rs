use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use chrono::Utc;

use crate::AppState;
use crate::api::dossier_group_actions_dto::DossierGroupActionsResponse;
use crate::api::dto::{
    DossierDetailDto, DossierDto, DossierListItemDto, DossierPageQuery, DossierPageResponse,
    RecentActivityQuery, RecentDossiersResponse, RefreshQuery, RefreshResponse, RegistryResponse,
};
use crate::api::error::ApiError;
use crate::api::text_brief_dto::{DossierBriefQuery, DossierBriefResponse};
use crate::application::ports::dossier_repository::{DossierCriteria, DossierOrder};
use crate::application::use_cases::browse_dossiers::{BrowseDossiers, DossierQuery};
use crate::application::use_cases::browse_themes::BrowseThemes;
use crate::application::use_cases::fetch_recent_dossiers::FetchRecentDossiers;
use crate::application::use_cases::get_dossier_brief::{GetDossierBrief, GetDossierBriefError};
use crate::application::use_cases::get_dossier_detail::GetDossierDetail;
use crate::application::use_cases::get_dossier_group_actions::GetDossierGroupActions;
use crate::application::use_cases::journaling::Journaling;
use crate::application::use_cases::refresh_actor_registry::RefreshActorRegistry;
use crate::application::use_cases::refresh_all::RefreshAll;
use crate::application::use_cases::refresh_dossier_group_summaries::RefreshDossierGroupSummaries;
use crate::application::use_cases::refresh_dossiers::RefreshScope;
use crate::application::use_cases::refresh_text_briefs::RefreshTextBriefs;
use crate::application::use_cases::save_dossier::SaveDossier;
use crate::domain::dossier::DossierUid;
use crate::infrastructure::config;

pub async fn get_recent_dossiers(
    State(state): State<AppState>,
    Query(params): Query<RecentActivityQuery>,
) -> Result<Json<RecentDossiersResponse>, ApiError> {
    let uc = FetchRecentDossiers::new(state.dossier_repository.as_ref());

    let dossiers = uc
        .execute(params.days, Utc::now().date_naive())
        .await
        .map_err(ApiError::internal)?;

    let dtos: Vec<DossierDto> = dossiers.into_iter().map(DossierDto::from).collect();

    Ok(Json(RecentDossiersResponse {
        count: dtos.len(),
        dossiers: dtos,
    }))
}

/// Liste paginée des dossiers, par dernière activité ou dernier scrutin. Les critères de
/// la requête restreignent l'affichage, jamais le contenu de la base.
pub async fn browse_dossiers(
    State(state): State<AppState>,
    Query(params): Query<DossierPageQuery>,
) -> Result<Json<DossierPageResponse>, ApiError> {
    let query = DossierQuery::new(params.page, params.per_page, DossierCriteria::from(&params))
        .ordered_by(DossierOrder::from(&params));
    let uc = BrowseDossiers::new(
        state.dossier_repository.as_ref(),
        state.actor_repository.as_ref(),
    );

    let per_page = query.per_page();
    let page_number = query.page();

    let page = uc.execute(query).await.map_err(ApiError::internal)?;

    Ok(Json(DossierPageResponse {
        page: page_number,
        per_page,
        total: page.total,
        total_pages: page.total.div_euclid(per_page)
            + i64::from(page.total.rem_euclid(per_page) > 0),
        dossiers: page
            .items
            .into_iter()
            .map(DossierListItemDto::from)
            .collect(),
    }))
}

pub async fn get_dossier_detail(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<DossierDetailDto>, ApiError> {
    let uid = DossierUid::new(uid).map_err(ApiError::bad_request)?;

    let uc = GetDossierDetail::new(
        state.dossier_repository.as_ref(),
        state.assembly_source.as_ref(),
        state.actor_repository.as_ref(),
    );

    let result = uc
        .execute(&uid)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Dossier not found"))?;

    let families = BrowseThemes::new(state.theme_repository.as_ref())
        .families_of_dossier(uid.as_str())
        .await
        .map_err(ApiError::internal)?;

    let mut dto = DossierDetailDto::from_result(result);
    dto.families = families.into_iter().map(Into::into).collect();
    Ok(Json(dto))
}

pub async fn get_dossier_group_actions(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<Json<DossierGroupActionsResponse>, ApiError> {
    let result = GetDossierGroupActions::new(
        state.dossier_group_actions_repository.as_ref(),
        state.dossier_summary_repository.as_ref(),
        state.vote_explanation_repository.as_ref(),
    )
    .execute(&uid)
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("Dossier not found"))?;

    Ok(Json(DossierGroupActionsResponse::from(result)))
}

/// « En bref » d'un dossier : la dernière version disponible, ou celle que
/// `?version=` désigne (CU-03). Un dossier ou une version inconnus sont des
/// adresses qui n'existent pas.
pub async fn get_dossier_brief(
    State(state): State<AppState>,
    Path(uid): Path<String>,
    Query(query): Query<DossierBriefQuery>,
) -> Result<Json<DossierBriefResponse>, ApiError> {
    let uid = DossierUid::new(uid).map_err(ApiError::bad_request)?;

    let brief = GetDossierBrief::new(
        state.dossier_repository.as_ref(),
        state.text_version_repository.as_ref(),
        state.brief_line_repository.as_ref(),
        state.brief_generator.as_ref(),
    )
    .execute(&uid, query.version.as_deref())
    .await
    .map_err(status_of)?
    .ok_or_else(|| ApiError::not_found("Version not found"))?;

    Ok(Json(DossierBriefResponse::from(brief)))
}

/// Un dossier inconnu est une adresse qui n'existe pas, pas une panne.
fn status_of(error: GetDossierBriefError) -> ApiError {
    match error {
        GetDossierBriefError::NotFound(_) => ApiError::not_found("Dossier not found"),
        GetDossierBriefError::Repository(_) => ApiError::internal(error),
    }
}

pub async fn save_dossier(
    State(state): State<AppState>,
    Path(uid): Path<String>,
) -> Result<StatusCode, ApiError> {
    let uid = DossierUid::new(uid).map_err(ApiError::bad_request)?;

    let uc = SaveDossier::new(
        state.assembly_source.as_ref(),
        state.dossier_repository.as_ref(),
        state.actor_repository.as_ref(),
    );

    uc.execute(&uid).await.map_err(ApiError::internal)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Rafraichit le referentiel puis les dossiers, dans cet ordre.
///
/// Par defaut seuls les dossiers dont la source a bouge sont reecrits.
/// `?full=true` force la reecriture complete, necessaire apres un changement
/// de regle de derivation (score, sort, rattachement).
pub async fn refresh_dossiers(
    State(state): State<AppState>,
    Query(query): Query<RefreshQuery>,
) -> Result<Json<RefreshResponse>, ApiError> {
    let scope = if query.full {
        RefreshScope::Full
    } else {
        RefreshScope::Incremental
    };

    let uc = RefreshAll::new(
        state.actor_source.as_ref(),
        state.actor_repository.as_ref(),
        state.assembly_source.as_ref(),
        state.dossier_repository.as_ref(),
        state.scrutin_source.as_ref(),
        state.scrutin_repository.as_ref(),
        state.amendment_source.as_ref(),
        state.amendment_repository.as_ref(),
        state.amendment_citation_repository.as_ref(),
        state.sitting_record_source.as_ref(),
        state.vote_explanation_repository.as_ref(),
        state.theme_repository.as_ref(),
        state.theme_classifier.as_ref(),
        config::theme_batch_per_refresh(),
        config::amendment_batch_per_refresh(),
        Journaling::to(state.ingestion_journal.as_ref(), Utc::now()),
    );

    let outcome = uc
        .execute_with(scope, Utc::now().date_naive())
        .await
        .map_err(ApiError::internal)?;

    let summary_report = RefreshDossierGroupSummaries::new(
        state.dossier_group_actions_repository.as_ref(),
        state.dossier_summary_repository.as_ref(),
        state.dossier_summary_generator.as_ref(),
    )
    .execute(config::dossier_summary_batch_per_refresh())
    .await
    .map_err(ApiError::internal)?;

    // RM-18 : la passe « En bref » rapporte ses défaillances, elle ne fait
    // jamais échouer la route.
    let text_brief_report = RefreshTextBriefs::new(
        state.text_version_source.as_ref(),
        state.text_version_repository.as_ref(),
        state.legal_text_source.as_ref(),
        state.brief_line_repository.as_ref(),
        state.brief_generator.as_ref(),
    )
    .execute(
        config::text_brief_dossiers_per_refresh(),
        Utc::now().date_naive(),
    )
    .await;

    Ok(Json(RefreshResponse::from((
        outcome,
        summary_report,
        text_brief_report,
    ))))
}

pub async fn refresh_actor_registry(
    State(state): State<AppState>,
) -> Result<Json<RegistryResponse>, ApiError> {
    let uc = RefreshActorRegistry::new(
        state.actor_source.as_ref(),
        state.actor_repository.as_ref(),
        Journaling::to(state.ingestion_journal.as_ref(), Utc::now()),
    );

    let summary = uc.execute().await.map_err(ApiError::internal)?;

    Ok(Json(RegistryResponse::from(summary)))
}
