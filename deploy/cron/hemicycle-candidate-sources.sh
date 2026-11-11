#!/usr/bin/env bash
#
# Vérifie quotidiennement les sources déjà attribuées aux candidatures 2027.
# Une page modifiée est signalée dans le journal ; elle n'est jamais importée
# automatiquement : un nouvel extrait ou une nouvelle candidature exige une
# source primaire et une revue humaine (README.md §3.3 et §8.2).
#
# Certains sites bannissent les visiteurs trop assidus : une candidature dont
# un extrait cite un PDF n'est plus parcourue. Restent ses sources citées, une
# seule fois par document quel que soit le nombre de « #page=N », et une page
# de son site dont on ne compare que la liste des PDF liés, pour repérer un
# nouveau document.
#
# Une page HTML est comparée sur son texte visible normalisé (visible-text.pl),
# jamais sur son code brut : nonces, jetons et chronométrages y changent à
# chaque requête. Une page dont l'empreinte diffère est relue une fois : si la
# seconde lecture ne confirme pas la première, la page varie d'elle-même et
# rien n'est signalé.

set -euo pipefail

: "${DATABASE_URL:?DATABASE_URL est requis}"

readonly CURL_TIMEOUT=45
readonly CURL_CONNECT_TIMEOUT=10
readonly PAUSE_SECONDS=2
# Séparateur non blanc : read fusionnerait des tabulations consécutives et
# décalerait les champs quand ETag ou Last-Modified manque.
readonly FIELD_SEPARATOR=$'\x1f'
readonly SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
readonly TEMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TEMP_DIR"' EXIT

psql_base=(psql -X -q -v ON_ERROR_STOP=1 "$DATABASE_URL")

source_urls="$(${psql_base[@]} -At -F $'\t' <<'SQL'
WITH pdf_candidates AS (
    SELECT candidate_id AS id
    FROM candidate_program_proposals
    WHERE split_part(source_url, '#', 1) ILIKE '%.pdf'
    UNION
    SELECT id
    FROM presidential_candidates
    WHERE split_part(program_url, '#', 1) ILIKE '%.pdf'
),
other_candidates AS (
    SELECT *
    FROM presidential_candidates
    WHERE id NOT IN (SELECT id FROM pdf_candidates)
),
sources (source_url, pdf_links_only) AS (
    SELECT declaration_source_url, false
    FROM other_candidates
    UNION ALL
    SELECT official_site_url, false
    FROM other_candidates
    UNION ALL
    SELECT program_url, false
    FROM other_candidates
    UNION ALL
    SELECT link.source_url, false
    FROM candidate_political_organizations link
    JOIN other_candidates candidate ON candidate.id = link.candidate_id
    UNION ALL
    SELECT source_url, false
    FROM candidate_program_proposals
    UNION ALL
    SELECT program_url, false
    FROM presidential_candidates
    WHERE split_part(program_url, '#', 1) ILIKE '%.pdf'
    UNION ALL
    -- La page qui annoncerait un nouveau PDF : celle du programme, sinon le site.
    SELECT CASE
               WHEN split_part(program_url, '#', 1) ILIKE '%.pdf' THEN official_site_url
               ELSE COALESCE(program_url, official_site_url)
           END,
           true
    FROM presidential_candidates
    WHERE id IN (SELECT id FROM pdf_candidates)
)
-- Une page qui porte aussi un extrait garde sa comparaison complète.
SELECT split_part(source_url, '#', 1), bool_and(pdf_links_only)
FROM sources
WHERE source_url IS NOT NULL
GROUP BY 1
ORDER BY 1;
SQL
)"

if [[ -z "$source_urls" ]]; then
    echo "$(date -u +%FT%TZ) — aucune source de candidature a vérifier"
    exit 0
fi

record_observation() {
    local source_url="$1"
    local http_status="$2"
    local etag="$3"
    local last_modified="$4"
    local content_sha256="$5"
    local content_hash_method="${6:-}"

    "${psql_base[@]}" \
        --set=source_url="$source_url" \
        --set=http_status="$http_status" \
        --set=etag="$etag" \
        --set=last_modified="$last_modified" \
        --set=content_sha256="$content_sha256" \
        --set=content_hash_method="$content_hash_method" <<'SQL' >/dev/null
INSERT INTO candidate_program_source_observations (
    source_url, http_status, etag, last_modified, content_sha256, content_hash_method
) VALUES (
    :'source_url', :'http_status'::smallint, NULLIF(:'etag', ''),
    NULLIF(:'last_modified', ''), NULLIF(:'content_sha256', ''),
    NULLIF(:'content_hash_method', '')
)
ON CONFLICT (source_url) DO UPDATE SET
    last_checked_at = NOW(),
    http_status = EXCLUDED.http_status,
    etag = COALESCE(EXCLUDED.etag, candidate_program_source_observations.etag),
    last_modified = COALESCE(EXCLUDED.last_modified, candidate_program_source_observations.last_modified),
    content_sha256 = COALESCE(EXCLUDED.content_sha256, candidate_program_source_observations.content_sha256),
    content_hash_method = CASE
        WHEN EXCLUDED.content_sha256 IS NULL
        THEN candidate_program_source_observations.content_hash_method
        ELSE EXCLUDED.content_hash_method
    END,
    -- Deux empreintes ne se comparent que calculées de la même façon : un
    -- changement de méthode rebase l'empreinte sans signaler de modification.
    last_changed_at = CASE
        WHEN candidate_program_source_observations.content_sha256 <> EXCLUDED.content_sha256
         AND candidate_program_source_observations.content_hash_method
             IS NOT DISTINCT FROM EXCLUDED.content_hash_method
        THEN NOW()
        ELSE candidate_program_source_observations.last_changed_at
    END;
SQL
}

# Liste triée et dédoublonnée : un jeton ou une bannière ne la fait pas varier.
list_pdf_links() {
    { grep -aoiE "[^\"'<>[:space:]()]+\.pdf" "$1" || true; } | LC_ALL=C sort -u
}

# Empreinte d'une réponse et sa méthode, séparées par une tabulation. Une page
# HTML est comparée sur son texte visible (visible-text.pl) : son code brut
# change à chaque requête. Un PDF ou un autre document l'est octet pour octet.
# Changer la méthode d'un type de source impose une nouvelle valeur, acceptée
# par la migration de `content_hash_method`.
fingerprint() {
    local body="$1" headers="$2" pdf_links_only="$3" content_type
    if [[ "$pdf_links_only" == "t" ]]; then
        printf '%s\tpdf-links\n' "$(list_pdf_links "$body" | sha256sum | awk '{print $1}')"
        return
    fi
    # Après redirection, le dernier Content-Type est celui du document servi.
    content_type="$(awk 'tolower($0) ~ /^content-type:/ { result = tolower($0) } END { print result }' "$headers")"
    if [[ "$content_type" == *html* ]] \
        || { [[ -z "$content_type" ]] && head -c 4096 "$body" | grep -qi '<html'; }; then
        printf '%s\tvisible-text-v1\n' "$(perl "$SCRIPT_DIR/visible-text.pl" < "$body" | sha256sum | awk '{print $1}')"
    else
        printf '%s\traw\n' "$(sha256sum "$body" | awk '{print $1}')"
    fi
}

failures=0

while IFS=$'\t' read -r source_url pdf_links_only; do
    sleep "$PAUSE_SECONDS"

    state="$("${psql_base[@]}" -At -F "$FIELD_SEPARATOR" --set=source_url="$source_url" <<'SQL'
SELECT COALESCE(etag, ''), COALESCE(last_modified, ''), COALESCE(content_sha256, ''),
       COALESCE(content_hash_method, '')
FROM candidate_program_source_observations
WHERE source_url = :'source_url';
SQL
)"
    IFS="$FIELD_SEPARATOR" read -r previous_etag previous_last_modified previous_hash previous_method <<< "$state"

    headers="$TEMP_DIR/headers"
    body="$TEMP_DIR/body"
    curl_args=(
        --location --silent --show-error --compressed
        --connect-timeout "$CURL_CONNECT_TIMEOUT" --max-time "$CURL_TIMEOUT"
        --dump-header "$headers" --output "$body" --write-out '%{http_code}'
    )
    [[ -n "$previous_etag" ]] && curl_args+=(--header "If-None-Match: $previous_etag")
    [[ -n "$previous_last_modified" ]] && curl_args+=(--header "If-Modified-Since: $previous_last_modified")

    if ! status="$(curl "${curl_args[@]}" "$source_url")"; then
        echo "$(date -u +%FT%TZ) — source inaccessible : $source_url" >&2
        record_observation "$source_url" 0 "" "" ""
        failures=$((failures + 1))
        continue
    fi

    etag="$(awk 'tolower($0) ~ /^etag:/ { value = $0; sub(/^[^:]*:[[:space:]]*/, "", value); sub(/\r$/, "", value); result = value } END { print result }' "$headers")"
    last_modified="$(awk 'tolower($0) ~ /^last-modified:/ { value = $0; sub(/^[^:]*:[[:space:]]*/, "", value); sub(/\r$/, "", value); result = value } END { print result }' "$headers")"

    if [[ "$status" == "304" ]]; then
        record_observation "$source_url" "$status" "$etag" "$last_modified" ""
        echo "$(date -u +%FT%TZ) — source inchangée : $source_url"
        continue
    fi

    # Certains éditeurs refusent volontairement les requêtes automatisées. La
    # source affichée reste valide ; seule sa vérification périodique est
    # impossible. Le signaler sans dégrader l'état du timer chaque jour.
    if [[ "$status" == "401" || "$status" == "403" || "$status" == "429" ]]; then
        echo "$(date -u +%FT%TZ) — vérification automatique refusée (HTTP $status), source conservée : $source_url" >&2
        record_observation "$source_url" "$status" "$etag" "$last_modified" ""
        continue
    fi

    if [[ ! "$status" =~ ^2[0-9][0-9]$ ]]; then
        echo "$(date -u +%FT%TZ) — source en erreur HTTP $status : $source_url" >&2
        record_observation "$source_url" "$status" "$etag" "$last_modified" ""
        failures=$((failures + 1))
        continue
    fi

    IFS=$'\t' read -r content_hash content_hash_method < <(fingerprint "$body" "$headers" "$pdf_links_only")
    if [[ ! "$content_hash" =~ ^[0-9a-f]{64}$ ]]; then
        echo "$(date -u +%FT%TZ) — empreinte impossible à calculer : $source_url" >&2
        record_observation "$source_url" "$status" "$etag" "$last_modified" ""
        failures=$((failures + 1))
        continue
    fi

    if [[ -z "$previous_hash" ]]; then
        outcome="référencée pour la première fois"
    elif [[ "$previous_method" != "$content_hash_method" ]]; then
        outcome="rebasée sur une empreinte $content_hash_method, sans comparaison"
    elif [[ "$previous_hash" == "$content_hash" ]]; then
        outcome="inchangée"
    else
        # Une page qui varie d'une requête à l'autre (bandeau tiré au hasard)
        # signalerait une modification chaque jour. Une seconde lecture le
        # distingue d'un vrai changement : l'empreinte connue est alors gardée.
        sleep "$PAUSE_SECONDS"
        recheck_hash=""
        if curl --location --silent --compressed \
                --connect-timeout "$CURL_CONNECT_TIMEOUT" --max-time "$CURL_TIMEOUT" \
                --dump-header "$TEMP_DIR/recheck-headers" --output "$TEMP_DIR/recheck-body" \
                "$source_url"; then
            IFS=$'\t' read -r recheck_hash _ < <(fingerprint "$TEMP_DIR/recheck-body" "$TEMP_DIR/recheck-headers" "$pdf_links_only")
        fi
        if [[ "$recheck_hash" != "$content_hash" ]]; then
            echo "$(date -u +%FT%TZ) — contenu variable d'une requête à l'autre, comparaison impossible, empreinte connue conservée : $source_url" >&2
            record_observation "$source_url" "$status" "$etag" "$last_modified" ""
            continue
        fi
        if [[ "$pdf_links_only" == "t" ]]; then
            outcome="liens PDF modifiés — nouveau document possible, revue humaine requise"
        else
            outcome="modifiée — revue humaine requise avant toute publication"
        fi
    fi

    record_observation "$source_url" "$status" "$etag" "$last_modified" "$content_hash" "$content_hash_method"
    echo "$(date -u +%FT%TZ) — source $outcome : $source_url"
done <<< "$source_urls"

exit "$failures"
