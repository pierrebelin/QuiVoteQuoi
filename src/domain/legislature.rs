//! Legislature couverte par le site.
//!
//! Le site expose les votes de la legislature en cours, et d'elle seule. Son
//! numero est porte ici, au domaine, plutot que dans un cas d'usage: les
//! sources, la persistance et l'ingestion s'y refere tous, aucun ne le decide.

/// Legislature couverte par le site (RM-07).
pub const CURRENT_LEGISLATURE: u16 = 17;

/// Ouverture de la legislature (18 juillet 2024): sa premiere semaine borne la consultation (RM-03).
pub const CURRENT_LEGISLATURE_OPENING: chrono::NaiveDate =
    match chrono::NaiveDate::from_ymd_opt(2024, 7, 18) {
        Some(date) => date,
        None => panic!("invalid legislature opening date"),
    };
