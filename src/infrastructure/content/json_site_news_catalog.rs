//! Fichier versionne des nouveautes du site (`content/nouveautes.json`).

use chrono::NaiveDate;
use serde::Deserialize;

use crate::application::ports::site_news_catalog::SiteNewsCatalog;
use crate::domain::site_news::{SiteNews, SiteNewsId};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SiteNewsEntry {
    pub id: String,
    pub title: String,
    pub text: String,
    pub delivered_on: NaiveDate,
    pub path: String,
}

const VERSIONED_NEWS: &str = include_str!("../../../content/nouveautes.json");

pub struct JsonSiteNewsCatalog {
    news: Vec<SiteNews>,
}

impl JsonSiteNewsCatalog {
    pub fn parse(json: &str) -> Result<Vec<SiteNewsEntry>, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    /// Catalogue du fichier versionne, lu une fois au demarrage.
    pub fn embedded() -> Result<Self, String> {
        let news = Self::parse(VERSIONED_NEWS)?
            .into_iter()
            .map(|entry| {
                Ok(SiteNews {
                    id: SiteNewsId::new(entry.id).map_err(|e| e.to_string())?,
                    title: entry.title,
                    text: entry.text,
                    delivered_on: entry.delivered_on,
                    path: entry.path,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Self { news })
    }
}

impl SiteNewsCatalog for JsonSiteNewsCatalog {
    fn all(&self) -> Vec<SiteNews> {
        self.news.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERSIONED_FILE: &str = include_str!("../../../content/nouveautes.json");

    #[test]
    fn reads_every_entry_of_the_versioned_file_with_its_delivery_date() {
        let entries = JsonSiteNewsCatalog::parse(VERSIONED_FILE).expect("versioned file parses");

        let week = entries
            .iter()
            .find(|entry| entry.id == "semaine")
            .expect("entry semaine present");
        assert_eq!(
            week.delivered_on,
            NaiveDate::from_ymd_opt(2026, 11, 13).unwrap()
        );
        assert_eq!(week.path, "/semaine");
    }
}
