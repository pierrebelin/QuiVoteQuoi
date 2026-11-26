#!/usr/bin/env bash
#
# File de revue des sources de programme (README.md §8.2) : une issue ouverte
# par candidature, une section par source modifiée avec les fragments ajoutés
# et retirés. Un nouveau changement d'une candidature déjà en revue s'ajoute en
# commentaire daté au lieu d'ouvrir une autre issue ; les sections d'un même
# passage partent dans un seul message. Rien n'est republié automatiquement.
#
# Usage : program-source-issues.sh <réponse de /api/candidats-2027/sources-modifiees>
# Environnement : GH_REPO (owner/nom), GH_TOKEN ; DRY_RUN=1 affiche les issues
# et commentaires au lieu de les publier.

set -euo pipefail

readonly RESPONSE="${1:?réponse JSON de sources-modifiees attendue}"
: "${GH_REPO:?GH_REPO est requis}"
readonly LABEL=programmes
# Les marqueurs des changements déjà signalés se trouvent dans les corps
# d'issue et dans les commentaires. La fenêtre de l'API ne dépasse pas 90
# jours : les commentaires plus anciens ne peuvent plus rien dédoublonner.
readonly COMMENT_LOOKBACK_DAYS=95
# Un corps ou un commentaire GitHub plafonne à 65 536 caractères : marge pour
# l'en-tête et le pied de l'issue.
readonly MAX_POST_BYTES=60000

WORK_DIR="$(mktemp -d)"
readonly WORK_DIR
trap 'rm -rf "$WORK_DIR"' EXIT

if [[ "${DRY_RUN:-}" != "1" ]]; then
    gh label create "$LABEL" --force \
        --description "Source de programme modifiée, revue humaine requise" >/dev/null
fi

# API REST paginée : `gh issue list` plafonne à 1 000 résultats, et un marqueur
# absent de la liste ferait signaler deux fois le même changement.
gh api --paginate "repos/${GH_REPO}/issues?labels=${LABEL}&state=all&per_page=100" \
    --jq '.[] | {number, state, title, body: (.body // "")}' > "$WORK_DIR/issues.jsonl"

since="$(date -u -d "-${COMMENT_LOOKBACK_DAYS} days" +%FT%TZ 2>/dev/null \
    || date -u -v-"${COMMENT_LOOKBACK_DAYS}"d +%FT%TZ)"
gh api --paginate "repos/${GH_REPO}/issues/comments?since=${since}&per_page=100" \
    --jq '.[].body // empty | select(contains("source-change:"))' > "$WORK_DIR/comments.txt"

{
    jq -r '.body' "$WORK_DIR/issues.jsonl"
    cat "$WORK_DIR/comments.txt"
} | { grep -oE 'source-change:[^ ]+@[^ ]+' || true; } > "$WORK_DIR/known.txt"

# Titre → issue ouverte qui le porte, la plus ancienne si plusieurs. Le titre
# nomme la candidature : toutes ses sources partagent une même issue.
jq -sr 'sort_by(.number) | .[]
        | select(.state == "open")
        | "\(.title)\t\(.number)"' "$WORK_DIR/issues.jsonl" > "$WORK_DIR/open.tsv"

open_issue_for() {
    awk -F'\t' -v title="$1" '$1 == title { print $2; exit }' "$WORK_DIR/open.tsv"
}

# Une source rattachée à aucune candidature garde sa propre issue, nommée
# d'après son adresse.
title_for() {
    jq -r '"Programme modifié : "
           + (if (.candidates | length) == 0
              then "\(.source_url) (aucune candidature rattachée)"
              else (.candidates | join(", ")) end)' <<<"$1"
}

# Section « Modification du … » : date, source et extrait. Les fragments sont
# cités dans un bloc de code : ni Markdown interprété, ni mention notifiée.
render_change() {
    jq -r '
      def fence: "~~~text\n" + (map(gsub("~~~"; "~ ~ ~")) | join("\n")) + "\n~~~";
      def part($title; $lines; $total):
        if $total == 0 then "**\($title)** : rien."
        elif ($lines | length) < $total
        then "**\($title)** (\($total) fragments, les \($lines | length) premiers) :\n\n" + ($lines | fence)
        else "**\($title)** (\($total) fragment\(if $total > 1 then "s" else "" end)) :\n\n" + ($lines | fence)
        end;
      (.changed_at | sub("\\.[0-9]+"; "") | fromdateiso8601 | strftime("%d/%m/%Y à %H:%M UTC")) as $date
      | "## Modification du \($date)\n\nSource : <\(.source_url)>\n\n"
        + (if .excerpt == null
           then "Extrait indisponible : la lecture précédente n’avait pas gardé de texte, ou le document n’est ni une page HTML ni un PDF lisible. Relire la source."
           else part("Ajouté"; .excerpt.added; .excerpt.added_total) + "\n\n"
                + part("Retiré"; .excerpt.removed; .excerpt.removed_total)
           end)
    ' <<<"$1"
}

# Une section par changement encore inconnu, rangée sous le titre de sa
# candidature.
: > "$WORK_DIR/pending.tsv"
count=0
while IFS= read -r change; do
    url="$(jq -r '.source_url' <<<"$change")"
    changed_at="$(jq -r '.changed_at' <<<"$change")"
    marker="source-change:${url}@${changed_at}"
    if grep -qxF -- "$marker" "$WORK_DIR/known.txt"; then
        continue
    fi
    echo "$marker" >> "$WORK_DIR/known.txt"
    count=$((count + 1))
    {
        render_change "$change"
        echo
        echo "<!-- ${marker} -->"
    } > "$WORK_DIR/section-${count}.md"
    printf '%s\t%s\n' "$(title_for "$change")" "$count" >> "$WORK_DIR/pending.tsv"
done < <(jq -c '.changed_sources | sort_by(.changed_at) | .[]' "$RESPONSE")

# Publie un message : corps d'une nouvelle issue s'il n'y en a pas d'ouverte
# pour ce titre, commentaire sinon.
publish() {
    local title="$1" post="$2"
    local issue
    issue="$(open_issue_for "$title")"
    if [[ -n "$issue" ]]; then
        if [[ "${DRY_RUN:-}" == "1" ]]; then
            echo "=== commentaire sur #${issue}"
            cat "$post"
        else
            gh issue comment "$issue" --body-file "$post" >/dev/null
        fi
        return
    fi
    {
        echo "Une source du programme de cette candidature a changé. Tant que cette issue reste ouverte, les modifications suivantes de ses sources s'y ajoutent en commentaire, une section par source."
        echo
        echo "À faire :"
        echo "- [ ] Relire chaque source et comparer avec les extraits publiés"
        echo "- [ ] Mettre à jour \`scripts/candidates-2027-proposals.json\` si un extrait a changé"
        echo "- [ ] Merger sur \`main\` : le déploiement relance \`scripts/seed-candidates-2027.mjs\` de lui-même"
        echo "- [ ] Fermer l'issue"
        echo
        cat "$post"
        echo
        echo "Rien n'est republié automatiquement. Une page HTML est comparée sur son texte visible, sans le code ni l'ordre des blocs, et seulement si une seconde lecture confirme le changement : un bandeau ou un pied de page modifié déclenche encore l'alerte, auquel cas fermer l'issue. Pour une candidature qui cite un PDF, la page de son site n'est comparée que sur la liste de ses PDF liés : l'extrait montre alors le document ajouté ou retiré."
    } > "$WORK_DIR/body.md"
    if [[ "${DRY_RUN:-}" == "1" ]]; then
        issue="dry-run-$(wc -l < "$WORK_DIR/open.tsv" | tr -d ' ')"
        echo "=== nouvelle issue : ${title}"
        cat "$WORK_DIR/body.md"
    else
        local created
        created="$(gh issue create --label "$LABEL" --title "$title" \
            --body-file "$WORK_DIR/body.md")"
        issue="${created##*/}"
    fi
    printf '%s\t%s\n' "$title" "$issue" >> "$WORK_DIR/open.tsv"
}

# Les sections d'une même candidature partent dans un seul message, découpé
# sous la limite de GitHub quand elles sont nombreuses.
cut -f1 "$WORK_DIR/pending.tsv" | awk '!seen[$0]++' | while IFS= read -r title; do
    post="$WORK_DIR/post.md"
    : > "$post"
    while IFS= read -r index; do
        section="$WORK_DIR/section-${index}.md"
        if [[ -s "$post" ]] \
            && (( $(wc -c < "$post") + $(wc -c < "$section") > MAX_POST_BYTES )); then
            publish "$title" "$post"
            : > "$post"
        fi
        [[ -s "$post" ]] && echo >> "$post"
        cat "$section" >> "$post"
    done < <(awk -F'\t' -v title="$title" '$1 == title { print $2 }' "$WORK_DIR/pending.tsv")
    publish "$title" "$post"
done
