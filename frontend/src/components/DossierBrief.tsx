import type { ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { useSearchParams } from 'react-router'
import { Card, ErrorPanel, Meta, Note, Pill } from './ui'
import { fetchJson } from '../lib/fetchJson'
import type {
  ArticleBriefDto,
  DossierBriefResponse,
  TextVersionDto,
} from '../types/textBrief'

/** Au-delà, le texte s'affiche replié par division de premier niveau. */
const FOLD_ABOVE = 20

const DIVISION_SEPARATOR = ' > '

function formatDate(iso: string) {
  return new Date(iso.slice(0, 10) + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'long',
    year: 'numeric',
  })
}

/** Premier niveau d'une division : « TITRE Ier — … » dans « TITRE Ier — … > Chapitre II — … ». */
function topLevel(division: string | null) {
  return division === null ? null : division.split(DIVISION_SEPARATOR)[0]
}

/** Ce qui suit le premier niveau : le sous-titre d'une division repliée. */
function belowTopLevel(division: string | null) {
  if (division === null) return null
  const rest = division.split(DIVISION_SEPARATOR).slice(1).join(DIVISION_SEPARATOR)
  return rest === '' ? null : rest
}

/** Suites d'articles consécutifs qui ont la même clé, dans l'ordre du texte. */
function runsBy(
  articles: ArticleBriefDto[],
  key: (article: ArticleBriefDto) => string | null,
) {
  const runs: { key: string | null; articles: ArticleBriefDto[] }[] = []
  for (const article of articles) {
    const current = runs[runs.length - 1]
    if (current && current.key === key(article)) current.articles.push(article)
    else runs.push({ key: key(article), articles: [article] })
  }
  return runs
}

/** Un titre que le texte officiel écrit en capitales se lit en minuscules. */
function readableDivision(raw: string) {
  if (raw !== raw.toUpperCase()) return raw
  return raw
    .toLowerCase()
    .replace(/\s*:\s*/, ' — ')
    .replace(/\b(i{1,3}|iv|vi{0,3})\b/g, (roman) => roman.toUpperCase())
    .replace(/(^|— )(\p{L})/gu, (_, before: string, letter: string) => before + letter.toUpperCase())
}

function shortName(article: ArticleBriefDto) {
  return article.label === 'liminaire' ? 'Lim.' : `Art. ${article.label}`
}

function fullName(article: ArticleBriefDto) {
  return article.label === 'liminaire' ? 'Article liminaire' : `Article ${article.label}`
}

function waitingWord(article: ArticleBriefDto) {
  return article.state === 'pending' ? 'résumé en attente' : 'résumé indisponible'
}

function OfficialLink({ officialUrl, children }: { officialUrl: string; children: ReactNode }) {
  return (
    <a href={officialUrl} target="_blank" rel="noreferrer" className="text-accent hover:underline">
      {children}
    </a>
  )
}

function Chevron({ openedBy }: { openedBy: string }) {
  return (
    <span aria-hidden className={`w-3 shrink-0 text-center text-ink-faint transition-transform ${openedBy}`}>
      ›
    </span>
  )
}

function ArticleLine({
  article,
  officialUrl,
}: {
  article: ArticleBriefDto
  officialUrl: string | null
}) {
  return (
    <li className="grid grid-cols-[3.25rem_minmax(0,1fr)] gap-2.5 py-2 text-sm leading-relaxed">
      <span className="text-[13px] font-semibold text-ink-soft" title={fullName(article)}>
        {shortName(article)}
      </span>
      {article.state === 'ready' ? (
        <span className="text-ink">{article.line}</span>
      ) : (
        <span className="text-[13.5px] text-ink-faint">
          {waitingWord(article)}
          {officialUrl && (
            <>
              {' · '}
              <OfficialLink officialUrl={officialUrl}>Texte officiel ↗</OfficialLink>
            </>
          )}
        </span>
      )}
    </li>
  )
}

/** Des articles sans ligne qui se suivent, en une rangée : leurs numéros restent dépliables (CU-03). */
function WaitingArticles({
  articles,
  officialUrl,
}: {
  articles: ArticleBriefDto[]
  officialUrl: string | null
}) {
  const pending = articles.filter((article) => article.state === 'pending').length
  const unavailable = articles.length - pending
  const counts = [
    pending > 0 ? `${pending} en attente` : null,
    unavailable > 0 ? `${unavailable} ${unavailable > 1 ? 'indisponibles' : 'indisponible'}` : null,
  ].filter((count) => count !== null)
  const link = articles.length > 2 ? 'à' : 'et'
  return (
    <details className="group/waiting my-1.5 rounded-lg bg-surface-soft text-[13.5px] text-ink-soft">
      <summary className="flex cursor-pointer list-none flex-wrap items-center gap-x-2 gap-y-0.5 px-2.5 py-2 [&::-webkit-details-marker]:hidden">
        <Chevron openedBy="group-open/waiting:rotate-90" />
        <span className="font-semibold">
          Articles {articles[0].label} {link} {articles[articles.length - 1].label}
        </span>
        <span className="ml-auto">{counts.join(', ')}</span>
      </summary>
      <ul className="flex flex-wrap gap-1 px-2.5 pb-2 text-[12.5px]">
        {articles.map((article, index) => (
          <li
            key={`${article.label}-${index}`}
            title={`${fullName(article)} : ${waitingWord(article)}`}
            className={`rounded-md bg-surface px-1.5 py-px ${article.state === 'pending' ? '' : 'text-ink-faint line-through'}`}
          >
            {article.label}
            <span className="sr-only"> : {waitingWord(article)}</span>
          </li>
        ))}
      </ul>
      {officialUrl && (
        <p className="px-2.5 pb-2.5 text-[13px]">
          <OfficialLink officialUrl={officialUrl}>Lire ces articles dans le texte officiel ↗</OfficialLink>
        </p>
      )}
    </details>
  )
}

/** Les articles d'une division, ceux sans ligne qui se suivent regroupés. */
function DivisionBlock({
  heading,
  articles,
  officialUrl,
}: {
  heading: string | null
  articles: ArticleBriefDto[]
  officialUrl: string | null
}) {
  const runs = runsBy(articles, (article) => (article.state === 'ready' ? 'ready' : 'waiting'))
  return (
    <div className="mt-3 first:mt-0">
      {heading && (
        <h4 className="text-[12.5px] font-semibold text-ink-soft">
          {heading.split(DIVISION_SEPARATOR).map(readableDivision).join(' › ')}
        </h4>
      )}
      {runs.map((run, index) =>
        run.key === 'waiting' && run.articles.length > 1 ? (
          <WaitingArticles key={index} articles={run.articles} officialUrl={officialUrl} />
        ) : (
          <ul key={index} className="divide-y divide-line">
            {run.articles.map((article, articleIndex) => (
              <ArticleLine key={`${article.label}-${articleIndex}`} article={article} officialUrl={officialUrl} />
            ))}
          </ul>
        ),
      )}
    </div>
  )
}

function partTally(articles: ArticleBriefDto[]) {
  const ready = articles.filter((article) => article.state === 'ready').length
  return `${articles.length} ${articles.length > 1 ? 'articles' : 'article'} · ${ready} ${ready > 1 ? 'résumés' : 'résumé'}`
}

/** Tous les articles, rangés sous leurs divisions ; repliés par grande partie quand le texte est long, la première ouverte. */
function Articles({
  articles,
  officialUrl,
}: {
  articles: ArticleBriefDto[]
  officialUrl: string | null
}) {
  const hasDivisions = articles.some((article) => article.division !== null)

  if (articles.length <= FOLD_ABOVE || !hasDivisions) {
    return (
      <div>
        {runsBy(articles, (article) => article.division).map((run, index) => (
          <DivisionBlock key={index} heading={run.key} articles={run.articles} officialUrl={officialUrl} />
        ))}
      </div>
    )
  }

  const parts = runsBy(articles, (article) => topLevel(article.division))
  const firstNamedPart = parts.findIndex((part) => part.key !== null)
  return (
    <div className="space-y-2.5">
      {parts.map((part, index) =>
        part.key === null ? (
          <DivisionBlock key={index} heading={null} articles={part.articles} officialUrl={officialUrl} />
        ) : (
          <details key={index} open={index === firstNamedPart} className="group/part rounded-xl border border-line">
            <summary className="flex cursor-pointer list-none items-start gap-2 px-3.5 py-3 text-sm font-semibold text-ink [&::-webkit-details-marker]:hidden">
              <Chevron openedBy="group-open/part:rotate-90" />
              <span className="min-w-0">
                {readableDivision(part.key)}
                <span className="block text-[13px] font-normal text-ink-faint">{partTally(part.articles)}</span>
              </span>
            </summary>
            <div className="px-3.5 pb-2">
              {runsBy(part.articles, (article) => article.division).map((run, subIndex) => (
                <DivisionBlock
                  key={subIndex}
                  heading={belowTopLevel(run.key)}
                  articles={run.articles}
                  officialUrl={officialUrl}
                />
              ))}
            </div>
          </details>
        ),
      )}
    </div>
  )
}

function VersionPicker({
  data,
  onSelect,
}: {
  data: DossierBriefResponse
  onSelect: (documentUid: string) => void
}) {
  return (
    <label className="flex flex-wrap items-center gap-2 text-xs text-ink-faint">
      Version
      <select
        value={data.shown?.document_uid ?? ''}
        onChange={(event) => onSelect(event.target.value)}
        className="min-w-0 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-sm text-ink-soft"
      >
        {data.shown === null && (
          <option value="" disabled>
            Choisir une version
          </option>
        )}
        {data.versions.map((version) => (
          <option key={version.document_uid} value={version.document_uid}>
            {version.label} · {formatDate(version.deposited_on)}
            {version.available ? '' : ' (non résumée)'}
          </option>
        ))}
      </select>
    </label>
  )
}

/** La version montrée : nature, chambre, date et lien vers son texte officiel. */
function ShownVersion({ shown }: { shown: TextVersionDto }) {
  return (
    <p className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
      <span className="text-sm font-medium text-ink">{shown.label}</span>
      <Meta>{formatDate(shown.deposited_on)}</Meta>
      {shown.official_url && (
        <a
          href={shown.official_url}
          target="_blank"
          rel="noreferrer"
          className="text-xs font-medium text-accent hover:underline"
        >
          Lire le texte officiel ↗
        </a>
      )}
    </p>
  )
}

/** RM-14 : une version plus récente dont l'Assemblée ne publie pas le texte n'est jamais résumée. */
function NewerVersionNote({ version }: { version: TextVersionDto }) {
  const which = `« ${version.label} », ${formatDate(version.deposited_on)}`
  return (
    <Note>
      {version.chamber === 'sn'
        ? `Une version plus récente, publiée par le Sénat seulement, n'est pas résumée : ${which}.`
        : `Une version plus récente n'est pas résumée, son texte n'étant pas disponible sur le site de l'Assemblée nationale : ${which}.`}
    </Note>
  )
}

const WITHOUT_LINES: Record<'unread' | 'missing' | 'senate_only', string> = {
  unread: "Le texte de cette version n'a pas encore été lu : aucune ligne n'est disponible pour le moment.",
  missing:
    "Le texte de cette version n'a pas pu être lu sur le site de l'Assemblée nationale : aucune ligne n'est disponible.",
  senate_only: "Cette version est publiée par le Sénat seulement : « En bref » ne la résume pas.",
}

function Body({ data }: { data: DossierBriefResponse }) {
  const { shown, text_status: status } = data

  if (shown === null) {
    return (
      <p className="text-sm text-ink-soft">
        {data.versions.length === 0
          ? "Aucune version n'est publiée pour ce texte pour l'instant."
          : "Aucune version de ce texte n'est publiée par l'Assemblée nationale : « En bref » n'a rien à résumer pour l'instant."}
      </p>
    )
  }
  if (status !== 'read') {
    return (
      <p className="text-sm text-ink-soft">
        {WITHOUT_LINES[status ?? 'unread']}
      </p>
    )
  }
  if (data.articles.length === 0) {
    return <p className="text-sm text-ink-soft">Aucun article n'a été reconnu dans cette version.</p>
  }
  return <Articles articles={data.articles} officialUrl={shown.official_url} />
}

function Footer({ data }: { data: DossierBriefResponse }) {
  if (data.models.length === 0 || !data.generated_at) return null
  return (
    <p className="mt-4 text-xs leading-relaxed text-ink-faint">
      Lignes produites par {data.models.join(', ')}, dernière génération le {formatDate(data.generated_at)}.
    </p>
  )
}

/**
 * « En bref » : ce que change chaque article du texte, en une ligne. La
 * version affichée est la dernière disponible, ou celle de `?version=`. Un
 * article sans ligne le dit ; aucune ligne n'est inventée (README.md §8.1).
 */
export default function DossierBrief({ uid }: { uid: string }) {
  const [params, setParams] = useSearchParams()
  const version = params.get('version')
  const { data, isLoading, isError, error } = useQuery<DossierBriefResponse>({
    queryKey: ['dossier-en-bref', uid, version ?? ''],
    queryFn: () =>
      fetchJson<DossierBriefResponse>(
        `/api/dossiers/${uid}/en-bref${version ? `?version=${encodeURIComponent(version)}` : ''}`,
      ),
    enabled: !!uid,
    // Une version inconnue ne s'améliore pas en réessayant.
    retry: (failures, failure) => failure.message !== 'HTTP 404' && failures < 3,
  })

  const selectVersion = (documentUid: string | null) => {
    const next = new URLSearchParams(params)
    if (documentUid) next.set('version', documentUid)
    else next.delete('version')
    setParams(next, { preventScrollReset: true })
  }

  let content
  if (isLoading) {
    content = <p className="animate-pulse text-sm text-ink-faint">Chargement du résumé…</p>
  } else if (isError && version && error.message === 'HTTP 404') {
    content = (
      <p className="text-sm text-ink-soft">
        Cette version n'existe pas pour ce texte.{' '}
        <button
          type="button"
          onClick={() => selectVersion(null)}
          className="font-medium text-accent hover:underline"
        >
          Voir la dernière version
        </button>
      </p>
    )
  } else if (isError || !data) {
    content = <ErrorPanel error={error} />
  } else {
    content = (
      <div className="space-y-3">
        <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
          <div className="min-w-0 space-y-1">
            {data.shown && <ShownVersion shown={data.shown} />}
            {data.text_status === 'read' && !data.is_latest && (
              <Meta>Version précédente : un article sans ligne n'y sera pas résumé.</Meta>
            )}
          </div>
          {data.versions.length > 1 && <VersionPicker data={data} onSelect={selectVersion} />}
        </div>
        {data.newer_unavailable_version && <NewerVersionNote version={data.newer_unavailable_version} />}
        <Body data={data} />
        <Footer data={data} />
      </div>
    )
  }

  return (
    <section className="mb-4" aria-labelledby="dossier-en-bref">
      <Card className="px-4 py-4 sm:px-6 sm:py-5">
        <div className="mb-3 flex flex-wrap items-center gap-2">
          <h3 id="dossier-en-bref" className="text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
            En bref
          </h3>
          <Pill tone="info">Synthèse automatique</Pill>
        </div>
        {content}
      </Card>
    </section>
  )
}
