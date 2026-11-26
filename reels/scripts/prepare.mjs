// Chaîne du Reel du jour (REELS-PLAN-F4, CU-01 → CU-04) : porte horaire → posts
// Buffer du jour → historique publié → retrait des brouillons périmés → API du site →
// Release `reel-<date>` réutilisée, ou rendu puis Release → posts manquants, programmés
// pour 18 h : Buffer les publie seul, l'éditeur peut les corriger ou les retenir d'ici là.
// Dans le même passage, le post Instagram du jour : carrousel des mêmes scènes en
// 4:5 (`scripts/stills.mjs`), images dans la même Release, programmé à la même heure.
// Puis la story d'annonce du post : titre du sujet en 9:16 (compositions `…Story`),
// programmée quelques minutes après, elle renvoie les abonnés au carrousel du profil.
// Toute erreur, comme une période incomplète, ouvre ou commente l'issue `reels`.
//
// Environnement : PUBLIC_BASE_URL, BUFFER_API_KEY, BUFFER_INSTAGRAM_CHANNEL_ID,
// BUFFER_TIKTOK_CHANNEL_ID, GH_TOKEN, GITHUB_REPOSITORY ; REEL_DATE (AAAA-MM-JJ,
// aujourd'hui à Paris par défaut), DRY_RUN=true, MANUAL=true (workflow_dispatch).

import { execFileSync } from 'node:child_process'
import { appendFileSync, existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { bufferClient } from './buffer.mjs'
import { renderPostImages } from './stills.mjs'

const REELS = join(dirname(fileURLToPath(import.meta.url)), '..')
const OUT = join(REELS, 'out')
const CATALOG = join(REELS, '..', 'content', 'nouveautes.json')

/**
 * Premier passage utile à 12 h 17, heure de Paris (Exécution programmée) : la passe
 * d'ingestion de minuit couvre déjà la veille, et les passages suivants, jusqu'à 16 h 17
 * l'hiver, rattrapent un cron que GitHub retarde ou saute avant 18 h.
 */
const GATE_HOUR = 12
/** Statuts Buffer d'un post du jour déjà pris en charge : rien à redéposer. */
const HANDLED_STATUSES = ['draft', 'scheduled', 'sending', 'sent', 'error']
/** Plafond de posts programmés par canal (Plafond de Buffer). */
const SCHEDULED_CAP = 10
const HISTORY_SIZE = 200
const LINK_TOKEN = '[lien]'
const ALERT_LABEL = 'reels'
/** Étiquette Buffer des posts en carrousel, distincte de `reel` : l'historique des sujets ne lit que les Reels. */
const POST_TAG = 'post'
/** Étiquette Buffer de la story d'annonce du post. */
const STORY_TAG = 'story'
/** La story part après le post : il est déjà sur le profil quand elle y renvoie. */
const STORY_DELAY_MINUTES = 5
const CAPTURE_FILE = 'capture.png'
const CAPTURE_WIDTH = 390
const CAPTURE_HEIGHT = 2400

const env = process.env
const dryRun = env.DRY_RUN === 'true'
const manual = env.MANUAL === 'true'
const base = (env.PUBLIC_BASE_URL ?? '').replace(/\/$/, '')
// TikTok reste facultatif tant que son canal n'est pas connecté dans Buffer.
const configuredChannels = [
  { service: 'instagram', label: 'Instagram', id: env.BUFFER_INSTAGRAM_CHANNEL_ID, required: true },
  { service: 'tiktok', label: 'TikTok', id: env.BUFFER_TIKTOK_CHANNEL_ID, required: false },
]
const channels = configuredChannels.filter((channel) => channel.id)

const log = (message) => console.log(message)
let step = 'démarrage'

/* ------------------------------------------------------------ heure de Paris */

const parisDate = (instant) => new Intl.DateTimeFormat('en-CA', { timeZone: 'Europe/Paris' }).format(instant)
const parisHour = (instant) =>
  Number(new Intl.DateTimeFormat('en-GB', { timeZone: 'Europe/Paris', hour: '2-digit', hourCycle: 'h23' }).format(instant))

/** Minuit à Paris du jour donné, en instant UTC. */
function parisMidnight(date) {
  for (const offset of ['+01:00', '+02:00']) {
    const instant = new Date(`${date}T00:00:00${offset}`)
    if (parisDate(instant) === date && parisHour(instant) === 0) return instant
  }
  throw new Error(`Minuit introuvable à Paris pour ${date}`)
}

function nextDay(date) {
  const day = new Date(`${date}T12:00:00Z`)
  day.setUTCDate(day.getUTCDate() + 1)
  return day.toISOString().slice(0, 10)
}

/* ------------------------------------------------------------------ GitHub */

function gh(args, options = {}) {
  return execFileSync('gh', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...options })
}

function openAlert() {
  const found = gh(['issue', 'list', '--label', ALERT_LABEL, '--state', 'open', '--json', 'number', '--jq', '.[0].number // empty'])
  return found.trim() || null
}

/** Ouvre ou commente l'issue `reels` (RM-09, RM-17), comme `freshness.yml`. */
function alert(body) {
  log(`ALERTE : ${body}`)
  gh(['label', 'create', ALERT_LABEL, '--force', '--description', 'Reel du jour non déposé'])
  const existing = openAlert()
  if (existing) gh(['issue', 'comment', existing, '--body', body])
  else gh(['issue', 'create', '--label', ALERT_LABEL, '--title', 'Reel du jour non déposé', '--body', body])
}

function closeAlert(date) {
  const existing = openAlert()
  if (!existing) return
  gh(['issue', 'comment', existing, '--body', `Reel du ${date} préparé : retour à la normale.`])
  gh(['issue', 'close', existing])
}

const releaseTag = (date) => `reel-${date}`

/**
 * Marque de rendu, heure UTC `HHMMSS` du passage, dans le nom de chaque fichier
 * rendu : un nouveau rendu du même jour obtient une adresse neuve. Buffer met en
 * cache vignettes et médias par adresse : un fichier remplacé sous le même nom
 * resterait affiché, voire publié, dans sa version précédente.
 */
const renderStamp = new Date().toISOString().slice(11, 19).replaceAll(':', '')
const videoName = (date) => `reel-${date}-${renderStamp}.mp4`
const VIDEO_NAME = /^reel-(\d{4}-\d{2}-\d{2})(?:-(\d{6}))?\.mp4$/
const POST_IMAGE_NAME = /^post-(\d{4}-\d{2}-\d{2})(?:-(\d{6}))?-(\d{2})\.png$/

/** Fichiers de la Release du jour ; `null` si elle n'existe pas. */
function releaseAssets(date) {
  try {
    return gh(['release', 'view', releaseTag(date), '--json', 'assets', '--jq', '.assets[].name']).split('\n').filter(Boolean)
  } catch {
    return null
  }
}

/**
 * Fichiers du jour d'un rendu complet, le plus récent : `[{ name, stamp, index }]`.
 * Un nom sans marque (Releases antérieures) compte comme le rendu le plus ancien.
 */
function latestRender(date, pattern) {
  const matches = (releaseAssets(date) ?? [])
    .map((name) => ({ name, match: pattern.exec(name) }))
    .filter(({ match }) => match?.[1] === date)
    .map(({ name, match }) => ({ name, stamp: match[2] ?? '', index: match[3] ?? '' }))
  const stamp = matches.map((file) => file.stamp).sort().at(-1)
  return matches.filter((file) => file.stamp === stamp).sort((a, b) => a.index.localeCompare(b.index))
}

const STORY_IMAGE_NAME = /^story-(\d{4}-\d{2}-\d{2})(?:-(\d{6}))?\.png$/
const storyImageName = (date) => `story-${date}-${renderStamp}.png`
const postImageName = (date, index) => `post-${date}-${renderStamp}-${String(index + 1).padStart(2, '0')}.png`
const assetUrl = (date, name) => `https://github.com/${env.GITHUB_REPOSITORY}/releases/download/${releaseTag(date)}/${name}`

/** Ajoute des fichiers à la Release du jour, créée au besoin. */
function uploadToRelease(date, files, notes) {
  if (releaseAssets(date) === null) {
    gh(['release', 'create', releaseTag(date), ...files, '--title', `Reel du ${date}`, '--notes', notes, '--latest=false'])
  } else {
    gh(['release', 'upload', releaseTag(date), ...files, '--clobber'])
  }
}

/* -------------------------------------------------------------------- site */

async function api(path) {
  const response = await fetch(`${base}${path}`, { signal: AbortSignal.timeout(60_000) })
  if (!response.ok) throw new Error(`GET ${path} : ${response.status}`)
  return response.json()
}

/** Identifiants des nouveautés encore au catalogue (Nouveautés présentées encore au catalogue). */
function catalogIds() {
  return new Set(JSON.parse(readFileSync(CATALOG, 'utf8')).map((news) => news.id))
}

/** Dernier thème et nouveautés présentés, lus sur les posts publiés (RM-07, RM-08). */
function presentedSubjects(published) {
  const catalog = catalogIds()
  const names = (post) => post.tags.map((tag) => tag.name)
  const lastTheme = published
    .flatMap(names)
    .find((name) => name.startsWith('reel-theme-'))
    ?.slice('reel-theme-'.length)
  const news = new Set(
    published
      .flatMap(names)
      .filter((name) => name.startsWith('reel-nouveaute-'))
      .map((name) => name.slice('reel-nouveaute-'.length))
      .filter((id) => catalog.has(id)),
  )
  return { lastTheme: lastTheme ?? null, news: [...news] }
}

function compositionOf(reel) {
  if (reel.theme) return 'Theme'
  if (reel.site_news) return 'SiteNews'
  if (reel.programmes.length > 0) return 'Programmes'
  return 'Votes'
}

/** Détail publié de chaque scrutin montré (RM-03, RM-10). */
async function scrutinDetails(reel) {
  const uids = new Set(reel.final_votes.map((vote) => vote.scrutin_uid))
  if (reel.most_followed) uids.add(reel.most_followed.uid)
  const details = {}
  for (const uid of uids) details[uid] = { ...(await api(`/api/scrutins/${uid}`)), explanations: null }
  return details
}

/**
 * Capture de la page de la nouveauté en production (RM-08), sur la largeur d'un
 * téléphone et sur toute sa hauteur jusqu'à `CAPTURE_HEIGHT` : le Reel la fait
 * défiler. Les éléments fixes (barre de navigation) sont masqués, sinon ils
 * resteraient au milieu de la page capturée.
 */
async function captureNews(path) {
  if (process.platform === 'linux') {
    execFileSync('npx', ['playwright', 'install', '--with-deps', 'chromium'], { cwd: REELS, stdio: 'inherit' })
  }
  const { chromium } = await import('playwright')
  const browser = await chromium.launch()
  try {
    const page = await browser.newPage({ viewport: { width: CAPTURE_WIDTH, height: 844 }, deviceScaleFactor: 2 })
    await page.goto(`${base}${path}`, { waitUntil: 'networkidle' })
    await page.evaluate(() => {
      for (const element of document.querySelectorAll('*')) {
        if (getComputedStyle(element).position === 'fixed') element.style.display = 'none'
      }
    })
    mkdirSync(join(REELS, 'public'), { recursive: true })
    await page.screenshot({
      path: join(REELS, 'public', CAPTURE_FILE),
      fullPage: true,
      clip: { x: 0, y: 0, width: CAPTURE_WIDTH, height: CAPTURE_HEIGHT },
    })
  } finally {
    await browser.close()
  }
  return CAPTURE_FILE
}

/** Propriétés communes au Reel et au carrousel, lues une fois sur l'API. */
async function propsOf(reel) {
  const composition = compositionOf(reel)
  const themeLabels =
    reel.programmes.length > 0
      ? Object.fromEntries((await api('/api/themes')).families.map((family) => [family.code, family.label]))
      : {}
  const props = {
    reel,
    details: composition === 'Votes' ? await scrutinDetails(reel) : {},
    themeLabels,
    site: new URL(base).host,
    capture: reel.site_news ? await captureNews(reel.site_news.path) : null,
    audio: existsSync(join(REELS, 'public', 'piste.mp3')),
  }
  return props
}

async function render(date, reel, props) {
  const composition = compositionOf(reel)
  const propsFile = join(OUT, `props-${date}.json`)
  writeFileSync(propsFile, JSON.stringify(props))
  const output = join(OUT, videoName(date))
  log(`Rendu ${composition} → ${output}`)
  execFileSync('npx', ['remotion', 'render', 'src/index.ts', composition, output, `--props=${propsFile}`], {
    cwd: REELS,
    stdio: 'inherit',
  })
  return output
}

/** Légende déposée : celle de l'API, lien complété, mots-dièse ajoutés (RM-12). */
function captionOf(reel) {
  const link = `${base}${reel.link_path ?? ''}`
  const hashtags = reel.tags.filter((tag) => tag.startsWith('#')).join(' ')
  return `${(reel.caption ?? '').replace(LINK_TOKEN, link)}\n\n${hashtags}`.trim()
}

/** Étiquettes Buffer : `reel`, `reel-<kind>` et les étiquettes de sujet de l'API. */
const bufferTagsOf = (reel) => ['reel', `reel-${reel.kind}`, ...reel.tags.filter((tag) => !tag.startsWith('#'))]

/** Étiquettes du post : `post` et `post-<kind>` seulement, pour ne pas compter le sujet deux fois. */
const postTagsOf = (reel) => [POST_TAG, `${POST_TAG}-${reel.kind}`]

/** Carrousel du post : images déjà dans la Release, sinon rendues puis ajoutées. */
async function postImages(date, reel, props) {
  const existing = latestRender(date, POST_IMAGE_NAME)
  if (!dryRun && existing.length > 0) {
    log(`Images du post déjà dans la Release ${releaseTag(date)} : réutilisées.`)
    return existing.map((file) => assetUrl(date, file.name))
  }
  const dir = join(OUT, `post-${date}`)
  const rendered = await renderPostImages(compositionOf(reel), props, dir)
  const files = rendered.map((file, index) => {
    const named = join(dir, postImageName(date, index))
    renameSync(file, named)
    return named
  })
  log(`Post : ${files.length} images rendues.`)
  if (dryRun) {
    log(`Images du post : Release sautée (exécution à blanc), images en artefact du run.`)
    return []
  }
  uploadToRelease(date, files, reel.period.label)
  return files.map((_, index) => assetUrl(date, postImageName(date, index)))
}

/** Étiquettes de la story : `story` et `story-<kind>`. */
const storyTagsOf = (reel) => [STORY_TAG, `${STORY_TAG}-${reel.kind}`]

/** Image de la story : déjà dans la Release, sinon rendue (composition `<Sujet>Story`) puis ajoutée. */
function storyImage(date, reel, props) {
  const existing = latestRender(date, STORY_IMAGE_NAME)[0]
  if (!dryRun && existing) {
    log(`Image de la story déjà dans la Release ${releaseTag(date)} : réutilisée.`)
    return assetUrl(date, existing.name)
  }
  const propsFile = join(OUT, `props-${date}.json`)
  writeFileSync(propsFile, JSON.stringify(props))
  const output = join(OUT, storyImageName(date))
  execFileSync('npx', ['remotion', 'still', 'src/index.ts', `${compositionOf(reel)}Story`, output, `--props=${propsFile}`], { cwd: REELS, stdio: 'inherit' })
  log(`Story : image rendue → ${output}`)
  if (dryRun) {
    log(`Image de la story : Release sautée (exécution à blanc), image en artefact du run.`)
    return null
  }
  uploadToRelease(date, [output], reel.period.label)
  return assetUrl(date, storyImageName(date))
}

function journal(date, reel) {
  const line = `Reel du ${date} : statut ${reel.status}${reel.reason ? ` (${reel.reason})` : ''}, période ${reel.period.label}, dernière synchronisation des scrutins ${reel.last_success_at ?? 'aucune'}`
  log(line)
  if (env.GITHUB_STEP_SUMMARY) appendFileSync(env.GITHUB_STEP_SUMMARY, `${line}\n`)
}

/* --------------------------------------------------------------------- run */

async function run() {
  const now = new Date()
  const date = env.REEL_DATE || parisDate(now)
  if (!/^\d{4}-\d{2}-\d{2}$/.test(date)) throw new Error(`Date invalide : ${date}`)
  if (!base) throw new Error('PUBLIC_BASE_URL absent')
  for (const channel of configuredChannels) {
    if (channel.id) continue
    if (channel.required) throw new Error(`Canal Buffer ${channel.label} absent`)
    log(`Canal Buffer ${channel.label} absent : sauté.`)
  }
  log(`Reel du ${date}${dryRun ? ' (exécution à blanc)' : ''}${manual ? ', lancé à la main' : ''}`)

  step = 'porte horaire'
  if (!manual && parisHour(now) < GATE_HOUR) {
    log(`Il est ${parisHour(now)} h à Paris, avant ${GATE_HOUR} h : rien à faire.`)
    return
  }

  step = 'lecture des posts Buffer du jour'
  const buffer = bufferClient(env.BUFFER_API_KEY)
  const organizationId = await buffer.organizationId()
  const knownTags = await buffer.tags(organizationId)
  const reelTag = knownTags.get('reel')
  const channelIds = channels.map((channel) => channel.id)
  const dayStart = parisMidnight(date)
  const dayEnd = new Date(parisMidnight(nextDay(date)).getTime() - 1)
  const todayReels = reelTag
    ? await buffer.posts(organizationId, {
        channelIds,
        status: HANDLED_STATUSES,
        tagIds: [reelTag],
        dueAt: { start: dayStart.toISOString(), end: dayEnd.toISOString() },
      })
    : []
  const missing = channels.filter((channel) => !todayReels.some((post) => post.channelId === channel.id))
  const instagram = channels.find((channel) => channel.service === 'instagram')
  const postTag = knownTags.get(POST_TAG)
  const todayPosts = postTag
    ? await buffer.posts(organizationId, {
        channelIds: [instagram.id],
        status: HANDLED_STATUSES,
        tagIds: [postTag],
        dueAt: { start: dayStart.toISOString(), end: dayEnd.toISOString() },
      })
    : []
  const postMissing = todayPosts.length === 0
  const storyTag = knownTags.get(STORY_TAG)
  const todayStories = storyTag
    ? await buffer.posts(organizationId, {
        channelIds: [instagram.id],
        status: HANDLED_STATUSES,
        tagIds: [storyTag],
        dueAt: { start: dayStart.toISOString(), end: dayEnd.toISOString() },
      })
    : []
  const storyMissing = todayStories.length === 0
  if (missing.length === 0 && !postMissing && !storyMissing) {
    log('Les posts du jour sont déjà déposés.')
    closeAlert(date)
    return
  }

  step = "lecture de l'historique publié"
  const published = reelTag
    ? await buffer.posts(organizationId, { channelIds, status: ['sent'], tagIds: [reelTag] }, HISTORY_SIZE)
    : []
  const presented = presentedSubjects(published)
  log(`Dernier thème présenté : ${presented.lastTheme ?? 'aucun'} ; nouveautés présentées : ${presented.news.join(', ') || 'aucune'}`)

  step = 'retrait des brouillons périmés'
  const staleTags = [reelTag, postTag].filter(Boolean)
  const stale =
    staleTags.length > 0
      ? await buffer.posts(organizationId, {
          channelIds,
          status: ['draft'],
          tagIds: staleTags,
          dueAt: { end: new Date(dayStart.getTime() - 1).toISOString() },
        })
      : []
  for (const post of stale) {
    if (dryRun) log(`Brouillon périmé ${post.id} (${post.dueAt}) : retrait sauté (exécution à blanc)`)
    else {
      await buffer.deletePost(post.id)
      log(`Brouillon périmé ${post.id} (${post.dueAt}) retiré (RM-18)`)
    }
  }

  step = "lecture du Reel sur l'API"
  const query = new URLSearchParams()
  if (presented.lastTheme) query.set('dernier_theme', presented.lastTheme)
  if (presented.news.length > 0) query.set('nouveautes_presentees', presented.news.join(','))
  const reel = await api(`/api/reels/${date}${query.size > 0 ? `?${query}` : ''}`)
  journal(date, reel)
  if (reel.status === 'skipped') {
    log('Aucun Reel prévu ce jour (RM-02).')
    return
  }
  if (reel.status === 'incomplete') {
    alert(
      `Reel du ${date} non préparé : les votes de la période « ${reel.period.label} » sont incomplets (${reel.reason}). Dernière synchronisation des scrutins : ${reel.last_success_at ?? 'aucune'} (RM-09).`,
    )
    return
  }

  mkdirSync(OUT, { recursive: true })
  const caption = captionOf(reel)
  writeFileSync(join(OUT, `caption-${date}.txt`), caption)

  step = 'propriétés du rendu'
  const props = await propsOf(reel)

  step = 'vidéo'
  const existingVideo = latestRender(date, VIDEO_NAME)[0]
  let videoUrl = null
  if (missing.length === 0) {
    log('Reel du jour déjà déposé : vidéo non rendue.')
  } else if (!dryRun && existingVideo) {
    log(`Release ${releaseTag(date)} existante : ${existingVideo.name} réutilisée sans nouveau rendu.`)
    videoUrl = assetUrl(date, existingVideo.name)
  } else {
    const output = await render(date, reel, props)
    if (dryRun) log(`Release ${releaseTag(date)} : sautée (exécution à blanc), vidéo en artefact du run.`)
    else {
      step = 'Release'
      uploadToRelease(date, [output], reel.period.label)
      log(`Vidéo ${videoName(date)} ajoutée à la Release ${releaseTag(date)}.`)
      videoUrl = assetUrl(date, videoName(date))
    }
  }

  step = 'images du post'
  const imageUrls = postMissing ? await postImages(date, reel, props) : []

  step = 'image de la story'
  const storyUrl = storyMissing ? storyImage(date, reel, props) : null

  step = 'plafond de Buffer'
  for (const channel of channels) {
    const needed =
      (missing.includes(channel) ? 1 : 0) + (channel === instagram ? (postMissing ? 1 : 0) + (storyMissing ? 1 : 0) : 0)
    if (needed === 0) continue
    const scheduled = await buffer.posts(organizationId, { channelIds: [channel.id], status: ['scheduled'] }, SCHEDULED_CAP)
    if (scheduled.length + needed > SCHEDULED_CAP) {
      throw new Error(`plafond de ${SCHEDULED_CAP} posts programmés atteint sur ${channel.label}, aucun post déposé`)
    }
  }

  step = 'dépôt des posts'
  // Heure de 18 h déjà passée (lancement à la main en soirée) : un post programmé
  // partirait aussitôt sans relecture ; il reste un brouillon que l'éditeur publie.
  const dueAt = new Date(reel.due_at).toISOString()
  const draft = new Date(reel.due_at).getTime() <= Date.now()
  const deposited = draft ? 'Brouillon' : 'Programmé'
  const tagIds = dryRun ? [] : await buffer.ensureTags(organizationId, bufferTagsOf(reel), knownTags)
  for (const channel of missing) {
    if (dryRun) {
      log(`Reel ${channel.label} pour ${reel.due_at} : sauté (exécution à blanc), étiquettes ${bufferTagsOf(reel).join(', ')}`)
      continue
    }
    const id = await buffer.schedulePost({
      channelId: channel.id,
      service: channel.service,
      text: caption,
      videoUrl,
      dueAt,
      tagIds,
      draft,
    })
    log(`${deposited} : Reel ${channel.label} pour ${reel.due_at}, ${id}`)
  }

  step = 'dépôt du post'
  if (postMissing) {
    if (dryRun) log(`Post Instagram pour ${reel.due_at} : sauté (exécution à blanc), étiquettes ${postTagsOf(reel).join(', ')}`)
    else {
      const postTagIds = await buffer.ensureTags(organizationId, postTagsOf(reel), knownTags)
      const id = await buffer.schedulePost({
        channelId: instagram.id,
        service: 'instagram',
        text: caption,
        imageUrls,
        dueAt,
        tagIds: postTagIds,
        draft,
      })
      log(`${deposited} : post Instagram pour ${reel.due_at}, ${id} (${imageUrls.length} images)`)
    }
  }

  step = 'dépôt de la story'
  if (storyMissing) {
    const storyDueAt = new Date(new Date(reel.due_at).getTime() + STORY_DELAY_MINUTES * 60_000).toISOString()
    if (dryRun) log(`Story Instagram pour ${storyDueAt} : sautée (exécution à blanc), étiquettes ${storyTagsOf(reel).join(', ')}`)
    else {
      const storyTagIds = await buffer.ensureTags(organizationId, storyTagsOf(reel), knownTags)
      const id = await buffer.schedulePost({
        channelId: instagram.id,
        service: 'instagram',
        text: 'Nouveau post',
        imageUrls: [storyUrl],
        dueAt: storyDueAt,
        tagIds: storyTagIds,
        draft,
        story: true,
      })
      log(`${deposited} : story Instagram pour ${storyDueAt}, ${id}`)
    }
  }
  closeAlert(date)
}

try {
  await run()
} catch (error) {
  const message = error instanceof Error ? error.message : String(error)
  const date = env.REEL_DATE || parisDate(new Date())
  try {
    alert(`Reel du ${date} non déposé, étape « ${step} » : ${message} (RM-17).`)
  } catch (alertError) {
    console.error(`Alerte impossible : ${alertError instanceof Error ? alertError.message : alertError}`)
  }
  console.error(error)
  process.exit(1)
}
