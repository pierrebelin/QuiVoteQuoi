//! Jeton d'administration a rotation quotidienne.
//!
//! Un jeton fixe pose dans un `.env` vit aussi longtemps que l'operateur
//! oublie de le changer : copie dans un historique de shell, dans le
//! `localStorage` d'un navigateur, dans un presse-papier. Ici le jeton
//! presente n'est jamais stocke cote serveur : il est *derive* du secret
//! maitre et de la date du jour.
//!
//! ```text
//! jeton(jour) = hex(HMAC-SHA256(ADMIN_TOKEN_SECRET, "AAAA-MM-JJ"))[..32]
//! ```
//!
//! Consequences :
//! - le jeton change chaque jour a minuit UTC, sans redemarrage ni migration ;
//! - un jeton qui fuite est mort en 48 h au plus ;
//! - revoquer immediatement = changer `ADMIN_TOKEN_SECRET` et redemarrer ;
//! - n'importe quel client sachant faire un HMAC derive le meme jeton — voir
//!   `deploy/bin/admin-token.sh`, utilise par l'operateur comme par le CRON.
//!
//! Le serveur accepte le jour courant **et** la veille : sans cette tolerance,
//! une tache CRON lancee a 23 h 59 s'authentifie avec un jeton perime a 00 h 00,
//! et l'operateur est deconnecte en plein arbitrage au passage de minuit.

use chrono::NaiveDate;
use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Longueur du jeton rendu, en caracteres hexadecimaux. 32 hex = 128 bits,
/// hors de portee d'une recherche exhaustive sur une journee.
const TOKEN_HEX_LEN: usize = 32;

/// Longueur minimale du secret maitre. `openssl rand -hex 32` en produit 64.
const MIN_SECRET_LEN: usize = 32;

#[derive(Debug, PartialEq, Eq)]
pub enum SecretError {
    Empty,
    TooShort { len: usize },
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "ADMIN_TOKEN_SECRET vide"),
            Self::TooShort { len } => write!(
                f,
                "ADMIN_TOKEN_SECRET trop court ({len} caractères, minimum {MIN_SECRET_LEN})"
            ),
        }
    }
}

/// Secret maitre. Newtype a constructeur validant : un secret court n'entre
/// jamais dans le systeme, il est refuse au demarrage.
#[derive(Clone)]
pub struct AdminTokenSecret(String);

impl std::fmt::Debug for AdminTokenSecret {
    /// Ne jamais rendre le secret imprimable : un `tracing::debug!` sur
    /// l'etat de l'application le deverserait dans le journal systemd.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AdminTokenSecret(<masqué>)")
    }
}

impl AdminTokenSecret {
    pub fn new(raw: String) -> Result<Self, SecretError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(SecretError::Empty);
        }
        if trimmed.chars().count() < MIN_SECRET_LEN {
            return Err(SecretError::TooShort {
                len: trimmed.chars().count(),
            });
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Jeton valable pour un jour donne.
    pub fn token_for(&self, day: NaiveDate) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.0.as_bytes())
            .expect("HMAC-SHA256 accepte une clé de n'importe quelle longueur");
        mac.update(day.format("%Y-%m-%d").to_string().as_bytes());
        let digest = mac.finalize().into_bytes();

        let mut hex = String::with_capacity(TOKEN_HEX_LEN);
        for byte in digest.iter().take(TOKEN_HEX_LEN / 2) {
            use std::fmt::Write;
            let _ = write!(hex, "{byte:02x}");
        }
        hex
    }

    /// Jetons acceptes a une date donnee : le jour meme et la veille.
    pub fn accepted_at(&self, today: NaiveDate) -> Vec<String> {
        let mut tokens = vec![self.token_for(today)];
        if let Some(yesterday) = today.pred_opt() {
            tokens.push(self.token_for(yesterday));
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "0123456789abcdef0123456789abcdef";

    fn secret() -> AdminTokenSecret {
        AdminTokenSecret::new(SECRET.to_string()).unwrap()
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn rejects_an_empty_secret() {
        assert_eq!(
            AdminTokenSecret::new("   ".to_string()).unwrap_err(),
            SecretError::Empty
        );
    }

    #[test]
    fn rejects_a_too_short_secret() {
        assert_eq!(
            AdminTokenSecret::new("court".to_string()).unwrap_err(),
            SecretError::TooShort { len: 5 }
        );
    }

    #[test]
    fn token_is_32_hexadecimal_characters() {
        let token = secret().token_for(day(2026, 8, 7));
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn token_is_stable_for_the_same_day() {
        let first = secret().token_for(day(2026, 8, 7));
        let second = secret().token_for(day(2026, 8, 7));
        assert_eq!(first, second);
    }

    #[test]
    fn token_changes_from_one_day_to_the_next() {
        let today = secret().token_for(day(2026, 8, 7));
        let tomorrow = secret().token_for(day(2026, 8, 8));
        assert_ne!(today, tomorrow);
    }

    #[test]
    fn different_secrets_yield_different_tokens() {
        let other = AdminTokenSecret::new("fedcba9876543210fedcba9876543210".to_string()).unwrap();
        assert_ne!(
            secret().token_for(day(2026, 8, 7)),
            other.token_for(day(2026, 8, 7))
        );
    }

    #[test]
    fn accepted_tokens_are_todays_and_yesterdays() {
        let secret = secret();
        let accepted = secret.accepted_at(day(2026, 8, 7));

        assert_eq!(
            accepted,
            vec![
                secret.token_for(day(2026, 8, 7)),
                secret.token_for(day(2026, 8, 6))
            ]
        );
    }

    #[test]
    fn token_of_two_days_ago_is_no_longer_accepted() {
        let secret = secret();
        let accepted = secret.accepted_at(day(2026, 8, 7));
        assert!(!accepted.contains(&secret.token_for(day(2026, 8, 5))));
    }

    /// Verrou d'interoperabilite avec `deploy/bin/admin-token.sh`, qui derive
    /// le meme jeton en shell :
    ///   printf '%s' 2026-08-07 | openssl dgst -sha256 -hmac "$SECRET" -r
    /// Si cette valeur change, le CRON du VPS tombe en 401 : ne la corriger
    /// qu'en connaissance de cause.
    #[test]
    fn derivation_is_frozen() {
        assert_eq!(
            secret().token_for(day(2026, 8, 7)),
            "823f2fef9241e4059bd3c4cf4be91472"
        );
    }

    #[test]
    fn secret_does_not_leak_in_debug_output() {
        let rendered = format!("{:?}", secret());
        assert!(!rendered.contains(SECRET));
    }
}
