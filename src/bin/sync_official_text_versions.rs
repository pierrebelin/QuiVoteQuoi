//! Synchronise le registre versionne des rattachements vote → texte.
//!
//! Cette commande s'execute sur le VPS, apres l'ingestion des scrutins. Elle
//! refuse un scrutin absent ou ambigu : aucun UID n'est devine depuis ici.
//!
//! Usage: cargo run --bin sync-official-text-versions

use quivotequoi::infrastructure::official_text_versions_registry::{
    bundled_versions, VerifiedOfficialTextVersion,
};
use sqlx::{PgPool, Row};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let versions = bundled_versions()?;

    if versions.is_empty() {
        println!("Aucune version de texte vérifiée à synchroniser.");
        return Ok(());
    }

    let pool = quivotequoi::infrastructure::config::try_connect_database()
        .await
        .map_err(std::io::Error::other)?;

    let mut synchronized = 0;
    for version in &versions {
        synchronize(&pool, version).await?;
        synchronized += 1;
    }

    println!("{synchronized} version(s) officielle(s) synchronisée(s).");
    Ok(())
}

async fn synchronize(
    pool: &PgPool,
    version: &VerifiedOfficialTextVersion,
) -> Result<(), Box<dyn std::error::Error>> {
    let matches =
        sqlx::query("SELECT uid FROM scrutins WHERE legislature = $1 AND number = $2 ORDER BY uid")
            .bind(i16::try_from(version.legislature)?)
            .bind(&version.scrutin_number)
            .fetch_all(pool)
            .await?;

    let [row] = matches.as_slice() else {
        return Err(std::io::Error::other(format!(
            "scrutin {} de la {}e législature absent ou ambigu ({})",
            version.scrutin_number,
            version.legislature,
            matches.len()
        ))
        .into());
    };
    let scrutin_uid: String = row.get("uid");

    sqlx::query(
        "INSERT INTO final_vote_text_versions (
            scrutin_uid, document_uid, document_title, version_label,
            document_published_on, official_url, mapping_source_url,
            source_producer, source_license, source_metadata_fingerprint,
            source_retrieved_at
         ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         ON CONFLICT (scrutin_uid) DO UPDATE SET
            document_uid = EXCLUDED.document_uid,
            document_title = EXCLUDED.document_title,
            version_label = EXCLUDED.version_label,
            document_published_on = EXCLUDED.document_published_on,
            official_url = EXCLUDED.official_url,
            mapping_source_url = EXCLUDED.mapping_source_url,
            source_producer = EXCLUDED.source_producer,
            source_license = EXCLUDED.source_license,
            source_metadata_fingerprint = EXCLUDED.source_metadata_fingerprint,
            source_retrieved_at = EXCLUDED.source_retrieved_at",
    )
    .bind(scrutin_uid)
    .bind(&version.document_uid)
    .bind(&version.document_title)
    .bind(&version.version_label)
    .bind(version.document_published_on)
    .bind(&version.official_url)
    .bind(&version.mapping_source_url)
    .bind(&version.source_producer)
    .bind(&version.source_license)
    .bind(&version.source_metadata_fingerprint)
    .bind(version.source_retrieved_at)
    .execute(pool)
    .await?;

    Ok(())
}
