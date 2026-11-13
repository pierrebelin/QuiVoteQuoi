import { useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { useSearchParams } from 'react-router'
import { BottomSheet } from '../components/BottomSheet'
import { Icon, type IconName } from '../components/icons'
import { InfoTip } from '../components/InfoTip'
import DossierCard, { type DossierPageResponse } from '../components/DossierCard'
import FamilyFilter from '../components/FamilyFilter'
import ScrutinFeed from '../components/ScrutinFeed'
import VotesTabs from '../components/VotesTabs'
import { FOCUS, SheetButton, SheetSection } from '../components/SheetControls'
import { Button, ErrorPanel, Loading, SegmentedControl } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import type { ScrutinListResponse } from '../types/scrutins'
import type { FamiliesResponse } from '../types/themes'

const PAGE_SIZE = 50

const outcomes = [
  { value: '', label: 'Tous' },
  { value: 'adopté', label: 'Adoptés' },
  { value: 'rejeté', label: 'Rejetés' },
]

const ballotTypes = [
  { value: '', label: 'Tous' },
  { value: 'SPO', label: 'Ordinaires' },
  { value: 'SPS', label: 'Solennels' },
  { value: 'MOC', label: 'Censure' },
]

const dossierFilters = [
  { value: '', label: 'Tous' },
  { value: 'true', label: 'Avec' },
  { value: 'false', label: 'Sans' },
]

/**
 * `scrutin` : le fil par jour, scrutins successifs d'un texte réunis ;
 * `texte` : les textes, du plus récemment voté au plus ancien.
 */
type ListView = 'scrutin' | 'texte'

const views: { value: ListView; label: string; icon: IconName }[] = [
  { value: 'scrutin', label: 'Par vote', icon: 'list' },
  { value: 'texte', label: 'Par texte', icon: 'dossiers' },
]

/*
  Filtres de la vue par texte, tous dans l'URL : une liste filtrée se partage,
  et /dossiers y redirige en reprenant les siens. Sans paramètre : les textes
  votés, toutes origines, toutes issues, tous sujets.
*/
const PARAM_SCOPE = 'textes'
const SCOPE_ALL = 'tous'

/*
  Les cinq issues que le domaine distingue, cumulables, toutes retenues par
  défaut : en masquer une laisserait croire qu'elle n'existe pas. Jamais
  « en cours » : la source ne l'affirme pas (README.md §6).
*/
const textOutcomes = [
  { value: 'promulgated', label: 'Promulgués' },
  { value: 'withdrawn', label: 'Retirés' },
  { value: 'merged_into', label: 'Absorbés' },
  { value: 'rejected', label: 'Rejetés' },
  { value: 'no_recorded_conclusion', label: 'Sans conclusion à ce jour' },
]
const ALL_OUTCOMES = textOutcomes.map((option) => option.value)

const initiatives = [
  { value: '', label: 'Toutes' },
  { value: 'government', label: 'Projets de loi' },
  { value: 'parliamentary', label: 'Propositions de loi' },
]

function plural(count: number, word: string) {
  return `${count.toLocaleString('fr-FR')} ${word}${count > 1 ? 's' : ''}`
}

/** Pastilles du tiroir, à la ligne quand elles débordent ; pleines et cochées quand elles sont retenues. */
function ChipGroup({
  label,
  options,
  isOn,
  onToggle,
}: {
  label: string
  options: { value: string; label: string }[]
  isOn: (value: string) => boolean
  onToggle: (value: string) => void
}) {
  return (
    <SheetSection title={label}>
      <div role="group" aria-label={label} className="flex flex-wrap gap-1.5">
        {options.map((option) => {
          const on = isOn(option.value)
          return (
            <button
              key={option.value}
              type="button"
              aria-pressed={on}
              onClick={() => onToggle(option.value)}
              className={`inline-flex min-h-11 items-center gap-1 rounded-full px-3.5 text-[13.5px] whitespace-nowrap transition-colors ${FOCUS} ${
                on
                  ? 'bg-ink font-semibold text-surface'
                  : 'bg-surface font-medium text-ink-soft ring-1 ring-line-strong ring-inset hover:text-ink'
              }`}
            >
              {on && <Icon name="check" className="size-3.5" strokeWidth={3} />}
              {option.label}
            </button>
          )
        })}
      </div>
    </SheetSection>
  )
}

/** Compteur de la page, aligné à droite du titre. */
function PageCount({ total, label }: { total: number; label: string }) {
  return (
    <div className="shrink-0 text-right">
      <p className="figure text-[40px] tabular-nums">{total.toLocaleString('fr-FR')}</p>
      <p className="mt-1 text-xs text-ink-faint">{label}</p>
    </div>
  )
}

/** Précédents / Suivants, communs aux deux vues : même taille de page. */
function Pager({
  offset,
  count,
  total,
  onChange,
}: {
  offset: number
  count: number
  total: number
  onChange: (offset: number) => void
}) {
  return (
    <div className="mt-6 flex items-center justify-between gap-3">
      <Button onClick={() => onChange(Math.max(0, offset - PAGE_SIZE))} disabled={offset === 0}>
        ← Précédents
      </Button>
      {total > 0 && (
        <span className="text-xs text-ink-faint">
          {(offset + 1).toLocaleString('fr-FR')} à {(offset + count).toLocaleString('fr-FR')} sur{' '}
          {total.toLocaleString('fr-FR')}
        </span>
      )}
      <Button onClick={() => onChange(offset + PAGE_SIZE)} disabled={offset + count >= total}>
        Suivants →
      </Button>
    </div>
  )
}

/**
 * Bascule entre les deux lectures du fil. Pleine largeur sur mobile, où elle
 * se touche au pouce ; 320 px à gauche au-delà.
 */
function ViewToggle({ view, onChange }: { view: ListView; onChange: (view: ListView) => void }) {
  return (
    <div
      role="group"
      aria-label="Regrouper les votes"
      className="grid w-full grid-cols-2 gap-1 rounded bg-surface-soft p-[3px] md:w-80"
    >
      {views.map((option) => {
        const on = view === option.value
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={on}
            onClick={() => onChange(option.value)}
            className={`flex min-h-11 items-center justify-center gap-1.5 rounded px-2 text-sm whitespace-nowrap transition-colors ${FOCUS} ${
              on ? 'bg-surface font-semibold text-ink shadow-card' : 'font-medium text-ink-soft hover:text-ink'
            }`}
          >
            <Icon name={option.icon} className="size-[18px]" />
            {option.label}
          </button>
        )
      })}
    </div>
  )
}

/** Choix exclusif du tiroir : des boutons de 44 px côte à côte. */
function SheetChoice({
  label,
  options,
  value,
  onChange,
}: {
  label: string
  options: { value: string; label: string }[]
  value: string
  onChange: (value: string) => void
}) {
  return (
    <SheetSection title={label}>
      <div
        role="group"
        aria-label={label}
        className="grid auto-cols-fr grid-flow-col gap-1 rounded bg-surface-soft p-[3px]"
      >
        {options.map((option) => {
          const on = value === option.value
          return (
            <button
              key={option.value}
              type="button"
              aria-pressed={on}
              onClick={() => onChange(option.value)}
              className={`flex min-h-11 items-center justify-center gap-1 rounded px-1 text-[13px] whitespace-nowrap transition-colors ${FOCUS} ${
                on ? 'bg-ink font-semibold text-surface' : 'font-medium text-ink-soft hover:text-ink'
              }`}
            >
              {on && <Icon name="check" className="size-3.5" strokeWidth={3} />}
              {option.label}
            </button>
          )
        })}
      </div>
    </SheetSection>
  )
}

/** Bouton « Filtres » et le nombre de filtres posés. */
function FiltersButton({ count, onClick, className = '' }: { count: number; onClick: () => void; className?: string }) {
  return (
    <button
      type="button"
      aria-haspopup="dialog"
      onClick={onClick}
      className={`inline-flex min-h-11 shrink-0 items-center gap-1.5 rounded bg-surface px-3.5 text-sm font-semibold text-ink ring-1 ring-line-strong ring-inset transition-colors hover:bg-surface-soft ${FOCUS} ${className}`}
    >
      <Icon name="tune" className="size-[18px]" />
      Filtres
      {count > 0 && (
        <span className="flex min-w-5 items-center justify-center rounded-full bg-ink px-1.5 text-xs leading-5 font-semibold text-surface">
          {count}
          <span className="sr-only"> actif{count > 1 ? 's' : ''}</span>
        </span>
      )}
    </button>
  )
}

export default function ScrutinListPage() {
  // La vue et la recherche vivent dans l'URL : le lien se partage, et le
  // retour depuis un scrutin retrouve la même lecture. Sans paramètre, par
  // vote. `vue=dossier` est l'ancienne adresse de la vue par texte.
  const [urlParams, setUrlParams] = useSearchParams()
  const askedView = urlParams.get('vue')
  const view: ListView = askedView === 'texte' || askedView === 'dossier' ? 'texte' : 'scrutin'
  const search = urlParams.get('search') ?? ''

  // Filtres de la vue par texte.
  const allTexts = urlParams.get(PARAM_SCOPE) === SCOPE_ALL
  const outcomeParam = urlParams.get('outcome') ?? ''
  const askedOutcomes = outcomeParam.split(',').filter((value) => ALL_OUTCOMES.includes(value))
  const selectedOutcomes = askedOutcomes.length > 0 ? askedOutcomes : ALL_OUTCOMES
  const outcomesNarrowed = selectedOutcomes.length < ALL_OUTCOMES.length
  const initiative = initiatives.some((option) => option.value === urlParams.get('initiative'))
    ? (urlParams.get('initiative') ?? '')
    : ''
  const theme = urlParams.get('theme') ?? ''

  const [outcome, setOutcome] = useState('')
  const [ballotType, setBallotType] = useState('')
  const [withDossier, setWithDossier] = useState('')
  const [draft, setDraft] = useState(search)
  const [sheetOpen, setSheetOpen] = useState(false)
  // La page est attachée à la vue où elle a été choisie : changer de vue, même
  // par le retour arrière, repart de la première page.
  const [paging, setPaging] = useState<{ view: ListView; offset: number }>({ view, offset: 0 })
  const offset = paging.view === view ? paging.offset : 0
  const setOffset = (next: number) => setPaging({ view, offset: next })

  /** Pose ou retire des paramètres d'URL ; une valeur vide retire le sien. */
  function applyParams(changes: Record<string, string>) {
    setUrlParams((previous) => {
      const updated = new URLSearchParams(previous)
      for (const [key, value] of Object.entries(changes)) {
        if (value) updated.set(key, value)
        else updated.delete(key)
      }
      return updated
    })
    setOffset(0)
  }

  function changeView(next: ListView) {
    if (next === view) return
    applyParams({ vue: next === 'texte' ? 'texte' : '' })
    setPaging({ view: next, offset: 0 })
  }

  // Cumulables, mais jamais aucune : le dernier choix reste allumé. Revenir
  // aux cinq efface le paramètre.
  function toggleOutcome(value: string) {
    const next = selectedOutcomes.includes(value)
      ? selectedOutcomes.filter((v) => v !== value)
      : [...selectedOutcomes, value]
    if (next.length === 0) return
    const ordered = ALL_OUTCOMES.filter((v) => next.includes(v))
    applyParams({ outcome: ordered.length === ALL_OUTCOMES.length ? '' : ordered.join(',') })
  }

  const params = new URLSearchParams({
    limit: String(PAGE_SIZE),
    offset: String(offset),
  })
  if (outcome) params.set('outcome', outcome)
  if (ballotType) params.set('ballot_type', ballotType)
  if (withDossier) params.set('with_dossier', withDossier)
  if (search) params.set('search', search)

  const { data, isLoading, isError, error, isPlaceholderData } = useQuery<ScrutinListResponse>({
    queryKey: ['scrutins', params.toString()],
    queryFn: () =>
      fetch(`/api/scrutins?${params}`).then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return res.json()
      }),
    // Les résultats précédents restent affichés pendant qu'un filtre charge :
    // la page ne se vide pas à chaque clic.
    placeholderData: keepPreviousData,
    enabled: view === 'scrutin',
  })

  // Vue par texte : du dernier vote au plus ancien ; avec « Tous », les textes
  // sans vote viennent ensuite. Même taille de page que le fil.
  const textFilters = new URLSearchParams()
  if (search) textFilters.set('search', search)
  if (outcomesNarrowed) textFilters.set('outcome', selectedOutcomes.join(','))
  if (initiative) textFilters.set('initiative', initiative)
  if (theme) textFilters.set('family', theme)

  const dossierParams = new URLSearchParams(textFilters)
  dossierParams.set('page', String(offset / PAGE_SIZE + 1))
  dossierParams.set('per_page', String(PAGE_SIZE))
  dossierParams.set('sort', 'last_scrutin')
  if (!allTexts) dossierParams.set('with_scrutins', 'true')

  const dossiers = useQuery<DossierPageResponse>({
    queryKey: ['dossiers', dossierParams.toString()],
    queryFn: () => fetchJson<DossierPageResponse>(`/api/dossiers?${dossierParams}`),
    placeholderData: keepPreviousData,
    enabled: view === 'texte',
  })

  // L'autre périmètre (tous si l'on voit les votés, et l'inverse), pour les
  // deux nombres du choix « Votés / Tous » et l'encart de fin de liste.
  const otherParams = new URLSearchParams(textFilters)
  otherParams.set('page', '1')
  otherParams.set('per_page', '1')
  if (allTexts) otherParams.set('with_scrutins', 'true')
  const other = useQuery<DossierPageResponse>({
    queryKey: ['dossiers', otherParams.toString()],
    queryFn: () => fetchJson<DossierPageResponse>(`/api/dossiers?${otherParams}`),
    placeholderData: keepPreviousData,
    enabled: view === 'texte',
  })
  const currentTotal = dossiers.data?.total
  const votedTotal = allTexts ? other.data?.total : currentTotal
  const allTotal = allTexts ? currentTotal : other.data?.total
  const unvotedTotal = votedTotal !== undefined && allTotal !== undefined ? allTotal - votedTotal : undefined

  const families = useQuery<FamiliesResponse>({
    queryKey: ['themes'],
    queryFn: () => fetchJson<FamiliesResponse>('/api/themes'),
    retry: false,
    enabled: view === 'texte',
  })

  const today = new Date().toISOString().slice(0, 10)

  function reset(setter: (v: string) => void) {
    return (v: string) => {
      setter(v)
      setOffset(0)
    }
  }

  // Filtres posés dans le tiroir (la recherche a son champ) : c'est le nombre
  // affiché sur « Filtres ».
  const textFiltered = Boolean(outcomesNarrowed || initiative || theme)
  const activeCount =
    view === 'texte'
      ? (allTexts ? 1 : 0) + (outcomesNarrowed ? 1 : 0) + (initiative ? 1 : 0) + (theme ? 1 : 0)
      : (outcome ? 1 : 0) + (ballotType ? 1 : 0) + (withDossier ? 1 : 0)

  function resetFilters() {
    if (view === 'texte') {
      applyParams({ [PARAM_SCOPE]: '', outcome: '', initiative: '', theme: '' })
      return
    }
    setOutcome('')
    setBallotType('')
    setWithDossier('')
    setOffset(0)
  }

  const seeLabel =
    view === 'texte'
      ? !dossiers.data || dossiers.isPlaceholderData
        ? 'Voir les textes'
        : dossiers.data.total === 0
          ? 'Aucun texte ne correspond'
          : `Voir ${plural(dossiers.data.total, 'texte')}`
      : !data || isPlaceholderData
        ? 'Voir les votes'
        : data.total === 0
          ? 'Aucun vote ne correspond'
          : `Voir ${plural(data.total, 'vote')}`

  const searchLabel = view === 'texte' ? 'Rechercher dans le titre du texte' : "Rechercher dans l'objet du vote"

  function submitSearch() {
    applyParams({ search: draft.trim() })
  }

  const scopeOptions = [
    { value: '', label: votedTotal === undefined ? 'Votés' : `Votés (${votedTotal.toLocaleString('fr-FR')})` },
    { value: SCOPE_ALL, label: allTotal === undefined ? 'Tous' : `Tous (${allTotal.toLocaleString('fr-FR')})` },
  ]
  const setScope = (value: string) => applyParams({ [PARAM_SCOPE]: value })
  const setInitiative = (value: string) => applyParams({ initiative: value })

  const lede =
    view === 'texte'
      ? 'Tous les votes publics de la législature, regroupés par texte. Aucune sélection, aucun classement.'
      : "Tous les votes publics de la législature, dans l'ordre où ils ont eu lieu. Aucune sélection, aucun classement."

  return (
    <>
      <VotesTabs current="liste" />
      <div className="mt-[22px] mb-5">
        {/* Le compteur reste sur la ligne du titre dans les deux vues, quelle que soit la longueur de son libellé. */}
        <div className="flex items-end justify-between gap-4">
          <h1 className="heading-2">
            Votes
            <InfoTip label="À propos de cette page : Votes" className="md:hidden">
              <p>{lede}</p>
              {view === 'scrutin' && data && <p>{data.coverage_note}</p>}
            </InfoTip>
          </h1>
          {view === 'scrutin' && data && (
            <PageCount
              total={data.total}
              label={`vote${data.total > 1 ? 's' : ''}${outcome || ballotType || withDossier || search ? ' pour ces filtres' : ' publics'}`}
            />
          )}
          {view === 'texte' && dossiers.data && (
            <PageCount
              total={dossiers.data.total}
              label={`texte${dossiers.data.total > 1 ? 's' : ''}${allTexts ? '' : dossiers.data.total > 1 ? ' votés' : ' voté'}${textFiltered || search ? ' pour ces filtres' : ''}`}
            />
          )}
        </div>
        <p className="mt-1 hidden max-w-2xl text-ink-soft md:block">{lede}</p>
      </div>

      <div className="mb-4">
        <ViewToggle view={view} onChange={changeView} />
      </div>

      {/*
        Sous 768 px : la recherche et un bouton « Filtres » sur une ligne, les
        filtres dans un tiroir. Au-delà, la carte de recherche et de filtres.
      */}
      <form
        role="search"
        onSubmit={(e) => {
          e.preventDefault()
          submitSearch()
        }}
        className="mb-5 flex gap-2 md:hidden"
      >
        <div className="flex h-11 min-w-0 flex-1 items-center rounded border border-line-strong bg-surface text-ink-soft focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
          <button
            type="submit"
            aria-label="Lancer la recherche"
            className={`flex size-11 shrink-0 items-center justify-center rounded hover:text-ink ${FOCUS}`}
          >
            <Icon name="search" />
          </button>
          <input
            type="search"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            enterKeyHint="search"
            autoComplete="off"
            placeholder={view === 'texte' ? 'Rechercher un texte…' : 'Rechercher…'}
            aria-label={searchLabel}
            className="h-11 min-w-0 flex-1 bg-transparent pr-3 text-base text-ink outline-none placeholder:text-ink-faint"
          />
        </div>
        <FiltersButton count={activeCount} onClick={() => setSheetOpen(true)} />
      </form>

      {/* Recherche et filtres dans une seule carte, au-dessus de la liste. */}
      <div className="hidden flex-wrap items-center gap-x-5 gap-y-2.5 rounded border border-line bg-surface px-3.5 py-3 shadow-card md:flex">
        <form
          onSubmit={(e) => {
            e.preventDefault()
            submitSearch()
          }}
          className="flex min-w-0 flex-[1_1_320px] gap-1.5"
        >
          <label className="flex min-w-0 flex-1 items-center gap-2 rounded border border-line bg-surface px-2.5 py-1.5 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={1.8}
              strokeLinecap="round"
              aria-hidden
              className="size-4 shrink-0 text-ink-faint"
            >
              <circle cx="11" cy="11" r="7" />
              <path d="M20 20l-3.5-3.5" />
            </svg>
            <input
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              placeholder={`${searchLabel}…`}
              aria-label={searchLabel}
              className="min-w-0 flex-1 bg-transparent text-sm placeholder:text-ink-faint focus:outline-none"
            />
          </label>
          <Button type="submit">Rechercher</Button>
        </form>

        {view === 'scrutin' && (
          <>
            <SegmentedControl label="Issue" options={outcomes} value={outcome} onChange={reset(setOutcome)} />
            <SegmentedControl label="Type" options={ballotTypes} value={ballotType} onChange={reset(setBallotType)} />
            <SegmentedControl
              label="Texte"
              options={dossierFilters}
              value={withDossier}
              onChange={reset(setWithDossier)}
            />
          </>
        )}
        {/* Votés / Tous et l'origine restent sous les yeux ; l'issue et le
            sujet, plus longs, sont dans le panneau « Filtres ». */}
        {view === 'texte' && (
          <>
            <SegmentedControl
              label="Textes"
              options={scopeOptions}
              value={allTexts ? SCOPE_ALL : ''}
              onChange={setScope}
            />
            <SegmentedControl label="Origine" options={initiatives} value={initiative} onChange={setInitiative} />
            <FiltersButton count={activeCount} onClick={() => setSheetOpen(true)} />
          </>
        )}
      </div>

      {view === 'scrutin' && data && (
        <p className="mt-3 mb-5 hidden items-center gap-2 px-0.5 text-[12.5px] text-ink-faint md:flex">
          <Icon name="info" className="size-4 shrink-0" />
          {data.coverage_note}
        </p>
      )}

      {view === 'texte' && (
        <>
          <p className="-mt-2 mb-3 px-0.5 text-[12.5px] text-ink-faint md:mt-3">
            {allTexts
              ? 'Du plus récemment voté au plus ancien, puis les textes sans vote public'
              : 'Du plus récemment voté au plus ancien'}
          </p>
          {dossiers.isLoading && <Loading>Chargement des textes…</Loading>}
          {dossiers.isError && <ErrorPanel error={dossiers.error} />}
          {dossiers.data && (
            <>
              {dossiers.data.dossiers.length === 0 ? (
                <p className="rounded border border-line bg-surface px-4 py-3 text-sm text-ink-soft shadow-card">
                  {allTexts ? 'Aucun texte ne correspond à ces filtres.' : 'Aucun texte voté ne correspond à ces filtres.'}
                </p>
              ) : (
                <div className={`space-y-3 transition-opacity ${dossiers.isPlaceholderData ? 'opacity-50' : ''}`}>
                  {dossiers.data.dossiers.map((d) => (
                    <DossierCard key={d.uid} dossier={d} today={today} showSplit />
                  ))}
                </div>
              )}
              <Pager
                offset={offset}
                count={dossiers.data.dossiers.length}
                total={dossiers.data.total}
                onChange={setOffset}
              />
              {/* Par défaut, seuls les textes votés : ceux qui ne l'ont pas
                  été sont dits, et à un geste (README.md §2). */}
              {!allTexts && unvotedTotal !== undefined && unvotedTotal > 0 && (
                <div className="mt-5 flex items-start gap-3 rounded border border-dashed border-line-strong bg-surface px-4 py-3.5">
                  <Icon name="info" className="mt-0.5 size-4 shrink-0 text-ink-faint" />
                  <div>
                    <p className="text-[13.5px] leading-snug text-ink-soft">
                      <b className="font-semibold text-ink">
                        {unvotedTotal.toLocaleString('fr-FR')} autre{unvotedTotal > 1 ? 's' : ''} texte
                        {unvotedTotal > 1 ? 's' : ''}
                      </b>{' '}
                      {unvotedTotal > 1 ? "n'ont eu" : "n'a eu"} aucun vote public à l'Assemblée.
                    </p>
                    <button
                      type="button"
                      onClick={() => {
                        setScope(SCOPE_ALL)
                        window.scrollTo({ top: 0 })
                      }}
                      className={`mt-1 inline-flex min-h-11 items-center gap-1 rounded text-sm font-semibold link ${FOCUS}`}
                    >
                      Les inclure
                      <Icon name="chevron" className="size-4" />
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
        </>
      )}

      {view === 'scrutin' && isLoading && <Loading>Chargement des votes…</Loading>}
      {view === 'scrutin' && isError && <ErrorPanel error={error} />}

      {view === 'scrutin' && data && (
        <>
          {data.count === 0 ? (
            <p className="rounded border border-line bg-surface px-4 py-3 text-sm text-ink-soft shadow-card">
              Aucun vote ne correspond à ces filtres.
            </p>
          ) : (
            <ScrutinFeed scrutins={data.scrutins} />
          )}

          <Pager offset={offset} count={data.count} total={data.total} onChange={setOffset} />
        </>
      )}

      <BottomSheet
        open={sheetOpen}
        onClose={() => setSheetOpen(false)}
        title="Filtres"
        sideOnWide={view === 'texte'}
        footer={
          <>
            <SheetButton onClick={resetFilters} disabled={activeCount === 0}>
              Réinitialiser
            </SheetButton>
            <SheetButton primary onClick={() => setSheetOpen(false)}>
              {seeLabel}
            </SheetButton>
          </>
        }
      >
        {/* Le bouton du pied change de libellé à chaque filtre : la même
            information, annoncée aux lecteurs d'écran. */}
        <p role="status" className="sr-only">
          {seeLabel}
        </p>
        {view === 'scrutin' ? (
          <>
            <SheetChoice label="Issue" options={outcomes} value={outcome} onChange={reset(setOutcome)} />
            <SheetChoice label="Type" options={ballotTypes} value={ballotType} onChange={reset(setBallotType)} />
            <SheetChoice label="Texte" options={dossierFilters} value={withDossier} onChange={reset(setWithDossier)} />
          </>
        ) : (
          <>
            <SheetChoice label="Textes" options={scopeOptions} value={allTexts ? SCOPE_ALL : ''} onChange={setScope} />
            <p className="mx-0.5 mt-1.5 text-[12.5px] leading-snug text-ink-soft">
              « Tous » ajoute les textes sans vote public : leur fiche donne l'auteur, le dépôt et la dernière étape
              connue.
            </p>
            <ChipGroup
              label="Origine"
              options={initiatives}
              isOn={(value) => value === initiative}
              onToggle={setInitiative}
            />
            <ChipGroup
              label="Issue"
              options={textOutcomes}
              isOn={(value) => selectedOutcomes.includes(value)}
              onToggle={toggleOutcome}
            />
            {families.data && (
              <FamilyFilter
                families={families.data.families}
                theme={theme}
                onTheme={(code) => applyParams({ theme: code })}
              />
            )}
          </>
        )}
      </BottomSheet>
    </>
  )
}
