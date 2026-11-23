// Chaîne du Reel du jour (REELS-PLAN-F4, CU-01 → CU-04) : porte horaire → brouillons
// Buffer du jour → historique publié → retrait des brouillons périmés → API du site →
// Release `reel-<date>` réutilisée, ou rendu puis Release → brouillons manquants.
// Toute erreur, comme une période incomplète, ouvre ou commente l'issue `reels`.
//
// Environnement : PUBLIC_BASE_URL, BUFFER_API_KEY, BUFFER_INSTAGRAM_CHANNEL_ID,
// BUFFER_TIKTOK_CHANNEL_ID, GH_TOKEN, GITHUB_REPOSITORY ; REEL_DATE (AAAA-MM-JJ,
// aujourd'hui à Paris par défaut), DRY_RUN=true, MANUAL=true (workflow_dispatch).

import { execFileSync } from 'node:child_process'
import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { bufferClient } from './buffer.mjs'

const REELS = join(dirname(fileURLToPath(import.meta.url)), '..')
const OUT = join(REELS, 'out')
const CATALOG = join(REELS, '..', 'content', 'nouveautes.json')

/** Premier passage à 16 h 30, heure de Paris (Exécution programmée). */
const GATE_HOUR = 16
/** Plafond de posts programmés par canal (Plafond de Buffer). */
const SCHEDULED_CAP = 10
const HISTORY_SIZE = 200
const LINK_TOKEN = '[lien]'
const ALERT_LABEL = 'reels'
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
const videoName = (date) => `reel-${date}.mp4`
const videoUrl = (date) =>
  `https://github.com/${env.GITHUB_REPOSITORY}/releases/download/${releaseTag(date)}/${videoName(date)}`

function releaseHasVideo(date) {
  try {
    const names = gh(['release', 'view', releaseTag(date), '--json', 'assets', '--jq', '.assets[].name'])
    return names.split('\n').includes(videoName(date))
  } catch {
    return false
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

async function render(date, reel) {
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

  step = 'lecture des brouillons Buffer'
  const buffer = bufferClient(env.BUFFER_API_KEY)
  const organizationId = await buffer.organizationId()
  const knownTags = await buffer.tags(organizationId)
  const reelTag = knownTags.get('reel')
  const channelIds = channels.map((channel) => channel.id)
  const dayStart = parisMidnight(date)
  const dayEnd = new Date(parisMidnight(nextDay(date)).getTime() - 1)
  const todayDrafts = reelTag
    ? await buffer.posts(organizationId, {
        channelIds,
        status: ['draft'],
        tagIds: [reelTag],
        dueAt: { start: dayStart.toISOString(), end: dayEnd.toISOString() },
      })
    : []
  const missing = channels.filter((channel) => !todayDrafts.some((post) => post.channelId === channel.id))
  if (missing.length === 0) {
    log('Les brouillons du jour sont déjà déposés.')
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
  const stale = reelTag
    ? await buffer.posts(organizationId, {
        channelIds,
        status: ['draft'],
        tagIds: [reelTag],
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

  step = 'vidéo'
  if (!dryRun && releaseHasVideo(date)) {
    log(`Release ${releaseTag(date)} existante : réutilisée sans nouveau rendu.`)
  } else {
    const output = await render(date, reel)
    if (dryRun) log(`Release ${releaseTag(date)} : sautée (exécution à blanc), vidéo en artefact du run.`)
    else {
      step = 'Release'
      gh(['release', 'create', releaseTag(date), output, '--title', `Reel du ${date}`, '--notes', reel.period.label, '--latest=false'])
      log(`Release ${releaseTag(date)} créée.`)
    }
  }

  step = 'plafond de Buffer'
  for (const channel of missing) {
    const scheduled = await buffer.posts(organizationId, { channelIds: [channel.id], status: ['scheduled'] }, SCHEDULED_CAP)
    if (scheduled.length >= SCHEDULED_CAP) {
      throw new Error(`plafond de ${SCHEDULED_CAP} posts programmés atteint sur ${channel.label}, aucun brouillon déposé`)
    }
  }

  step = 'dépôt des brouillons'
  const tagIds = dryRun ? [] : await buffer.ensureTags(organizationId, bufferTagsOf(reel), knownTags)
  for (const channel of missing) {
    if (dryRun) {
      log(`Brouillon ${channel.label} pour ${reel.due_at} : sauté (exécution à blanc), étiquettes ${bufferTagsOf(reel).join(', ')}`)
      continue
    }
    const id = await buffer.createDraft({
      channelId: channel.id,
      service: channel.service,
      text: caption,
      videoUrl: videoUrl(date),
      dueAt: new Date(reel.due_at).toISOString(),
      tagIds,
    })
    log(`Brouillon ${channel.label} déposé : ${id}`)
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
