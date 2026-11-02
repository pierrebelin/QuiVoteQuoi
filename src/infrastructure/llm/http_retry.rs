//! Réglages HTTP et nouvelle tentative communs aux adapters LLM.

use std::time::Duration;

use reqwest::StatusCode;

/// Borne de temps d'un appel complet à un fournisseur LLM.
pub(crate) const HTTP_TIMEOUT: Duration = Duration::from_secs(180);

/// Délai avant la première nouvelle tentative; il double à chaque échec.
const BACKOFF_BASE_MS: u64 = 500;

/// Nombre de caractères du corps d'une erreur HTTP repris dans le message.
const ERROR_DETAIL_MAX_CHARS: usize = 300;

/// Attend avant la tentative suivante (`attempt` compté à partir de 1).
pub(crate) async fn backoff(attempt: u32) {
    tokio::time::sleep(Duration::from_millis(
        BACKOFF_BASE_MS * 2u64.pow(attempt - 1),
    ))
    .await;
}

/// Un quota dépassé (429) ou une panne du fournisseur (5xx) se retente.
pub(crate) fn is_retryable(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Tronque le corps d'une erreur HTTP avant de l'insérer dans un message.
pub(crate) fn truncate_detail(body: &str) -> String {
    body.chars().take(ERROR_DETAIL_MAX_CHARS).collect()
}
