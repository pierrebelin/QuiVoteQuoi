#!/usr/bin/env bash
#
# Lance le workflow GitHub `reels.yml` (Reel, carrousel et story du jour,
# programmes pour 18 h dans Buffer). Les crons de GitHub sont « au mieux » :
# en novembre 2026 ils passaient avec 6 a 12 h de retard, ou pas du tout, et
# les posts du jour manquaient. Un `workflow_dispatch` part en quelques
# secondes : le VPS le declenche donc lui-meme.
#
# Declenchement par `hemicycle-reels-dispatch.timer`, deux passages par jour.
# Le second ne coute rien : le script du workflow ne redepose pas un post du
# jour deja programme.
#
# Environnement (/home/hemicycle/shared/.env) :
#   REELS_DISPATCH_TOKEN  jeton GitHub a grain fin, limite au depot, droit
#                         « Actions : lecture et ecriture ». Absent = desactive.
#   REELS_REPOSITORY      depot cible, `proprietaire/nom` (defaut ci-dessous).

set -euo pipefail

REPOSITORY="${REELS_REPOSITORY:-pierrebelin/QuiVoteQuoi}"
WORKFLOW="reels.yml"
REF="main"

if [[ -z "${REELS_DISPATCH_TOKEN:-}" ]]; then
    echo "$(date -u +%FT%TZ) — REELS_DISPATCH_TOKEN absent : declenchement desactive"
    exit 0
fi

# Date omise : le workflow prend le jour de Paris. 204 attendu, sans corps.
# Corps de reponse dans un fichier : sous systemd, stderr est une socket que
# curl ne peut pas ouvrir par /dev/stderr.
# Jeton passe par fichier, pas en argument : invisible dans `ps`.
body=$(mktemp)
auth=$(mktemp)
trap 'rm -f "$body" "$auth"' EXIT
printf 'Authorization: Bearer %s\n' "$REELS_DISPATCH_TOKEN" > "$auth"
status=$(curl --silent --show-error --max-time 30 \
    --output "$body" --write-out '%{http_code}' \
    --request POST \
    --header "Accept: application/vnd.github+json" \
    --header "@${auth}" \
    --header "X-GitHub-Api-Version: 2022-11-28" \
    --data '{"ref":"'"${REF}"'","inputs":{"dry_run":"false"}}' \
    "https://api.github.com/repos/${REPOSITORY}/actions/workflows/${WORKFLOW}/dispatches")

if [[ "$status" != "204" ]]; then
    echo "$(date -u +%FT%TZ) — declenchement de ${WORKFLOW} refuse (HTTP ${status})" >&2
    cat "$body" >&2
    exit 1
fi
echo "$(date -u +%FT%TZ) — ${WORKFLOW} declenche sur ${REPOSITORY}@${REF}"
