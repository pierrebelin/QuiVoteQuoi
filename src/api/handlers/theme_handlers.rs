use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Utc;

use crate::api::error::ApiError;
use crate::api::theme_dto::{
    ArbitrationRequest, ArbitrationResponse, AssignedFamilyDto, ExtractionResponse,
    FamiliesResponse, MethodResponse, ProposalRequest, ProposalRunResponse, TextDetailResponse,
    TextListQuery, TextListResponse,
};
use crate::application::ports::theme_repository::AssignedFamily;
use crate::application::use_cases::arbitrate_theme::{
    ArbitrateTheme, ArbitrationCommand, ArbitrationError,
};
use crate::application::use_cases::browse_themes::BrowseThemes;
use crate::application::use_cases::extract_debated_texts::ExtractDebatedTexts;
use crate::application::use_cases::propose_theme_families::ProposeThemeFamilies;
use crate::domain::theme::{FamilyCode, TextKey};
use crate::AppState;

/// CU-06 — Referentiel des familles, servi tel quel.
pub async fn list_families(State(state): State<AppState>) -> Json<FamiliesResponse> {
    let families = BrowseThemes::new(state.theme_repository.as_ref()).families();
    Json(FamiliesResponse::new(families))
}

/// CU-06 — Page methode.
pub async fn get_method(State(state): State<AppState>) -> Result<Json<MethodResponse>, ApiError> {
    let report = BrowseThemes::new(state.theme_repository.as_ref())
        .method()
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(MethodResponse::from(report)))
}

/// CU-05 — Textes sans famille. Atteignable depuis toute page de theme (RM-01).
pub async fn list_unassigned_texts(
    State(state): State<AppState>,
    Query(query): Query<TextListQuery>,
) -> Result<Json<TextListResponse>, ApiError> {
    let (limit, offset) = query.bounds();
    let page = BrowseThemes::new(state.theme_repository.as_ref())
        .unassigned_texts(limit, offset)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(TextListResponse::new(page, offset)))
}

/// CU-04 — Textes d'une famille.
pub async fn list_family_texts(
    State(state): State<AppState>,
    Path(code): Path<String>,
    Query(query): Query<TextListQuery>,
) -> Result<Json<TextListResponse>, ApiError> {
    let family = FamilyCode::parse(&code).map_err(ApiError::not_found)?;
    let (limit, offset) = query.bounds();
    let page = BrowseThemes::new(state.theme_repository.as_ref())
        .texts_of_family(family, limit, offset)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(TextListResponse::new(page, offset)))
}

/// CU-04 — Fiche d'un texte: familles courantes, historique, proposition,
/// scrutins portes.
pub async fn get_text_detail(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(query): Query<TextListQuery>,
) -> Result<Json<TextDetailResponse>, ApiError> {
    let key = TextKey::from_raw(&key);
    let browse = BrowseThemes::new(state.theme_repository.as_ref());
    let detail = browse
        .text_detail(&key)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("texte inconnu"))?;

    let (limit, offset) = query.bounds();
    let scrutins = state
        .theme_repository
        .scrutins_of_text(&key, limit, offset)
        .await
        .map_err(ApiError::internal)?;

    Ok(Json(TextDetailResponse::new(detail, scrutins)))
}

/// CU-01 — Extraire les textes debattus des objets de scrutin.
pub async fn extract_texts(
    State(state): State<AppState>,
) -> Result<Json<ExtractionResponse>, ApiError> {
    let report = ExtractDebatedTexts::new(state.theme_repository.as_ref())
        .execute()
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(ExtractionResponse::from(report)))
}

/// CU-02 — Soumettre au modele les textes en attente.
pub async fn propose_families(
    State(state): State<AppState>,
    body: Option<Json<ProposalRequest>>,
) -> Result<Json<ProposalRunResponse>, ApiError> {
    let batch = body
        .and_then(|Json(request)| request.batch)
        .unwrap_or(25)
        .clamp(1, 500);

    let run = ProposeThemeFamilies::new(
        state.theme_repository.as_ref(),
        state.theme_classifier.as_ref(),
    )
    .execute(batch, Utc::now().date_naive())
    .await
    .map_err(ApiError::internal)?;

    Ok(Json(ProposalRunResponse::from(run)))
}

/// CU-03 — Arbitrer une proposition.
pub async fn arbitrate(
    State(state): State<AppState>,
    Json(request): Json<ArbitrationRequest>,
) -> Result<Json<ArbitrationResponse>, ApiError> {
    let mut families = Vec::with_capacity(request.families.len());
    for code in &request.families {
        families.push(FamilyCode::parse(code).map_err(ApiError::bad_request)?);
    }

    let command = ArbitrationCommand {
        subject_kind: request.subject_kind.clone(),
        subject_id: request.subject_id.clone(),
        families,
        author: request.author,
        motive: request.motive,
    };

    let opened = ArbitrateTheme::new(state.theme_repository.as_ref())
        .execute(command, Utc::now().date_naive())
        .await
        .map_err(|e| match e {
            ArbitrationError::Repository(error) => ApiError::internal(error),
            other => ApiError::bad_request(other),
        })?;

    Ok(Json(ArbitrationResponse {
        subject_kind: request.subject_kind,
        subject_id: request.subject_id,
        families: opened
            .iter()
            .map(|assignment| {
                AssignedFamilyDto::from(AssignedFamily {
                    family: assignment.family(),
                    opened_on: assignment.opened_on(),
                    motive: assignment.motive().map(str::to_string),
                })
            })
            .collect(),
    }))
}
