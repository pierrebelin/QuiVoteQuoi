use axum::extract::State;
use axum::Json;

use crate::api::dto::HealthResponse;
use crate::application::use_cases::check_health::CheckHealth;
use crate::AppState;

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let report = CheckHealth::new(state.health_probe.as_ref())
        .execute()
        .await;

    Json(HealthResponse::from(report))
}
