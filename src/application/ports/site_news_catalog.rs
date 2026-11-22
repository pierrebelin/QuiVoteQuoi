use crate::domain::site_news::SiteNews;

/// Nouveautes du site, lues dans un fichier versionne : sans IO.
pub trait SiteNewsCatalog: Send + Sync {
    fn all(&self) -> Vec<SiteNews>;
}
