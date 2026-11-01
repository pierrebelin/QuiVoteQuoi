//! Fige les versions Open Data HTML des textes officiellement rattaches.
//!
//! Cette commande s'execute sur le VPS apres `sync-official-text-versions`.
//! Elle conserve le document HTML brut et un texte derive, mais ne publie rien
//! elle-meme : la reproduction pour les lecteurs reste une etape distincte.
//!
//! Usage: cargo run --bin capture-official-text-versions

use std::time::Duration;

use chrono::Utc;
use quivotequoi::infrastructure::official_text_versions_registry::{
    bundled_versions, VerifiedOfficialTextVersion,
};
use reqwest::Client;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};

const MINIMUM_TEXT_LENGTH: usize = 400;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let versions = bundled_versions()?;
    if versions.is_empty() {
        println!("Aucune version de texte vérifiée à capturer.");
        return Ok(());
    }

    let http = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let pool = quivotequoi::infrastructure::config::try_connect_database()
        .await
        .map_err(std::io::Error::other)?;

    let mut captured = 0;
    for version in &versions {
        capture(&pool, &http, version).await?;
        captured += 1;
    }
    println!("{captured} version(s) officielle(s) capturée(s).");
    Ok(())
}

async fn capture(
    pool: &PgPool,
    http: &Client,
    version: &VerifiedOfficialTextVersion,
) -> Result<(), Box<dyn std::error::Error>> {
    let scrutin_uid = scrutin_uid(pool, version).await?;
    ensure_reference_is_synchronized(pool, &scrutin_uid).await?;

    let response = http.get(&version.content_url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(std::io::Error::other(format!(
            "capture impossible pour le scrutin {}: HTTP {status}",
            version.scrutin_number
        ))
        .into());
    }
    let document_html = response.text().await?;
    let document_text = text_from_open_data_html(&document_html);
    if document_text.chars().count() < MINIMUM_TEXT_LENGTH {
        return Err(std::io::Error::other(format!(
            "capture refusee pour le scrutin {}: texte extrait trop court",
            version.scrutin_number
        ))
        .into());
    }
    let fingerprint = format!("sha256:{:x}", Sha256::digest(document_html.as_bytes()));

    sqlx::query(
        "INSERT INTO final_vote_text_contents (
            scrutin_uid, content_url, document_html, document_text,
            content_fingerprint, source_retrieved_at
         ) VALUES ($1, $2, $3, $4, $5, $6)
         ON CONFLICT (scrutin_uid) DO UPDATE SET
            content_url = EXCLUDED.content_url,
            document_html = EXCLUDED.document_html,
            document_text = EXCLUDED.document_text,
            content_fingerprint = EXCLUDED.content_fingerprint,
            source_retrieved_at = EXCLUDED.source_retrieved_at",
    )
    .bind(scrutin_uid)
    .bind(&version.content_url)
    .bind(document_html)
    .bind(document_text)
    .bind(fingerprint)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

async fn scrutin_uid(
    pool: &PgPool,
    version: &VerifiedOfficialTextVersion,
) -> Result<String, Box<dyn std::error::Error>> {
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
    Ok(row.get("uid"))
}

async fn ensure_reference_is_synchronized(
    pool: &PgPool,
    scrutin_uid: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let exists = sqlx::query("SELECT 1 FROM final_vote_text_versions WHERE scrutin_uid = $1")
        .bind(scrutin_uid)
        .fetch_optional(pool)
        .await?
        .is_some();
    if !exists {
        return Err(std::io::Error::other(
            "version de texte non synchronisée ; lancer sync-official-text-versions d'abord",
        )
        .into());
    }
    Ok(())
}

/// Extrait un texte lisible de l'HTML Open Data, sans prétendre en reconstruire
/// la mise en page. Le document HTML brut reste la copie de reference.
fn text_from_open_data_html(html: &str) -> String {
    let mut output = String::new();
    let mut cursor = 0;
    let mut ignored_element: Option<String> = None;

    while let Some(open_relative) = html[cursor..].find('<') {
        let open = cursor + open_relative;
        if ignored_element.is_none() {
            output.push_str(&html[cursor..open]);
        }
        let Some(close_relative) = html[open..].find('>') else {
            break;
        };
        let close = open + close_relative;
        let tag = html[open + 1..close].trim();
        let tag_name = tag
            .trim_start_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_ascii_lowercase();
        let closing = tag.starts_with('/');

        if let Some(ignored) = ignored_element.as_deref() {
            if closing && tag_name == ignored {
                ignored_element = None;
            }
        } else if !closing && matches!(tag_name.as_str(), "script" | "style") {
            ignored_element = Some(tag_name);
        } else if matches!(tag_name.as_str(), "p" | "div" | "br" | "li" | "tr") {
            output.push('\n');
        }
        cursor = close + 1;
    }
    if ignored_element.is_none() {
        output.push_str(&html[cursor..]);
    }

    normalize_text(&decode_html_entities(&output))
}

fn normalize_text(raw: &str) -> String {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn decode_html_entities(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        output.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find(';') else {
            output.push('&');
            rest = after;
            continue;
        };
        let entity = &after[..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ => numeric_entity(entity),
        };
        match decoded {
            Some(character) => output.push(character),
            None => {
                output.push('&');
                output.push_str(entity);
                output.push(';');
            }
        }
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    output
}

fn numeric_entity(entity: &str) -> Option<char> {
    let value = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
        .and_then(|raw| u32::from_str_radix(raw, 16).ok())
        .or_else(|| {
            entity
                .strip_prefix('#')
                .and_then(|raw| raw.parse::<u32>().ok())
        })?;
    char::from_u32(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_text_without_styles_scripts_or_images() {
        let html = r#"<html><head><style>.x { color: red }</style></head><body>
            <p>Article&#xa0;premier &amp; suite.</p><img src="data:image/png;base64,abc">
            <script>window.secret = 'not text'</script><div>Deuxième ligne.</div>
        </body></html>"#;

        assert_eq!(
            text_from_open_data_html(html),
            "Article premier & suite.\nDeuxième ligne."
        );
    }

    #[test]
    fn keeps_unknown_entities_visible_instead_of_silently_changing_them() {
        assert_eq!(decode_html_entities("A &inconnue; B"), "A &inconnue; B");
    }
}
