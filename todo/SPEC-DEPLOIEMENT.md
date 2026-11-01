# SPEC — Déploiement VPS Debian 13 + Nginx

Cible : `https://<DOMAINE_PUBLIC>`, VPS Debian 13 (trixie) déjà en service
(Nginx installé, une application Node déjà hébergée). Déploiement automatique à
chaque merge sur `main` via GitHub Actions.

## 1. Décisions

| Sujet | Décision |
|---|---|
| Utilisateur système | `hemicycle`, sans mot de passe, connexion par clé SSH uniquement |
| Emplacement | tout sous `/home/hemicycle` |
| Compilation | sur le VPS (Rust + Node installés sur la machine) |
| Stratégie release | écrasement en place, pas de dossiers horodatés, pas de rollback automatique |
| Migrations | jouées par le binaire au démarrage, `sqlx` ne rejoue jamais une migration déjà appliquée |
| Base | PostgreSQL local sur le VPS |
| Secrets | posés à la main une fois dans `/home/hemicycle/shared/.env`, jamais dans GitHub |
| Port backend | `8085` en local (le `3000` par défaut est supposé pris par l'application Node) |
| Écriture publique | tous les `POST /api/` exigent un identifiant HTTP Basic sur le vhost public (`401` sans lui) |
| Écriture applicative | jeton du jour obligatoire, dérivé de `ADMIN_TOKEN_SECRET` (§3.2) |

## 2. Arborescence cible

```
/home/hemicycle/
  .ssh/
    authorized_keys          → clé publique de déploiement (GitHub Actions)
    id_ed25519               → clé de lecture du dépôt privé (deploy key GitHub)
  app/                       → clone du dépôt, branche main
    target/release/hemicycle-data
    frontend/dist/
  www/                       → copie du build front, servie par Nginx
  shared/
    .env                     → secrets, mode 600
  bin/
    deploy.sh                → script de déploiement (copié depuis app/deploy/)
```

L'ingestion périodique ne pose aucun fichier : elle est déclenchée par un timer
systemd et journalise dans journald (§3.3). Un `shared/ingest.log` n'apparaît
que si l'on choisit le repli par crontab.

`www/` est distinct de `app/frontend/dist/` pour que Nginx n'ait à lire que ce
dossier : le reste du home reste en `750`, le code source n'est jamais exposé.

## 3. Architecture d'exécution

```
Internet ──► Nginx :443 (vhost public, TLS Let's Encrypt)
               ├─ /            → fichiers statiques /home/hemicycle/www (fallback SPA)
               ├─ POST /api/   → HTTP Basic, puis proxy 127.0.0.1:8085
               └─ GET  /api/   → proxy 127.0.0.1:8085

Tunnel SSH ──► Nginx :8080 sur 127.0.0.1 (vhost admin)
               ├─ /            → mêmes fichiers statiques
               └─ /api/        → proxy 127.0.0.1:8085, toutes méthodes

systemd (hemicycle.service, User=hemicycle)
   └─ /home/hemicycle/app/target/release/hemicycle-data
         EnvironmentFile=/home/hemicycle/shared/.env
         PORT=8085

systemd (hemicycle-ingest.timer → .service, toutes les 2 h)
   └─ /home/hemicycle/app/deploy/cron/hemicycle-ingest.sh
         POST 127.0.0.1:8085/api/{registry/refresh,scrutins/refresh,refresh}
         (`refresh` porte aussi l'extraction des textes et le rattachement)
         en-tête x-admin-token : jeton du jour

systemd (hemicycle-text-capture.timer → .service, chaque jour)
   └─ /home/hemicycle/app/deploy/cron/hemicycle-text-capture.sh
         binaires release : synchronisation du registre puis capture Open Data
         PostgreSQL direct, sans route d'administration ni appel LLM
```

### 3.1 Pourquoi ce découpage pour l'écriture

Le front est du JavaScript public : aucune valeur qu'il embarque (jeton, en-tête,
clé) n'est un secret, et `Origin`/`Referer` sont falsifiables par un simple
`curl`. Restreindre les appels « au front seul » n'est pas réalisable.

En revanche, les huit routes en écriture ne sont appelées que par des écrans
d'administration :

| Route | Appelée depuis |
|---|---|
| `POST /api/refresh` | `DossierSelectionPage` |
| `POST /api/registry/refresh` | aucun front (outil) |
| `POST /api/scrutins/refresh` | aucun front (outil) |
| `POST /api/dossiers/{uid}/curate` | `DossierDetailPage`, `DossierSelectionPage` |
| `POST /api/dossiers/{uid}/save` | `DossierDetailPage` |
| `POST /api/themes/extract` | aucun front (outil) |
| `POST /api/themes/propose` | aucun front (outil) |
| `POST /api/themes/arbitrate` | `ThemeArbitrationPage` |

Aucun parcours de consultation n'écrit. Le vhost public exige donc un
identifiant HTTP Basic (`auth_basic`) sur toute méthode autre que
`GET`/`HEAD`/`OPTIONS` sous `/api/`, et répond `401` sans lui. Sans cela,
n'importe qui déclenche les ingestions Assemblée nationale et consomme la clé
Anthropic.

L'opérateur administre depuis le site en ligne : le navigateur demande
l'identifiant à la première écriture, puis le renvoie seul ; l'écran demande
ensuite le jeton du jour (§3.2). Le fichier d'identifiants
`/etc/nginx/hemicycle-admin.htpasswd` est posé à la main (§4.7), jamais
versionné.

Le tunnel SSH vers le vhost admin reste disponible, sans identifiant HTTP Basic :

```bash
ssh -N -L 8080:127.0.0.1:8080 hemicycle@<IP_DU_VPS>
```

puis `http://localhost:8080` dans le navigateur. Le vhost admin n'écoute que sur
la boucle locale, il n'est joignable d'aucune façon depuis Internet.

### 3.2 Garde applicative et jeton du jour

Les trois dettes que portait cette section sont soldées.

**Le filtre n'est plus seulement périmétrique.** Un middleware Axum
(`src/api/security.rs`) exige un jeton sur les huit routes d'écriture. Le
`limit_except` du vhost public reste en place : deux barrières, dont une qui
tient même si un autre service du VPS atteint `127.0.0.1:8085` en direct. Un
jeton qui fuite ne suffit pas sans l'identifiant HTTP Basic, et inversement.

**Le jeton change tous les jours.** Il n'est pas posé dans le `.env` et n'est
stocké nulle part — il est dérivé du secret et de la date UTC :

```text
jeton(jour) = hex(HMAC-SHA256(ADMIN_TOKEN_SECRET, "AAAA-MM-JJ"))[..32]
```

Le serveur accepte le jour courant et la veille. Sans cette tolérance, une tâche
CRON lancée à 23 h 59 s'authentifierait avec un jeton périmé à 00 h 00, et
l'opérateur serait déconnecté en plein arbitrage au passage de minuit. Fenêtre
d'exposition d'un jeton qui fuite : 48 h au plus. Révocation immédiate :
changer `ADMIN_TOKEN_SECRET` et redémarrer.

Les deux appelants légitimes le dérivent du même secret :

| Appelant | Comment |
|---|---|
| Écran d'administration (site public après identifiant HTTP Basic, ou tunnel SSH) | l'opérateur colle le jeton du jour dans le champ prévu |
| Tâche CRON du VPS | `deploy/cron/hemicycle-ingest.sh` appelle `deploy/bin/admin-token.sh` |

```bash
ssh hemicycle@<IP_DU_VPS> '~/app/deploy/bin/admin-token.sh'
```

Le secret ne transite jamais par `argv` — `ps` l'exposerait à tout utilisateur
de la machine. `admin-token.sh` charge `~/shared/.env` puis exécute le binaire
`admin-token`, qui lit l'environnement.

**Écoute sur la boucle locale.** `BIND_ADDR` vaut `127.0.0.1` par défaut : le
pare-feu n'est plus la seule chose qui empêche d'atteindre l'API en direct sur
le port 8085. `ufw` reste à vérifier, il n'est simplement plus seul.

**CORS explicite.** `CorsLayer::permissive()` est remplacé par une liste
d'origines lue dans `ALLOWED_ORIGINS`, vide par défaut.

**Ce qui n'est pas protégé, et ne peut pas l'être** : les routes de
consultation. Le site publie de la donnée publique (README.md §2) ; un jeton
embarqué dans un bundle JavaScript public serait lisible par n'importe qui et ne
protégerait rien.

### 3.3 Ingestion périodique

`deploy/cron/hemicycle-ingest.sh` enchaîne `registry/refresh`,
`scrutins/refresh` puis `refresh`, dans cet ordre : sans acteurs à jour un
scrutin référence des députés inconnus, et sans scrutins l'extraction ne voit
rien.

Depuis le 9 août 2026, `POST /api/refresh` porte lui-même l'extraction des
textes débattus et le rattachement thématique — appeler `themes/extract` après
lui le rejouerait à vide. **Il consomme donc la clé Anthropic**, ce qui n'était
pas le cas auparavant. Trois garde-fous rendent la dépense prévisible :

- le porteur du thème est le texte, pas le scrutin — 8 434 scrutins tiennent en
  322 textes, et scrutins comme dossiers en héritent sans appel supplémentaire ;
- un objet déjà rattaché n'est jamais resoumis, et une table de règles publiée
  prend sans appel ce que la nature du texte suffit à classer ;
- `THEME_BATCH_PER_REFRESH` (100 par défaut) plafonne le nombre d'objets soumis
  par passe ; le reliquat part à la suivante. `0` suspend le rattachement sans
  toucher au reste de l'ingestion.

Une passe de routine ne trouve donc qu'une poignée d'objets nouveaux.
`POST /api/themes/propose` reste disponible pour rattraper un arriéré à la
main, hors cadence.

**Le déclencheur est `hemicycle-ingest.timer`, toutes les deux heures.** Le
timer est préféré à une ligne de crontab pour trois raisons : la sortie part
dans journald plutôt que dans un `ingest.log` que personne ne fait tourner,
`Persistent=true` rejoue la passe manquée après un redémarrage, et systemd
n'exécute jamais deux instances de la même unité en parallèle.

> **Les deux mécanismes s'excluent.** Si le timer est activé, il ne faut *pas*
> installer la ligne de crontab, sinon l'ingestion tourne deux fois sur deux
> horaires. La ligne reste documentée en tête du script comme repli.

Deux heures plutôt qu'une fois par jour : le script attaque des routes locales,
et côté Assemblée nationale un passage coûte trois requêtes conditionnelles
(§6.2) qui se résolvent en `304 Not Modified` tant que les archives n'ont pas
changé. La cadence est donc quasi gratuite pour la source comme pour le VPS.
Sans la revalidation conditionnelle, il faudrait l'espacer.

Le changement de jeton à minuit n'est pas un obstacle : le serveur accepte le
jeton du jour **et** celui de la veille (§3.2), quelle que soit l'heure du
passage.

### 3.4 Capture quotidienne des textes officiels

`hemicycle-text-capture.timer` déclenche chaque jour, après le passage
d'ingestion de 04:07, `deploy/cron/hemicycle-text-capture.sh`. Le script attend
le health check, puis exécute les binaires release dans cet ordre :

1. `sync-official-text-versions` rattache le registre versionné aux scrutins
   effectivement présents en base ;
2. `capture-official-text-versions` télécharge chaque document référencé, en
   conserve le HTML, le texte dérivé, l'empreinte SHA-256 et la date de collecte.

Cette tâche n'appelle pas l'API d'administration et ne consomme aucune clé LLM.
Elle utilise seulement `DATABASE_URL` et les URLs publiques du registre. Une
erreur de référence, de réseau ou de contenu court fait échouer l'unité et reste
visible dans journald ; le passage quotidien suivant la réessaie.

## 4. Préparation du serveur (une seule fois, en root)

### 4.1 Utilisateur

```bash
adduser --disabled-password --gecos "" hemicycle
install -d -m 700 -o hemicycle -g hemicycle /home/hemicycle/.ssh
install -d -m 750 -o hemicycle -g hemicycle /home/hemicycle/www
install -d -m 700 -o hemicycle -g hemicycle /home/hemicycle/shared
install -d -m 750 -o hemicycle -g hemicycle /home/hemicycle/bin
chmod 750 /home/hemicycle
usermod -a -G hemicycle www-data
```

`www-data` (Nginx) entre dans le groupe `hemicycle` pour lire `www/` ; le home en
`750` lui interdit tout le reste.

### 4.2 Droit de redémarrage

Fichier `/etc/sudoers.d/hemicycle`, créé avec `visudo -f`, mode `440` :

```
hemicycle ALL=(root) NOPASSWD: /usr/bin/systemctl restart hemicycle, /usr/bin/systemctl status hemicycle, /usr/bin/systemctl is-active hemicycle
```

Aucun autre droit `sudo`. Vérifier le chemin réel de `systemctl` avec
`command -v systemctl` — un chemin faux dans `sudoers` est une passoire.

### 4.3 Clés SSH

Deux clés distinctes, deux rôles.

**a. Clé d'entrée (GitHub Actions → VPS)** — générée sur le poste local :

```bash
ssh-keygen -t ed25519 -f ~/.ssh/hemicycle_deploy -C "github-actions@hemicycle" -N ""
```

La partie publique va dans `/home/hemicycle/.ssh/authorized_keys` (mode `600`,
propriétaire `hemicycle`). La partie privée va dans le secret GitHub
`VPS_SSH_KEY`. Restreindre la clé dans `authorized_keys` :

```
no-agent-forwarding,no-X11-forwarding,no-user-rc ssh-ed25519 AAAA... github-actions@hemicycle
```

Ne pas mettre `command=` : le workflow a besoin de lancer `deploy.sh` avec un
argument (le SHA).

**b. Clé de lecture du dépôt (VPS → GitHub)** — le dépôt est privé, générée sur
le VPS en tant que `hemicycle` :

```bash
ssh-keygen -t ed25519 -f /home/hemicycle/.ssh/id_ed25519 -N ""
cat /home/hemicycle/.ssh/id_ed25519.pub
```

Ajouter cette clé publique dans GitHub → Settings du dépôt → Deploy keys, en
**lecture seule**. Puis, toujours en tant que `hemicycle` :

```bash
ssh-keyscan github.com >> /home/hemicycle/.ssh/known_hosts
git clone git@github.com:pierrebelin/QuiVoteQuoi.git /home/hemicycle/app
```

### 4.4 Chaînes de compilation

En tant que `hemicycle` :

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Node : la version LTS système suffit si elle est ≥ 20 (Vite 8 l'exige) ;
`node -v` pour vérifier. Si l'application Node déjà présente impose une autre
version, installer `nvm` sous `hemicycle` et poser un `.nvmrc` dans `frontend/`.

### 4.5 Secrets

Fichier `/home/hemicycle/shared/.env`, propriétaire `hemicycle`, mode `600`,
écrit à la main, jamais versionné, jamais dans les secrets GitHub :

```
DATABASE_URL=postgresql://hemicycle:mot_de_passe@127.0.0.1:5432/hemicycle?sslmode=disable
LLM_PROVIDER=anthropic
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-proj-...
ADMIN_TOKEN_SECRET=<openssl rand -hex 32>
BIND_ADDR=127.0.0.1
PORT=8085
RUST_LOG=info
```

Les clés LLM sont facultatives : sans la clé du fournisseur choisi par
`LLM_PROVIDER`, la thématisation ne propose rien, les synthèses restent en
attente et le site tourne (RM-01). Le fournisseur peut être changé dans ce
fichier, puis le service redémarré. `ADMIN_TOKEN_SECRET` ne l'est pas en pratique : sans
lui, les huit routes d'écriture répondent `403` et l'ingestion devient
impossible, CRON compris. Moins de 32 caractères : refusé au démarrage, écriture
fermée de la même façon.

### 4.6 Port libre

Vérifier avant d'aller plus loin :

```bash
ss -ltnp | grep -E ':(8085|8080)\b'
```

Rien en sortie = les deux ports sont libres. Sinon, changer `PORT` dans `.env`
et les `proxy_pass` des deux vhosts de façon cohérente.

### 4.7 Installation des unités

Le vhost public livré porte le marqueur `__DOMAINE_PUBLIC__` au lieu du domaine :
le dépôt est public, l'adresse de l'instance n'a pas à y figurer. La substitution
se fait à l'installation, jamais dans le dépôt.

```bash
install -m 644 /home/hemicycle/app/deploy/systemd/hemicycle.service /etc/systemd/system/
install -m 644 /home/hemicycle/app/deploy/systemd/hemicycle-ingest.service /etc/systemd/system/
install -m 644 /home/hemicycle/app/deploy/systemd/hemicycle-ingest.timer /etc/systemd/system/
install -m 644 /home/hemicycle/app/deploy/systemd/hemicycle-text-capture.service /etc/systemd/system/
install -m 644 /home/hemicycle/app/deploy/systemd/hemicycle-text-capture.timer /etc/systemd/system/
install -m 644 /home/hemicycle/app/deploy/systemd/hemicycle-candidate-sources.service /etc/systemd/system/
install -m 644 /home/hemicycle/app/deploy/systemd/hemicycle-candidate-sources.timer /etc/systemd/system/
install -m 644 /home/hemicycle/app/deploy/nginx/hemicycle-admin.conf /etc/nginx/sites-available/

# Vhost public : domaine injecté à la volée.
sed 's/__DOMAINE_PUBLIC__/<DOMAINE_PUBLIC>/' \
    /home/hemicycle/app/deploy/nginx/hemicycle-public.conf \
    > /etc/nginx/sites-available/hemicycle-public.conf
chmod 644 /etc/nginx/sites-available/hemicycle-public.conf
grep -q '__DOMAINE_PUBLIC__' /etc/nginx/sites-available/hemicycle-public.conf \
    && { echo 'Substitution ratée : nginx refusera le fichier' >&2; exit 1; }

# Identifiant HTTP Basic des écritures publiques (§3.1). bcrypt (-B) ; le mot
# de passe est demandé au clavier, jamais passé en argument. Lisible par les
# workers nginx (www-data), par personne d'autre.
apt install -y apache2-utils
htpasswd -B -c /etc/nginx/hemicycle-admin.htpasswd <IDENTIFIANT>
chown root:www-data /etc/nginx/hemicycle-admin.htpasswd
chmod 640 /etc/nginx/hemicycle-admin.htpasswd

ln -sf /etc/nginx/sites-available/hemicycle-public.conf /etc/nginx/sites-enabled/
ln -sf /etc/nginx/sites-available/hemicycle-admin.conf /etc/nginx/sites-enabled/
systemctl daemon-reload
systemctl enable hemicycle
# Le timer seul est activé : le service d'ingestion est déclenché par lui,
# jamais au démarrage (§6.2). Ne pas ajouter en plus la ligne de crontab.
systemctl enable --now hemicycle-ingest.timer
# Capture indépendante, quotidienne : elle n'emploie ni le jeton admin ni un LLM.
systemctl enable --now hemicycle-text-capture.timer
# Vérifie les sources des candidatures 2027 et journalise les changements ;
# une revue humaine reste nécessaire avant toute publication (§6.4).
systemctl enable --now hemicycle-candidate-sources.timer
```

Premier build manuel avant le premier démarrage (sinon le binaire n'existe pas) :

```bash
sudo -u hemicycle /home/hemicycle/app/deploy/deploy.sh --skip-restart
systemctl start hemicycle
systemctl status hemicycle
```

### 4.8 TLS

Le vhost public livré écoute en HTTP sur le port 80. Poser d'abord
l'enregistrement DNS `A` de `<DOMAINE_PUBLIC>` vers l'IP du VPS, attendre
sa propagation (`dig +short <DOMAINE_PUBLIC>`).

Méthode **webroot**, pas le plugin `--nginx` en authentificateur : tant que
`www/` est vide, le fallback SPA `try_files … /index.html` renvoie `404` et le
bloc temporaire injecté par le plugin ne prend pas la main — l'émission échoue
avec `Invalid response from …/.well-known/acme-challenge/… : 404`. Le vhost
livré contient un `location ^~ /.well-known/acme-challenge/` permanent qui règle
le problème et sert aussi aux renouvellements.

```bash
mkdir -p /var/www/certbot/.well-known/acme-challenge
```

Vérifier que le chemin est servi avant d'appeler l'autorité :

```bash
echo ok > /var/www/certbot/.well-known/acme-challenge/probe
curl -i http://<DOMAINE_PUBLIC>/.well-known/acme-challenge/probe   # 200 attendu
```

Puis émettre et installer le certificat :

```bash
certbot run -a webroot -w /var/www/certbot -i nginx -d <DOMAINE_PUBLIC> --redirect --agree-tos -m <EMAIL_CERTBOT>
rm /var/www/certbot/.well-known/acme-challenge/probe
```

Certbot réécrit le vhost en place : il ajoute le bloc `listen 443 ssl`, les
chemins de certificat et la redirection 80 → 443. Le renouvellement est assuré
par le timer `certbot.timer` (`systemctl list-timers certbot.timer` pour
vérifier).

**Conséquence sur les mises à jour** : ne jamais réinstaller
`hemicycle-public.conf` par-dessus la version modifiée par Certbot sans
rejouer `certbot --nginx` derrière. Le déploiement automatique ne touche pas aux
fichiers Nginx — seule une intervention manuelle peut casser cela.

## 5. Déploiement automatique

### 5.1 Déclencheur

`push` sur `main` (donc tout merge de pull request), plus `workflow_dispatch`
pour un déclenchement manuel.

### 5.2 Étapes

1. **Job `test`** (runner GitHub `ubuntu-latest`) — `cargo test` (197 tests),
   `oxlint`, puis `npm run build` qui enchaîne `tsc -b` et `vite build`. Aucun
   accès à la base : le code n'utilise aucune macro `sqlx::query!` vérifiée à la
   compilation, tout passe par les variantes runtime, et les tests de use case
   reposent sur des fakes in-memory.

   `cargo fmt --check` et `cargo clippy -- -D warnings` sont **volontairement
   absents du verrou** : à ce jour le code ne les passe pas (13 erreurs clippy,
   formatage non conforme). Les mettre dans le verrou bloquerait tout
   déploiement dès le premier merge. À rétablir une fois la dette résorbée.
2. **Job `deploy`** — s'exécute uniquement si `test` est vert et si la référence
   est `refs/heads/main`. Se connecte en SSH sous `hemicycle` et lance
   `~/app/deploy/deploy.sh <sha>`.

Le job `deploy` déclare `concurrency: hemicycle-production` avec
`cancel-in-progress: false` : deux merges rapprochés se déploient l'un après
l'autre, jamais en parallèle sur le même dossier.

### 5.3 Ce que fait `deploy.sh` sur le VPS

1. `git fetch --prune`, puis `git reset --hard <sha>` sur `app/` — l'état du
   serveur est exactement celui du commit, toute modification locale est écrasée.
2. `cargo build --release --locked --bins`.
3. `npm ci` puis `npm run build` dans `frontend/`.
4. `rsync -a --delete frontend/dist/ ~/www/`.
5. `sudo systemctl restart hemicycle`.
6. Attente active de `GET 127.0.0.1:8085/api/health` (30 tentatives, 1 s). Échec
   du health check = sortie non nulle = job GitHub rouge.

### 5.4 Indisponibilité et retour arrière

Écrasement en place, assumé :

- Le redémarrage du service coupe l'API environ une seconde.
- `rsync --delete` sur `www/` ouvre une fenêtre de quelques centaines de
  millisecondes où un visiteur peut recevoir un `index.html` neuf pointant vers
  des fichiers d'actifs déjà supprimés. Il obtient une page cassée jusqu'au
  rechargement. Acceptable au trafic visé ; c'est le prix de l'écrasement.
- Pas de rollback automatique. Retour arrière manuel :

```bash
ssh hemicycle@<IP_DU_VPS> '~/app/deploy/deploy.sh <sha_precedent>'
```

Une migration déjà appliquée n'est pas défaite par ce retour arrière — voir §5.5.

### 5.5 Migrations

`sqlx::migrate!()` s'exécute au démarrage du binaire
(`src/infrastructure/config.rs`). La table `_sqlx_migrations` tient le journal :
une migration déjà appliquée n'est jamais rejouée, seules les nouvelles passent.

Aucune sauvegarde n'est prise avant. En clair : une migration destructrice part
en production sans filet, et le retour arrière du code ne la défait pas. Les
sauvegardes PostgreSQL et leur restauration sont opérées séparément sur le VPS.
Contrainte de travail à respecter : écrire des migrations additives (ajout de
colonne nullable, nouvelle table), jamais de `DROP` ni de `ALTER` réducteur sans
étape de transition.

### 5.6 Secrets GitHub

| Secret | Contenu |
|---|---|
| `VPS_HOST` | IP ou nom d'hôte du VPS |
| `VPS_SSH_KEY` | clé privée de §4.3.a, contenu intégral y compris les lignes `-----BEGIN/END-----` |
| `VPS_KNOWN_HOSTS` | sortie de `ssh-keyscan <IP_DU_VPS>` |
| `PUBLIC_BASE_URL` | URL publique du site, avec le schéma et sans barre finale (`https://exemple.tld`) |

`PUBLIC_BASE_URL` n'est pas un secret au sens cryptographique : c'est un secret
de dépôt, pour que le domaine de l'instance ne soit pas écrit dans un dépôt
public. Sans lui, l'étape « Vérification publique » interroge `/api/health` et
échoue.

`VPS_KNOWN_HOSTS` n'est pas cosmétique : sans lui, le workflow accepterait
n'importe quelle clé d'hôte et livrerait la clé de déploiement au premier
intercepteur venu. Aucun secret applicatif ne transite par GitHub.

### 5.7 Fichiers livrés

| Fichier | Rôle |
|---|---|
| `.github/workflows/deploy.yml` | verrou de tests + déploiement SSH |
| `deploy/deploy.sh` | script exécuté sur le VPS (build, publication, restart, health check) |
| `deploy/systemd/hemicycle.service` | unité systemd, à installer dans `/etc/systemd/system/` |
| `deploy/systemd/hemicycle-ingest.service` | job d'ingestion (`oneshot`), déclenché par le timer |
| `deploy/systemd/hemicycle-ingest.timer` | cadence de l'ingestion, toutes les 2 h |
| `deploy/cron/hemicycle-ingest.sh` | script appelé par le job : les quatre routes d'ingestion, avec le jeton du jour |
| `deploy/systemd/hemicycle-text-capture.service` | job de capture des textes officiels (`oneshot`) |
| `deploy/systemd/hemicycle-text-capture.timer` | cadence de capture, quotidienne |
| `deploy/cron/hemicycle-text-capture.sh` | synchronise le registre et capture les documents, sans LLM |
| `deploy/systemd/hemicycle-candidate-sources.service` | vérifie les sources déjà publiées des candidatures 2027 |
| `deploy/systemd/hemicycle-candidate-sources.timer` | lance cette vérification chaque jour |
| `deploy/cron/hemicycle-candidate-sources.sh` | compare les réponses HTTP, journalise les changements et les erreurs |
| `deploy/nginx/hemicycle-public.conf` | vhost public, à installer dans `/etc/nginx/sites-available/` |
| `deploy/nginx/hemicycle-admin.conf` | vhost d'administration sur `127.0.0.1:8080` |
| `deploy/bin/admin-token.sh` | affiche le jeton du jour (§3.2) |

Ordre de mise en service : §4 en entier (dont `certbot`, §4.8) **avant** le
premier merge sur `main`. L'étape « Vérification publique » du workflow
interroge `https://<DOMAINE_PUBLIC>` et échoue tant que le certificat
n'est pas posé.

## 6. Exploitation

### 6.1 Commandes

```bash
sudo systemctl status hemicycle          # état
journalctl -u hemicycle -f               # logs en direct
journalctl -u hemicycle --since "1 hour ago" -p err
curl -s 127.0.0.1:8085/api/health        # santé côté serveur
~/app/deploy/bin/admin-token.sh          # jeton d'écriture du jour
tail -f ~/shared/ingest.log              # dernière ingestion CRON
```

Vérification du filtre d'écriture depuis l'extérieur — doit répondre `401` avec
`WWW-Authenticate: Basic` (sans cet en-tête, le `401` vient du backend : nginx
a laissé passer la requête) :

```bash
curl -s -o /dev/null -D - -X POST https://<DOMAINE_PUBLIC>/api/refresh | grep -iE '^(HTTP|www-authenticate)'
```

### 6.2 Ingestion périodique

`hemicycle-ingest.timer` déclenche `deploy/cron/hemicycle-ingest.sh` toutes les
deux heures (00:07, 02:07, 04:07… plus un décalage aléatoire de 5 min au plus).
Le script attend que `/api/health` réponde — un redémarrage laisse les
migrations sqlx et le démarrage du service en cours — puis appelle les quatre
routes d'ingestion avec le jeton du jour (§3.3).

Côté dossiers, `POST /api/refresh` enchaîne référentiel des acteurs, dossiers,
scrutins. Un dossier nouveau est écrit avec le sort dérivé de ses actes ; un
dossier dont la source a bougé est réécrit, sort compris ; un dossier au sort
définitif (promulgation, retrait, fusion) est sauté, plus rien ne peut le
changer.

Deux heures est un rythme choisi pour rester sous le seuil de perceptibilité,
pas pour suivre la source : l'open data de l'Assemblée n'est pas mis à jour à
cette cadence. La plupart des passages ne réécrivent rien et se comptent en
secondes.

Un passage coûte **trois requêtes** — les trois archives ZIP de
`data.assemblee-nationale.fr` — et rien de plus : tout le reste est parsé en
local, il n'y a aucun appel par dossier. Ces trois requêtes sont
conditionnelles (`If-None-Match` / `If-Modified-Since`) : quand l'archive n'a
pas changé, la source répond `304 Not Modified` en quelques octets et la copie
en mémoire est resservie sans être reparsée. Sur une archive republiée une fois
par jour, onze passages sur douze ne téléchargent donc rien. C'est ce qui rend
la cadence de deux heures tenable ; sans ce mécanisme, il faudrait l'espacer.

Le client se nomme auprès de la source
(`QuiVoteQuoi/<version> (+<URL du dépôt>)`). Si l'Assemblée devait un jour
resserrer l'accès, c'est ce qui permet de nous joindre plutôt que de nous
couper.

```bash
systemctl list-timers hemicycle-ingest.timer     # prochaine et dernière passe
journalctl -u hemicycle-ingest -n 50 --no-pager  # résumé du dernier passage
sudo systemctl start hemicycle-ingest.service    # déclenchement immédiat
sudo systemctl disable --now hemicycle-ingest.timer   # suspendre la cadence
```

Les deux dernières sont des commandes root : le `sudoers` de `hemicycle` (§4.2)
ne couvre que le service principal, et le timer n'a besoin d'aucun droit `sudo`
pour tourner — c'est systemd qui le déclenche.

Le script journalise une ligne par route, avec son code HTTP. Une route en échec
n'empêche pas les suivantes de tourner, mais le code de sortie du job la
reflète : l'unité passe en `failed` et `systemctl list-timers` le montre.

### 6.3 Capture quotidienne des textes officiels

`hemicycle-text-capture.timer` déclenche `deploy/cron/hemicycle-text-capture.sh`
chaque jour vers 05:20, après l'ingestion nocturne. Il n'appelle pas le backend
d'administration : il lit `DATABASE_URL`, exécute les deux binaires release et
écrit les captures dans PostgreSQL.

```bash
systemctl list-timers hemicycle-text-capture.timer
journalctl -u hemicycle-text-capture -n 50 --no-pager
sudo systemctl start hemicycle-text-capture.service
sudo systemctl disable --now hemicycle-text-capture.timer
```

Une réécriture complète (`?full=true`), nécessaire après un changement de règle
de dérivation (score, sort, rattachement), reste manuelle et n'a pas sa place
dans le timer. Elle exige le jeton du jour :

```bash
sudo -u hemicycle bash -c 'curl -sS -X POST \
  -H "x-admin-token: $(~/app/deploy/bin/admin-token.sh)" \
  "http://127.0.0.1:8085/api/refresh?full=true"'
```

### 6.4 Vérification quotidienne des sources de programmes 2027

`hemicycle-candidate-sources.timer` lance chaque jour, vers 06:10,
`deploy/cron/hemicycle-candidate-sources.sh`. Le job relit les URL déjà
attribuées aux déclarations, organisations et extraits de programme ; il
réemploie `ETag` et `Last-Modified` lorsque les serveurs les fournissent, puis
compare un SHA-256 de la réponse. Il ne conserve que ces métadonnées techniques
dans PostgreSQL, pas une copie du contenu source.

Un changement est écrit dans le journal avec « revue humaine requise ». Il ne
crée ni candidature ni extrait automatiquement : ces éléments doivent rester
attribués à une source primaire contrôlée conformément à `README.md` §3.3 et
§8.2. Les refus explicites d'automatisation (`401`, `403`, `429`) sont
journalisés mais ne font pas échouer l'unité ; ils demandent une vérification
manuelle de la source. Les autres erreurs de source font échouer l'unité, afin
qu'elles soient visibles dans la supervision.

```bash
systemctl list-timers hemicycle-candidate-sources.timer
journalctl -u hemicycle-candidate-sources -n 100 --no-pager
sudo systemctl start hemicycle-candidate-sources.service
sudo systemctl disable --now hemicycle-candidate-sources.timer
```

## 7. Recette

- [ ] `https://<DOMAINE_PUBLIC>` répond en 200, certificat valide
- [ ] Une route SPA profonde rechargée directement (F5) renvoie l'application, pas un 404
- [ ] `GET /api/health` répond via HTTPS
- [ ] `POST /api/refresh` depuis Internet renvoie `401` avec `WWW-Authenticate: Basic`
- [ ] Le même appel avec l'identifiant HTTP Basic mais sans jeton renvoie `401` sans `WWW-Authenticate`
- [ ] Sur le site public, l'écran d'arbitrage demande l'identifiant puis accepte une écriture avec le jeton du jour
- [ ] Le port `8085` n'est pas joignable depuis l'extérieur (`nc -zv <IP> 8085` échoue)
- [ ] Tunnel SSH ouvert, `http://localhost:8080` sert le site et accepte les POST
- [ ] `systemctl restart hemicycle` remonte le service et rejoue les migrations sans erreur
- [ ] Un merge sur `main` déclenche le workflow et le site sert bien le nouveau code
- [ ] `sudo -l` sous `hemicycle` ne liste que les trois commandes `systemctl`
- [ ] `~/app/deploy/bin/admin-token.sh` affiche 32 caractères hexadécimaux
- [ ] `POST /api/refresh` sans jeton, depuis le tunnel admin, renvoie `401`
- [ ] Le même appel avec le jeton du jour aboutit
- [ ] `deploy/cron/hemicycle-ingest.sh` lancé à la main sort en `0`
- [ ] `deploy/cron/hemicycle-text-capture.sh` lancé à la main sort en `0`
- [ ] `deploy/cron/hemicycle-candidate-sources.sh` lancé à la main sort en `0`
- [ ] `ss -ltnp | grep 8085` montre une écoute sur `127.0.0.1`, pas sur `0.0.0.0`
- [ ] `systemctl list-timers hemicycle-ingest.timer` annonce une prochaine passe à moins de 2 h
- [ ] `systemctl list-timers hemicycle-text-capture.timer` annonce une prochaine capture quotidienne
- [ ] `systemctl list-timers hemicycle-candidate-sources.timer` annonce une prochaine vérification quotidienne des sources 2027
- [ ] `crontab -l` sous `hemicycle` ne contient **pas** de ligne `hemicycle-ingest.sh` (le timer et la crontab s'excluent, §3.3)
- [ ] Après un `systemctl restart hemicycle`, le job attend la reprise au lieu d'échouer
