#!/usr/bin/env bash
#
# Deploiement de QuiVoteQuoi sur le VPS. Tourne sous l'utilisateur
# `hemicycle`, appele par GitHub Actions via SSH :
#
#   ~/app/deploy/deploy.sh <sha>
#
# Strategie : ecrasement en place. Pas de dossier de release, pas de rollback
# automatique. Retour arriere = relancer ce script avec le SHA precedent.
#
# Options :
#   --skip-restart   compile et publie sans redemarrer le service (premier
#                    deploiement, avant que l'unite systemd n'existe)

set -euo pipefail

APP_DIR="${HOME}/app"
WWW_DIR="${HOME}/www"
HEALTH_URL="http://127.0.0.1:${PORT:-8085}/api/health"
HEALTH_RETRIES=30

TARGET_REF="main"
SKIP_RESTART=0

for arg in "$@"; do
    case "$arg" in
        --skip-restart) SKIP_RESTART=1 ;;
        --*)            echo "Option inconnue : $arg" >&2; exit 2 ;;
        *)              TARGET_REF="$arg" ;;
    esac
done

log() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

# rustup et nvm ne sont pas charges dans un shell SSH non interactif.
# shellcheck source=/dev/null
[ -f "${HOME}/.cargo/env" ] && source "${HOME}/.cargo/env"
# shellcheck source=/dev/null
[ -s "${HOME}/.nvm/nvm.sh" ] && source "${HOME}/.nvm/nvm.sh"

command -v cargo >/dev/null || { echo "cargo introuvable" >&2; exit 1; }
command -v node  >/dev/null || { echo "node introuvable" >&2; exit 1; }
command -v npm   >/dev/null || { echo "npm introuvable" >&2; exit 1; }

cd "$APP_DIR"

log "Recuperation de ${TARGET_REF}"
git fetch --prune origin
# reset --hard : l'etat du serveur devient exactement celui du commit. Toute
# modification faite a la main dans ~/app est perdue, volontairement.
if git rev-parse --verify --quiet "${TARGET_REF}^{commit}" >/dev/null; then
    git reset --hard "$TARGET_REF"
else
    git reset --hard "origin/${TARGET_REF}"
fi
git submodule update --init --recursive 2>/dev/null || true
echo "HEAD : $(git rev-parse --short HEAD) — $(git log -1 --pretty=%s)"

log "Compilation du backend"
# Les jobs systemd annexes executent directement les binaires de
# synchronisation et de capture : les construire au deploiement, jamais sous
# systemd (son systeme de fichiers est en lecture seule).
cargo build --release --locked --bins

log "Compilation du frontend"
cd "${APP_DIR}/frontend"
npm ci --no-audit --no-fund
npm run build
cd "$APP_DIR"

log "Publication des fichiers statiques"
# --delete purge les actifs de la version precedente. Fenetre de quelques
# centaines de millisecondes ou un visiteur peut recevoir une page cassee ;
# assume, voir todo/SPEC-DEPLOIEMENT.md §5.4.
rsync -a --delete "${APP_DIR}/frontend/dist/" "${WWW_DIR}/"

if [ "$SKIP_RESTART" -eq 1 ]; then
    log "Redemarrage ignore (--skip-restart)"
    exit 0
fi

log "Redemarrage du service"
# Les migrations sqlx en attente sont jouees ici, au demarrage du binaire.
sudo /usr/bin/systemctl restart hemicycle

log "Verification de sante"
for i in $(seq 1 "$HEALTH_RETRIES"); do
    if curl -fsS --max-time 5 "$HEALTH_URL" >/dev/null 2>&1; then
        echo "OK apres ${i}s"
        log "Deploiement termine — $(git rev-parse --short HEAD)"
        exit 0
    fi
    sleep 1
done

echo "ECHEC : ${HEALTH_URL} ne repond pas apres ${HEALTH_RETRIES}s" >&2
sudo /usr/bin/systemctl status hemicycle --no-pager || true
journalctl -u hemicycle -n 50 --no-pager || true
exit 1
