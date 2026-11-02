//! Candidatures à l'élection présidentielle et extraits de programme sourcés.
//!
//! Une candidature n'est publiée qu'avec sa déclaration primaire. Les partis,
//! les groupes parlementaires et les propositions restent des relations
//! explicites : rien n'est déduit ni évalué.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CandidateError {
    #[error("candidate id must not be empty")]
    EmptyCandidateId,
    #[error("candidate id contains unsupported characters")]
    InvalidCandidateId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CandidateId(String);

impl CandidateId {
    pub fn new(raw: String) -> Result<Self, CandidateError> {
        if raw.is_empty() {
            return Err(CandidateError::EmptyCandidateId);
        }
        if !raw.bytes().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || raw.split('-').any(str::is_empty)
        {
            return Err(CandidateError::InvalidCandidateId);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CandidateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_id_is_a_stable_url_token() {
        assert_eq!(
            CandidateId::new("prenom-nom-2027".into()).unwrap().as_str(),
            "prenom-nom-2027"
        );
        assert_eq!(
            CandidateId::new("".into()),
            Err(CandidateError::EmptyCandidateId)
        );
        assert_eq!(
            CandidateId::new("Prenom Nom".into()),
            Err(CandidateError::InvalidCandidateId)
        );
        assert_eq!(
            CandidateId::new("candidate--a".into()),
            Err(CandidateError::InvalidCandidateId)
        );
    }
}
