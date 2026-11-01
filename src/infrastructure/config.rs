use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Objets rattachés par rafraîchissement quand la variable n'est pas posée.
const THEME_BATCH_DEFAULT: i64 = 100;
/// Plafond accepté pour `THEME_BATCH_PER_REFRESH`.
const THEME_BATCH_MAX: i64 = 2_000;
/// Amendements écrits par rafraîchissement quand la variable n'est pas posée.
const AMENDMENT_BATCH_DEFAULT: usize = 40_000;
/// Synthèses générées par rafraîchissement quand la variable n'est pas posée.
const DOSSIER_SUMMARY_BATCH_DEFAULT: usize = 10;
/// Bornes acceptées pour `DOSSIER_SUMMARY_BATCH_PER_REFRESH`.
const DOSSIER_SUMMARY_BATCH_MIN: usize = 1;
const DOSSIER_SUMMARY_BATCH_MAX: usize = 100;

/// Connexions maximales du pool PostgreSQL.
const DB_MAX_CONNECTIONS: u32 = 5;
const DB_MIN_CONNECTIONS: u32 = 0;
/// Une connexion inutilisée est fermée au bout de ce délai.
const DB_IDLE_TIMEOUT: Duration = Duration::from_secs(300);
/// Attente maximale d'une connexion libre dans le pool.
const DB_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);

/// Objets soumis au rattachement thematique a chaque rafraichissement.
///
/// Plafond, pas objectif: le reliquat est repris au rafraichissement suivant
/// (RM-14). Les 322 textes de la legislature sont rattrapes en quelques passes,
/// et un rafraichissement de routine n'en trouve qu'une poignee de nouveaux.
/// `THEME_BATCH_PER_REFRESH=0` suspend la categorisation sans toucher au reste.
pub fn theme_batch_per_refresh() -> i64 {
    std::env::var("THEME_BATCH_PER_REFRESH")
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .unwrap_or(THEME_BATCH_DEFAULT)
        .clamp(0, THEME_BATCH_MAX)
}

/// Amendements ecrits a chaque rafraichissement.
///
/// Plafond, pas objectif: le reliquat est repris a la passe suivante. La
/// legislature en compte plusieurs centaines de milliers, et la fenetre du
/// timer d'ingestion est de deux heures: sans borne, une passe la deborderait.
/// `AMENDMENT_BATCH_PER_REFRESH=0` leve la borne, pour un premier chargement
/// lance a la main hors cadence.
pub fn amendment_batch_per_refresh() -> usize {
    std::env::var("AMENDMENT_BATCH_PER_REFRESH")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(AMENDMENT_BATCH_DEFAULT)
}

pub fn dossier_summary_batch_per_refresh() -> usize {
    std::env::var("DOSSIER_SUMMARY_BATCH_PER_REFRESH")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DOSSIER_SUMMARY_BATCH_DEFAULT)
        .clamp(DOSSIER_SUMMARY_BATCH_MIN, DOSSIER_SUMMARY_BATCH_MAX)
}

pub async fn try_connect_database() -> Result<PgPool, String> {
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL not set".to_string())?;

    let pool = PgPoolOptions::new()
        .max_connections(DB_MAX_CONNECTIONS)
        .min_connections(DB_MIN_CONNECTIONS)
        .idle_timeout(DB_IDLE_TIMEOUT)
        .acquire_timeout(DB_ACQUIRE_TIMEOUT)
        .connect(&database_url)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::migrate!()
        .run(&pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(pool)
}
