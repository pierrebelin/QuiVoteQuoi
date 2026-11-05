#!/usr/bin/env node

/**
 * Revérifie chaque programme déclaré contre le web (README.md §8.2) :
 * chaque extrait doit figurer mot pour mot dans le texte visible de sa source,
 * en ignorant seulement la casse, les accents, la ponctuation et les espaces ;
 * chaque document externe doit être lié depuis la page indiquée.
 *
 * Un contenu rendu seulement en JavaScript apparaît absent : à vérifier au
 * navigateur avant de retirer l'extrait. Une source injoignable n'est pas une
 * absence, elle est signalée à part. Les requêtes restent une à une : certains
 * sites bannissent les rafales.
 *
 * Requiert pdftotext (poppler).
 *
 * Usage :
 *   node scripts/check-candidate-sources.mjs [identifiant-candidat...]
 *
 * Code de sortie 0 si tout est retrouvé, 1 sinon.
 */
import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

const USER_AGENT = 'QuiVoteQuoi-check-sources/1.0'
const TIMEOUT_MS = 60_000
const PAUSE_MS = 500

/** Les symboles porteurs de sens (€, %, $, +, °) restent significatifs. */
export function normalize(text) {
  return text
    .normalize('NFKD')
    .replace(/\p{M}/gu, '')
    .toLowerCase()
    .replace(/œ/g, 'oe')
    .replace(/æ/g, 'ae')
    .replace(/[^\p{L}\p{N}€%$+°]/gu, '')
}

function decodeEntities(text) {
  return text
    .replace(/&#(\d+);/g, (_, code) => String.fromCodePoint(Number(code)))
    .replace(/&#x([0-9a-f]+);/gi, (_, code) => String.fromCodePoint(parseInt(code, 16)))
    .replace(/&([a-z])(acute|grave|circ|uml|cedil|tilde);/gi, '$1')
    .replace(/&oelig;/gi, 'oe')
    .replace(/&euro;/gi, '€')
    .replace(/&amp;/gi, '&')
    .replace(/&[a-z0-9]+;/gi, ' ')
}

function visibleHtmlText(html) {
  const visible = html
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/<(head|script|style|noscript|template|svg)\b[\s\S]*?<\/\1\s*>/gi, ' ')
    .replace(/<[^>]*>/g, ' ')
  return decodeEntities(visible)
}

function pdfText(bytes) {
  const directory = mkdtempSync(join(tmpdir(), 'candidate-source-'))
  try {
    const path = join(directory, 'source.pdf')
    writeFileSync(path, bytes)
    const result = spawnSync('pdftotext', ['-enc', 'UTF-8', path, '-'], { encoding: 'utf8', maxBuffer: 1 << 28 })
    if (result.error) throw new Error(`pdftotext est requis : ${result.error.message}`)
    // Un numéro de page seul sur sa ligne coupe une phrase à cheval sur deux pages.
    return result.stdout.replace(/^\s*\d{1,4}\s*$/gm, '')
  } finally {
    rmSync(directory, { recursive: true, force: true })
  }
}

/** Un Google Doc se lit par son export texte, pas par sa visionneuse. */
function downloadUrl(url) {
  const googleDoc = url.match(/docs\.google\.com\/document\/d\/([\w-]+)/)
  return googleDoc ? `https://docs.google.com/document/d/${googleDoc[1]}/export?format=txt` : url
}

async function read(url) {
  const response = await fetch(downloadUrl(url), {
    headers: { 'user-agent': USER_AGENT, 'accept-language': 'fr' },
    signal: AbortSignal.timeout(TIMEOUT_MS),
  })
  if (!response.ok) return { error: `HTTP ${response.status}` }
  const bytes = Buffer.from(await response.arrayBuffer())
  if (bytes.subarray(0, 5).toString('latin1') === '%PDF-') return { text: pdfText(bytes), raw: '' }
  const body = bytes.toString('utf8')
  return { text: /html/.test(response.headers.get('content-type') ?? '') ? visibleHtmlText(body) : body, raw: body }
}

const seed = spawnSync(process.execPath, [new URL('./seed-candidates-2027.mjs', import.meta.url).pathname, '--json'], {
  encoding: 'utf8',
  maxBuffer: 1 << 28,
})
if (seed.status !== 0) {
  process.stderr.write(seed.stderr)
  process.exit(1)
}

const requested = process.argv.slice(2)
const candidates = JSON.parse(seed.stdout).filter((candidate) => requested.length === 0 || requested.includes(candidate.id))

const sources = new Map()
async function source(url) {
  const key = url.split('#')[0]
  if (!sources.has(key)) {
    await new Promise((resolve) => setTimeout(resolve, PAUSE_MS))
    sources.set(key, await read(key).catch((error) => ({ error: error.cause?.code ?? error.message })))
  }
  return sources.get(key)
}

let found = 0
const absent = []
const unreachable = new Map()
const flagUnreachable = (candidate, url, error, count = 1) => {
  const key = `${candidate.id} : ${url} (${error})`
  unreachable.set(key, (unreachable.get(key) ?? 0) + count)
}

for (const candidate of candidates) {
  const before = absent.length
  let candidateFound = 0
  for (const proposal of candidate.proposals) {
    const page = await source(proposal.sourceUrl)
    if (page.error) flagUnreachable(candidate, proposal.sourceUrl, page.error)
    else if (normalize(page.text).includes(normalize(proposal.excerpt))) candidateFound += 1
    else absent.push(`${candidate.id} : « ${proposal.excerpt} » absent de ${proposal.sourceUrl}`)
  }
  for (const document of candidate.officialDocuments ?? []) {
    const page = await source(document.linkedFrom)
    if (page.error) flagUnreachable(candidate, document.linkedFrom, page.error)
    else if (!page.raw.replace(/&amp;/g, '&').includes(document.url)) absent.push(`${candidate.id} : ${document.url} non lié depuis ${document.linkedFrom}`)
  }
  if (candidate.programUrl) {
    const page = await source(candidate.programUrl)
    if (page.error) flagUnreachable(candidate, candidate.programUrl, page.error, 0)
  }
  found += candidateFound
  process.stdout.write(`${candidate.id} : ${candidateFound}/${candidate.proposals.length} retrouvés, ${absent.length - before} absents\n`)
}

process.stdout.write(`\n${found} extraits retrouvés, ${absent.length} anomalies, ${unreachable.size} sources injoignables.\n`)
for (const line of absent) process.stdout.write(`ABSENT ${line}\n`)
for (const [line, count] of unreachable) process.stdout.write(`INJOIGNABLE ${line}${count ? ` — ${count} extraits non vérifiés` : ''}\n`)
process.exit(absent.length > 0 || unreachable.size > 0 ? 1 : 0)
