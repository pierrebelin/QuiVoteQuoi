//! Garde applicative des routes d'ecriture.
//!
//! Le front est du JavaScript public : rien de ce qu'il embarque n'est un
//! secret, et `Origin` comme `Referer` se falsifient avec un `curl -H`.
//! « N'autoriser que le front » n'est donc pas une regle applicable, et les
//! routes de consultation restent ouvertes — le site est un site de
//! transparence, il n'a rien a cacher (README.md §2).
//!
//! Ce qui est applicable : les routes d'ecriture sont des commandes
//! d'administration (ingestion Assemblee nationale, thematisation LLM),
//! aucune n'est utilisee par un parcours de consultation. Elles exigent
//! le jeton du jour, derive de `ADMIN_TOKEN_SECRET` (voir
//! `infrastructure::security`).
//!
//! Les deux appelants legitimes le derivent :
//! - l'ecran d'administration, ouvert via le tunnel SSH, ou l'operateur colle
//!   le jeton du jour obtenu par `deploy/bin/admin-token.sh` ;
//! - la tache CRON locale du VPS, qui lit le secret dans
//!   `/home/hemicycle/shared/.env` et derive le jeton a chaque execution.
//!
//! Le filtre Nginx du vhost public (`limit_except GET HEAD OPTIONS`) reste en
//! place : deux barrieres valent mieux qu'une, et celle-ci tient meme si un
//! autre service du VPS atteint le port du backend en direct.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use chrono::{NaiveDate, Utc};

use crate::infrastructure::security::AdminTokenSecret;

/// Etat du middleware. Volontairement distinct d'`AppState` : la garde n'a
/// besoin que du secret, et cette etroitesse la rend testable sans base.
#[derive(Clone, Debug)]
pub struct AdminGuard(Option<Arc<AdminTokenSecret>>);

impl AdminGuard {
    pub fn new(secret: Option<AdminTokenSecret>) -> Self {
        Self(secret.map(Arc::new))
    }

    /// Sans secret, l'ecriture est fermee — pas ouverte. Un deploiement qui
    /// oublie la variable perd l'administration, il n'expose pas l'ingestion.
    pub fn closed() -> Self {
        Self(None)
    }

    pub fn is_configured(&self) -> bool {
        self.0.is_some()
    }
}

/// Motif de rejet. Distinguer les deux cas rend le diagnostic possible sans
/// rien dire de la valeur attendue.
#[derive(Debug, PartialEq, Eq)]
pub enum AuthRejection {
    /// Aucun `ADMIN_TOKEN_SECRET` cote serveur.
    NotConfigured,
    /// Jeton absent, mal forme, perime, ou faux.
    InvalidToken,
}

impl AuthRejection {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotConfigured => StatusCode::FORBIDDEN,
            Self::InvalidToken => StatusCode::UNAUTHORIZED,
        }
    }

    fn message(&self) -> &'static str {
        match self {
            Self::NotConfigured => "écriture fermée : ADMIN_TOKEN_SECRET absent",
            // Ne pas distinguer « perime » de « faux » : ce serait dire a un
            // attaquant qu'il tient un jeton authentique, seulement trop vieux.
            Self::InvalidToken => "jeton invalide ou expiré",
        }
    }
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (self.status(), self.message()).into_response()
    }
}

/// Lit le jeton presente : `x-admin-token` en priorite, `Authorization:
/// Bearer` en repli. Le second sert aux clients en ligne de commande (CRON,
/// `curl`) qui ont deja un mecanisme pour cet en-tete.
fn presented_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-admin-token")
        .and_then(|value| value.to_str().ok())
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.strip_prefix("Bearer "))
        })
}

/// Comparaison a duree constante. Un `==` sur `str` sort au premier octet
/// different et laisse mesurer le prefixe correct, octet par octet. La
/// longueur fuit ; elle ne suffit pas a reconstituer un jeton de 128 bits.
fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Coeur de la garde, sans Axum ni horloge autour : c'est ce que testent les
/// tests. `today` est injecte pour que le passage de minuit soit testable.
pub fn verify(
    guard: &AdminGuard,
    headers: &HeaderMap,
    today: NaiveDate,
) -> Result<(), AuthRejection> {
    let secret = guard.0.as_deref().ok_or(AuthRejection::NotConfigured)?;
    let presented = presented_token(headers).ok_or(AuthRejection::InvalidToken)?;

    // Parcourir toute la liste sans court-circuit : sortir des le jeton du
    // jour trouve rendrait mesurable la difference entre « jeton du jour » et
    // « jeton de la veille ».
    let matched = secret
        .accepted_at(today)
        .iter()
        .fold(false, |found, candidate| {
            constant_time_eq(presented.as_bytes(), candidate.as_bytes()) | found
        });

    if matched {
        Ok(())
    } else {
        Err(AuthRejection::InvalidToken)
    }
}

/// Middleware Axum, pose en `route_layer` sur le sous-routeur d'ecriture.
pub async fn require_admin_token(
    State(guard): State<AdminGuard>,
    request: Request,
    next: Next,
) -> Response {
    match verify(&guard, request.headers(), Utc::now().date_naive()) {
        Ok(()) => next.run(request).await,
        Err(rejection) => {
            tracing::warn!(
                path = %request.uri().path(),
                reason = ?rejection,
                "écriture refusée"
            );
            rejection.into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn guard() -> AdminGuard {
        AdminGuard::new(Some(AdminTokenSecret::new(SECRET.to_string()).unwrap()))
    }

    fn token_of(day: NaiveDate) -> String {
        AdminTokenSecret::new(SECRET.to_string())
            .unwrap()
            .token_for(day)
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(
                axum::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
        headers
    }

    #[test]
    fn accepts_todays_token_in_the_dedicated_header() {
        let today = day(2026, 8, 7);
        let result = verify(
            &guard(),
            &headers(&[("x-admin-token", &token_of(today))]),
            today,
        );
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn accepts_todays_token_as_bearer() {
        let today = day(2026, 8, 7);
        let bearer = format!("Bearer {}", token_of(today));
        let result = verify(&guard(), &headers(&[("authorization", &bearer)]), today);
        assert_eq!(result, Ok(()));
    }

    /// Une tache CRON lancee a 23 h 59 doit encore passer a 00 h 01.
    #[test]
    fn accepts_yesterdays_token() {
        let today = day(2026, 8, 7);
        let result = verify(
            &guard(),
            &headers(&[("x-admin-token", &token_of(day(2026, 8, 6)))]),
            today,
        );
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn rejects_the_token_of_two_days_ago() {
        let result = verify(
            &guard(),
            &headers(&[("x-admin-token", &token_of(day(2026, 8, 5)))]),
            day(2026, 8, 7),
        );
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn rejects_tomorrows_token() {
        let result = verify(
            &guard(),
            &headers(&[("x-admin-token", &token_of(day(2026, 8, 8)))]),
            day(2026, 8, 7),
        );
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn rejects_a_request_without_headers() {
        let result = verify(&guard(), &headers(&[]), day(2026, 8, 7));
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn rejects_a_wrong_token() {
        let result = verify(
            &guard(),
            &headers(&[("x-admin-token", "00000000000000000000000000000000")]),
            day(2026, 8, 7),
        );
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn rejects_a_prefix_of_todays_token() {
        let today = day(2026, 8, 7);
        let truncated = &token_of(today)[..16];
        let result = verify(&guard(), &headers(&[("x-admin-token", truncated)]), today);
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn rejects_the_master_secret_presented_as_is() {
        let result = verify(
            &guard(),
            &headers(&[("x-admin-token", SECRET)]),
            day(2026, 8, 7),
        );
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn rejects_a_bearer_without_the_prefix() {
        let today = day(2026, 8, 7);
        let result = verify(
            &guard(),
            &headers(&[("authorization", &token_of(today))]),
            today,
        );
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn closes_writes_when_the_secret_is_not_configured() {
        let today = day(2026, 8, 7);
        let result = verify(
            &AdminGuard::closed(),
            &headers(&[("x-admin-token", &token_of(today))]),
            today,
        );
        assert_eq!(result, Err(AuthRejection::NotConfigured));
    }

    #[test]
    fn dedicated_header_takes_precedence_over_bearer() {
        let today = day(2026, 8, 7);
        let bearer = format!("Bearer {}", token_of(today));
        let result = verify(
            &guard(),
            &headers(&[("x-admin-token", "faux"), ("authorization", &bearer)]),
            today,
        );
        assert_eq!(result, Err(AuthRejection::InvalidToken));
    }

    #[test]
    fn secret_does_not_leak_in_the_guard_debug_output() {
        assert!(!format!("{:?}", guard()).contains(SECRET));
    }
}
