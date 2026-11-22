//! Nouveautes du site presentables dans un Reel (RM-06).

use chrono::NaiveDate;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SiteNewsError {
    #[error("site news id must not be empty")]
    EmptyId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SiteNewsId(String);

impl SiteNewsId {
    pub fn new(raw: String) -> Result<Self, SiteNewsError> {
        if raw.trim().is_empty() {
            return Err(SiteNewsError::EmptyId);
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteNews {
    pub id: SiteNewsId,
    pub title: String,
    pub text: String,
    pub delivered_on: NaiveDate,
    pub path: String,
}
