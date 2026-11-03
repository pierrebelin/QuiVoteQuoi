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
  const rest = division.split(DIVISION_SEPARATOR).slice(1).join(' › ')
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

function ArticleLine({
  article,
  officialUrl,
}: {
  article: ArticleBriefDto
  officialUrl: string | null
}) {
  const name = `Article ${article.label}`
  if (article.state === 'ready') {
    return (
      <li className="py-1.5 text-sm leading-relaxed text-ink">
        <span className="font-medium">{name}</span>
        {' '}— {article.line}
      </li>
    )
  }
  return (
    <li className="py-1.5 text-sm leading-relaxed text-ink-faint">
      <span className="font-medium">{name}</span>
      {' '}— {article.state === 'pending' ? 'résumé en attente' : 'résumé indisponible'}
      {officialUrl && (
        <>
          {' · '}
          <a href={officialUrl} target="_blank" rel="noreferrer" className="text-accent hover:underline">
            Texte officiel ↗
          </a>
        </>
      )}
    </li>
  )
}

/** Les articles d'une suite, sous l'en-tête discret de leur division. */
function DivisionBlock({
  heading,
  articles,
  officialUrl,
}: {
  heading: string | null
  articles: ArticleBriefDto[]
  officialUrl: string | null
}) {
  return (
    <div className="mt-3 first:mt-0">
      {heading && (
        <h4 className="text-[12.5px] font-semibold text-ink-soft">{heading.replaceAll(DIVISION_SEPARATOR, ' › ')}</h4>
      )}
      <ul className="divide-y divide-line">
        {articles.map((article, index) => (
          <ArticleLine key={`${article.label}-${index}`} article={article} officialUrl={officialUrl} />
        ))}
      </ul>
    </div>
  )
}

/** Tous les articles, rangés sous leurs divisions ; repliés par grande partie quand le texte est long. */
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

  return (
    <div className="space-y-2">
      {runsBy(articles, (article) => topLevel(article.division)).map((part, index) => (
        <details key={index} className="rounded-lg border border-line bg-surface-soft/40 px-3 py-2">
          <summary className="cursor-pointer text-sm font-medium text-ink">
            {part.key ?? 'Articles hors division'}
            <span className="ml-2 text-xs font-normal text-ink-faint">
              {part.articles.length} {part.articles.length > 1 ? 'articles' : 'article'}
            </span>
          </summary>
          <div className="mt-2">
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
      ))}
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
          ? "Aucune version du texte n'est publiée pour ce dossier pour l'instant."
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
        Cette version n'existe pas pour ce dossier.{' '}
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
