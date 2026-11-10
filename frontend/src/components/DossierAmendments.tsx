import { useEffect, useState } from 'react'
import { useQuery, keepPreviousData } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import {
  Card,
  ErrorPanel,
  Note,
  Pill,
  SectionTitle,
  TallyLine,
  type PillTone,
} from './ui'
import {
  amendmentAnchor,
  type AmendmentDto,
  type DossierAmendmentsResponse,
} from '../types/amendments'
import AmendmentAuthor from './AmendmentAuthor'

const PAGE_SIZE = 5

const fates = [
  { value: '', label: 'Tous' },
  { value: 'adopted', label: 'Adoptés' },
  { value: 'rejected', label: 'Rejetés' },
  { value: 'other', label: 'Autres' },
]

/** Fallback pour les lignes historiques, jusqu'à la migration de la base. */
function displaySummary(raw: string) {
  return raw
    .replace(/<br\s*\/?>/gi, '\n')
    .replace(/<\/p>/gi, '\n\n')
    .replace(/<[^>]+>/g, '')
    .replace(/&#x([0-9a-f]+);?/gi, (_, hexadecimal: string) => {
      const codePoint = Number.parseInt(hexadecimal, 16)
      return codePoint <= 0x10ffff && !(codePoint >= 0xd800 && codePoint <= 0xdfff)
        ? String.fromCodePoint(codePoint)
        : _
    })
    .replace(/&#([0-9]+);?/g, (_, decimal: string) => {
      const codePoint = Number.parseInt(decimal, 10)
      return codePoint <= 0x10ffff && !(codePoint >= 0xd800 && codePoint <= 0xdfff)
        ? String.fromCodePoint(codePoint)
        : _
    })
    .replace(/&nbsp;/gi, ' ')
    .replace(/&amp;/gi, '&')
    .replace(/&lt;/gi, '<')
    .replace(/&gt;/gi, '>')
    .replace(/&#39;|&apos;/gi, "'")
    .replace(/&quot;/gi, '"')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

/**
 * Teinte du sort. Même échelle que les dossiers : adopté et rejeté se
 * distinguent, tout le reste reste neutre. Un sort hors référentiel (`other`)
 * est neutre lui aussi — il n'a pas à ressembler à un rejet parce qu'on ne
 * sait pas le classer.
 */
const fateTones: Record<string, PillTone> = {
  adopted: 'yes',
  rejected: 'no',
}

function formatDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

function AmendmentRow({ amendment }: { amendment: AmendmentDto }) {
  const summary = amendment.summary
    ? /[<&]/.test(amendment.summary)
      ? displaySummary(amendment.summary)
      : amendment.summary
    : null

  return (
    <Card className="px-4 py-3">
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <span className="font-mono text-xs text-ink-faint">n° {amendment.number}</span>
        <span className="text-sm font-medium text-ink">{amendment.target_title}</span>
        <AmendmentAuthor amendment={amendment} />
        <Pill tone={fateTones[amendment.fate_code] ?? 'neutral'}>{amendment.fate_banner}</Pill>
        {amendment.deposited_on && (
          <span className="text-xs text-ink-faint">
            déposé le {formatDate(amendment.deposited_on)}
          </span>
        )}
      </div>

      {summary ? (
        <details className="mt-2">
          <summary className="cursor-pointer text-sm leading-relaxed text-ink-soft hover:text-ink">
            {amendment.summary_preview ?? summary}
            {amendment.summary_preview_truncated && (
              <span className="ml-1 text-xs font-medium underline">Lire la suite</span>
            )}
          </summary>
          {/* Verbatim : le texte du signataire, ni résumé ni abrégé (RM-03). */}
          <p className="mt-1.5 whitespace-pre-wrap text-sm leading-relaxed text-ink-soft">
            {summary}
          </p>
        </details>
      ) : (
        <p className="mt-2 text-xs text-ink-faint">
          La source ne publie pas d'exposé sommaire pour cet amendement.
        </p>
      )}

      {/* Pied de carte (RM-08) : les scrutins qui l'ont mis aux voix, du plus
          ancien au plus récent, ou la mention qu'aucun n'est rapproché. */}
      <div className="mt-3 border-t border-line pt-2 text-xs text-ink-faint">
        {amendment.votes.length > 0 ? (
          <ul className="space-y-1">
            {amendment.votes.map((vote) => (
              <li key={vote.scrutin_uid} className="flex flex-wrap items-baseline gap-x-2">
                <span>Mis aux voix ·</span>
                <Link
                  to={`/scrutins/${vote.scrutin_uid}`}
                  className="font-medium text-accent hover:underline"
                >
                  scrutin n° {vote.scrutin_number}
                </Link>
                <span>{formatDate(vote.date)}</span>
                <TallyLine tally={vote.tally} dense />
              </li>
            ))}
          </ul>
        ) : (
          <p>Aucun vote public rapproché</p>
        )}
      </div>
    </Card>
  )
}

/**
 * Section amendements d'un dossier. Toujours présente, même vide : une section
 * absente laisserait croire qu'aucun amendement n'a été déposé, alors que la
 * source peut simplement ne rattacher aucun texte de ce dossier.
 *
 * L'ordre est celui du dépôt, la borne est de {PAGE_SIZE} par page, et le total
 * est affiché : paginer n'est pas filtrer (README.md §2, RM-07). Aucun tri par
 * « importance » ni par nombre de cosignataires — ce serait un classement.
 */
function AmendmentNotes({ data }: { data: DossierAmendmentsResponse }) {
  return (
    <>
      {data.source_note} {data.pagination_note} {data.coverage_note}
      {data.coverage.without_summary > 0 && (
        <>
          {' '}
          {data.coverage.without_summary} amendement
          {data.coverage.without_summary > 1 ? 's' : ''} de ce texte
          {data.coverage.without_summary > 1 ? ' sont déposés' : ' est déposé'}{' '}
          sans exposé sommaire publié.
        </>
      )}
    </>
  )
}

export default function DossierAmendments({ uid, initialGroup = '' }: { uid: string; initialGroup?: string }) {
  const [offset, setOffset] = useState(0)
  const [fate, setFate] = useState('')
  const [group, setGroup] = useState(initialGroup)
  const [search, setSearch] = useState('')
  const [draft, setDraft] = useState('')
  // Depuis le détail d'un scrutin : `?amendement=<numéro>` ouvre la page qui
  // porte la carte (le serveur filtre sur ce numéro exact, paramètre `number`).
  const [searchParams, setSearchParams] = useSearchParams()
  const number = searchParams.get('amendement') ?? ''

  useEffect(() => {
    if (initialGroup) {
      setGroup(initialGroup)
      setOffset(0)
    }
  }, [initialGroup])

  const params = new URLSearchParams({
    limit: String(PAGE_SIZE),
    offset: String(offset),
  })
  if (fate) params.set('fate', fate)
  if (group) params.set('group', group)
  if (search) params.set('search', search)
  if (number) params.set('number', number)

  const { data, isLoading, isError, error } = useQuery<DossierAmendmentsResponse>({
    queryKey: ['dossier-amendements', uid, params.toString()],
    queryFn: () =>
      fetch(`/api/dossiers/${uid}/amendements?${params}`).then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return res.json()
      }),
    enabled: !!uid,
    placeholderData: keepPreviousData,
  })

  const hasPrevious = offset > 0
  const hasNext = data ? offset + data.count < data.total : false
  const reset = (setter: (value: string) => void) => (value: string) => {
    setter(value)
    setOffset(0)
  }
  const showAll = () => {
    const next = new URLSearchParams(searchParams)
    next.delete('amendement')
    setSearchParams(next, { replace: true })
    setOffset(0)
  }

  // Les cartes arrivent après la page : le navigateur ne peut pas suivre
  // l'ancre seul.
  useEffect(() => {
    if (number && data) {
      document
        .getElementById(amendmentAnchor(number))
        ?.scrollIntoView({ block: 'center' })
    }
  }, [number, data])

  return (
    <section id="dossier-amendements" className="mb-6">
      <SectionTitle
        count={data && data.total > 0 ? data.total : undefined}
        info={data && <AmendmentNotes data={data} />}
      >
        Amendements
      </SectionTitle>

      <div className="mb-3 flex flex-wrap items-center gap-x-4 gap-y-2">
        <form
          onSubmit={(event) => {
            event.preventDefault()
            setSearch(draft)
            setOffset(0)
          }}
          className="flex w-full gap-1.5 sm:w-auto"
        >
          <input
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder="Rechercher un amendement…"
            className="min-w-0 flex-1 rounded-lg border border-line bg-surface px-3 py-1.5 text-sm placeholder:text-ink-faint focus:border-accent focus:ring-2 focus:ring-accent/15 focus:outline-none sm:w-64 sm:flex-none"
          />
          <button type="submit" className="rounded-md border border-line px-2.5 py-1 text-xs text-ink-soft hover:bg-surface-soft">
            Rechercher
          </button>
        </form>
        <select
          aria-label="Filtrer par sort"
          value={fate}
          onChange={(event) => reset(setFate)(event.target.value)}
          className="rounded-lg border border-line bg-surface px-2.5 py-1.5 text-sm text-ink-soft"
        >
          {fates.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
        </select>
        <select
          aria-label="Filtrer par groupe politique"
          value={group}
          onChange={(event) => reset(setGroup)(event.target.value)}
          className="w-full min-w-0 rounded-lg border border-line bg-surface px-2.5 py-1.5 text-sm text-ink-soft sm:w-auto sm:max-w-xs"
        >
          <option value="">Tous les groupes</option>
          {data?.groups.map((option) => (
            <option key={option.uid} value={option.uid}>{option.abbrev} — {option.label}</option>
          ))}
        </select>
      </div>

      {number && (
        <div className="mb-3 flex flex-wrap items-center gap-x-3 gap-y-1 rounded-lg border border-line bg-surface-soft px-3 py-2 text-xs text-ink-soft">
          <span>Amendement n° {number}, ouvert depuis un vote.</span>
          <button
            type="button"
            onClick={showAll}
            className="font-medium text-accent hover:underline"
          >
            Afficher tous les amendements du texte
          </button>
        </div>
      )}

      {isLoading && (
        <p className="animate-pulse text-sm text-ink-faint">
          Chargement des amendements…
        </p>
      )}

      {isError && <ErrorPanel error={error} />}

      {data && (
        <div className="space-y-2">
          {data.total === 0 ? (
            /*
              Deux vides très différents, qui doivent se lire différemment.
              Le premier est un état du site, le second un constat sur ce qui
              a été ingéré — et aucun des deux n'autorise à parler au nom de
              l'Assemblée (README.md §2).
            */
            <p className="rounded-lg border border-line bg-surface px-4 py-3 text-sm text-ink-soft">
              {data.coverage.base_total === 0
                ? "Les amendements ne sont pas encore ingérés sur ce site. Cette section restera vide jusqu'à la première passe d'ingestion."
                : number
                  ? `Aucun amendement n° ${number} n'est ingéré pour ce texte.`
                  : "Aucun amendement ingéré ne se rattache à ce texte. Cela ne signifie pas qu'aucun amendement n'a été déposé sur ce texte."}
            </p>
          ) : (
            <>
              {data.amendments.map((amendment) => (
                // L'ancre que vise le détail d'un scrutin (SUJET CU-03).
                <div
                  key={amendment.uid}
                  id={amendmentAnchor(amendment.number)}
                  className="scroll-mt-20"
                >
                  <AmendmentRow amendment={amendment} />
                </div>
              ))}

              <div className="flex items-center justify-between gap-3 pt-1">
                <span className="text-xs text-ink-faint">
                  {offset + 1}–{offset + data.count} sur {data.total}
                </span>
                <div className="flex gap-2">
                  <button type="button" disabled={!hasPrevious} onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))} className="rounded-md border border-line px-2.5 py-1 text-xs text-ink-soft disabled:opacity-40 enabled:hover:bg-surface-soft">Précédents</button>
                  <button type="button" disabled={!hasNext} onClick={() => setOffset(offset + PAGE_SIZE)} className="rounded-md border border-line px-2.5 py-1 text-xs text-ink-soft disabled:opacity-40 enabled:hover:bg-surface-soft">Suivants</button>
                </div>
              </div>
            </>
          )}

          <Note desktopOnly>
            <AmendmentNotes data={data} />
          </Note>
        </div>
      )}
    </section>
  )
}
