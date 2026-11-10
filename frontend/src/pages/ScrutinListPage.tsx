import { useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { Link, useNavigate, useSearchParams } from 'react-router'
import { BottomSheet } from '../components/BottomSheet'
import { Icon, type IconName } from '../components/icons'
import { InfoTip } from '../components/InfoTip'
import DossierCard, { type DossierPageResponse } from '../components/DossierCard'
import ScrutinFeed from '../components/ScrutinFeed'
import { FOCUS, SheetButton, SheetSection } from '../components/SheetControls'
import { Button, ErrorPanel, Loading, SegmentedControl } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import type { ScrutinListResponse } from '../types/scrutins'

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
 * `scrutin` : le fil par jour, scrutins successifs d'un dossier réunis ;
 * `dossier` : les dossiers votés, du dernier scrutin au plus ancien.
 */
type ListView = 'scrutin' | 'dossier'

const views: { value: ListView; label: string; icon: IconName }[] = [
  { value: 'scrutin', label: 'Par vote', icon: 'list' },
  { value: 'dossier', label: 'Par texte', icon: 'dossiers' },
]

/*
  Sorts du dossier, un seul à la fois. La vue par dossier sert à suivre les
  textes que la source n'a pas conclus : « Sans conclusion à ce jour » est
  présélectionné, les autres restent à un geste. Jamais « en cours » : la
  source ne l'affirme pas (README.md §6).
*/
const ALL_STATES = 'tous'
const NO_CONCLUSION = 'no_recorded_conclusion'

// La pastille présélectionnée vient en tête : sur mobile, la rangée défile et
// une sélection hors écran ne se verrait pas.
const dossierStates = [
  { value: NO_CONCLUSION, label: 'Sans conclusion à ce jour' },
  { value: 'promulgated', label: 'Promulgués' },
  { value: 'rejected', label: 'Rejetés' },
  { value: 'withdrawn', label: 'Retirés' },
  { value: 'merged_into', label: 'Absorbés' },
  { value: ALL_STATES, label: 'Tous les états' },
]

/** Pastilles de sort, défilant horizontalement sur mobile plutôt que de passer à la ligne. */
function StateChips({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <div
      role="group"
      aria-label="État du texte"
      className="-mx-4 flex gap-1.5 overflow-x-auto px-4 pb-1 [scrollbar-width:none] md:mx-0 md:flex-wrap md:px-0"
    >
      {dossierStates.map((option) => {
        const on = value === option.value
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={on}
            onClick={() => onChange(option.value)}
            className={`inline-flex min-h-9 shrink-0 items-center gap-1 rounded-full px-3 text-[13px] whitespace-nowrap transition-colors ${FOCUS} ${
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
  )
}

/** Compteur de la page, aligné à droite du titre. */
function PageCount({ total, label }: { total: number; label: string }) {
  return (
    <div className="shrink-0 text-right">
      <p className="text-3xl leading-none font-semibold tracking-tight tabular-nums">{total.toLocaleString('fr-FR')}</p>
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
      className="grid w-full grid-cols-2 gap-1 rounded-xl bg-surface-soft p-[3px] md:w-80"
    >
      {views.map((option) => {
        const on = view === option.value
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={on}
            onClick={() => onChange(option.value)}
            className={`flex min-h-11 items-center justify-center gap-1.5 rounded-[9px] px-2 text-sm whitespace-nowrap transition-colors ${FOCUS} ${
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

/** Choix exclusif du tiroir : des boutons de 44 px côte à côte, comme l'initiative des dossiers. */
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
        className="grid auto-cols-fr grid-flow-col gap-1 rounded-xl bg-surface-soft p-[3px]"
      >
        {options.map((option) => {
          const on = value === option.value
          return (
            <button
              key={option.value}
              type="button"
              aria-pressed={on}
              onClick={() => onChange(option.value)}
              className={`flex min-h-11 items-center justify-center gap-1 rounded-[9px] px-1 text-[13px] whitespace-nowrap transition-colors ${FOCUS} ${
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

export default function ScrutinListPage() {
  // La vue vit dans l'URL : le lien se partage, et le retour depuis un
  // scrutin retrouve la même lecture. Sans paramètre, par scrutin. L'état
  // choisi dans la vue par dossier y vit aussi ; absent : sans conclusion.
  const [urlParams, setUrlParams] = useSearchParams()
  const navigate = useNavigate()
  const view: ListView = urlParams.get('vue') === 'dossier' ? 'dossier' : 'scrutin'
  const askedState = urlParams.get('etat')
  const dossierState =
    dossierStates.find((option) => option.value === askedState)?.value ?? NO_CONCLUSION

  const [outcome, setOutcome] = useState('')
  const [ballotType, setBallotType] = useState('')
  const [withDossier, setWithDossier] = useState('')
  const [search, setSearch] = useState('')
  const [draft, setDraft] = useState('')
  const [sheetOpen, setSheetOpen] = useState(false)
  // La page est attachée à la vue où elle a été choisie : changer de vue, même
  // par le retour arrière, repart de la première page.
  const [paging, setPaging] = useState<{ view: ListView; offset: number }>({ view, offset: 0 })
  const offset = paging.view === view ? paging.offset : 0
  const setOffset = (next: number) => setPaging({ view, offset: next })

  function changeView(next: ListView) {
    if (next === view) return
    setUrlParams((previous) => {
      const updated = new URLSearchParams(previous)
      if (next === 'dossier') updated.set('vue', 'dossier')
      else updated.delete('vue')
      return updated
    })
    setPaging({ view: next, offset: 0 })
  }

  function changeDossierState(next: string) {
    setUrlParams((previous) => {
      const updated = new URLSearchParams(previous)
      if (next === NO_CONCLUSION) updated.delete('etat')
      else updated.set('etat', next)
      return updated
    })
    setOffset(0)
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

  // Vue par dossier : les dossiers ayant au moins un scrutin, du dernier
  // scrutin au plus ancien, avec la même taille de page que le fil.
  const dossierParams = new URLSearchParams({
    page: String(offset / PAGE_SIZE + 1),
    per_page: String(PAGE_SIZE),
    with_scrutins: 'true',
    sort: 'last_scrutin',
  })
  if (dossierState !== ALL_STATES) dossierParams.set('outcome', dossierState)
  if (search) dossierParams.set('search', search)

  const dossiers = useQuery<DossierPageResponse>({
    queryKey: ['dossiers', dossierParams.toString()],
    queryFn: () => fetchJson<DossierPageResponse>(`/api/dossiers?${dossierParams}`),
    placeholderData: keepPreviousData,
    enabled: view === 'dossier',
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
  const activeCount = (outcome ? 1 : 0) + (ballotType ? 1 : 0) + (withDossier ? 1 : 0)

  function resetFilters() {
    setOutcome('')
    setBallotType('')
    setWithDossier('')
    setOffset(0)
  }

  const seeLabel =
    !data || isPlaceholderData
      ? 'Voir les votes'
      : data.total === 0
        ? 'Aucun vote ne correspond'
        : `Voir ${data.total.toLocaleString('fr-FR')} vote${data.total > 1 ? 's' : ''}`

  const searchLabel =
    view === 'dossier' ? 'Rechercher dans le titre du texte' : "Rechercher dans l'objet du vote"

  function submitSearch() {
    setSearch(draft)
    setOffset(0)
  }

  return (
    <>
      <div className="mb-5">
        {/* Le compteur reste sur la ligne du titre dans les deux vues, quelle que soit la longueur de son libellé. */}
        <div className="flex items-end justify-between gap-4">
          <h1 className="text-[28px] leading-tight font-semibold tracking-tight">
            Votes
            <InfoTip label="À propos de cette page : Votes" className="md:hidden">
              <p>
                Tous les votes publics de la législature, dans l'ordre où ils ont eu lieu. Aucune sélection,
                aucun classement.
              </p>
              {data && <p>{data.coverage_note}</p>}
            </InfoTip>
          </h1>
          {view === 'scrutin' && data && (
            <PageCount
              total={data.total}
              label={`vote${data.total > 1 ? 's' : ''}${outcome || ballotType || withDossier || search ? ' pour ces filtres' : ' publics'}`}
            />
          )}
          {view === 'dossier' && dossiers.data && (
            <PageCount
              total={dossiers.data.total}
              label={`texte${dossiers.data.total > 1 ? 's' : ''}${dossierState !== ALL_STATES || search ? ' pour ces filtres' : ' avec vote'}`}
            />
          )}
        </div>
        <p className="mt-1 hidden max-w-2xl text-ink-soft md:block">
          Tous les votes publics de la législature, dans l'ordre où ils ont eu lieu. Aucune sélection,
          aucun classement.
        </p>
        {/* Pendant du lien « Accéder à tous les scrutins » de la liste des dossiers. */}
        <Link
          to="/dossiers"
          className="inline-flex min-h-11 items-center text-sm font-medium text-accent hover:underline"
        >
          Tous les textes →
        </Link>
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
        <div className="flex h-11 min-w-0 flex-1 items-center rounded-xl border border-line-strong bg-surface text-ink-soft focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
          <button
            type="submit"
            aria-label="Lancer la recherche"
            className={`flex size-11 shrink-0 items-center justify-center rounded-xl hover:text-ink ${FOCUS}`}
          >
            <Icon name="search" />
          </button>
          <input
            type="search"
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            enterKeyHint="search"
            autoComplete="off"
            placeholder="Rechercher…"
            aria-label={searchLabel}
            className="h-11 min-w-0 flex-1 bg-transparent pr-3 text-base text-ink outline-none placeholder:text-ink-faint"
          />
        </div>
        {view === 'scrutin' && (
          <button
            type="button"
            aria-haspopup="dialog"
            onClick={() => setSheetOpen(true)}
            className={`inline-flex min-h-11 shrink-0 items-center gap-1.5 rounded-xl bg-surface px-3.5 text-sm font-semibold text-ink ring-1 ring-line-strong ring-inset transition-colors hover:bg-surface-soft ${FOCUS}`}
          >
            <Icon name="tune" className="size-[18px]" />
            Filtres
            {activeCount > 0 && (
              <span className="flex min-w-5 items-center justify-center rounded-full bg-ink px-1.5 text-xs leading-5 font-semibold text-surface">
                {activeCount}
                <span className="sr-only"> actif{activeCount > 1 ? 's' : ''}</span>
              </span>
            )}
          </button>
        )}
      </form>

      {/* Recherche et filtres dans une seule carte, au-dessus du fil. */}
      <div className="hidden flex-wrap items-center gap-x-5 gap-y-2.5 rounded-2xl border border-line bg-surface px-3.5 py-3 shadow-card md:flex">
        <form
          onSubmit={(e) => {
            e.preventDefault()
            submitSearch()
          }}
          className="flex min-w-0 flex-[1_1_320px] gap-1.5"
        >
          <label className="flex min-w-0 flex-1 items-center gap-2 rounded-lg border border-line bg-surface px-2.5 py-1.5 focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/15">
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
      </div>

      {view === 'dossier' && (
        <div className="-mt-2 mb-5 md:mt-3">
          <StateChips value={dossierState} onChange={changeDossierState} />
        </div>
      )}

      {view === 'scrutin' && data && (
        <p className="mt-3 mb-5 hidden items-center gap-2 px-0.5 text-[12.5px] text-ink-faint md:flex">
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={1.8}
            strokeLinecap="round"
            aria-hidden
            className="size-4 shrink-0"
          >
            <circle cx="12" cy="12" r="9" />
            <path d="M12 8h.01M11 12h1v4h1" />
          </svg>
          {data.coverage_note}
        </p>
      )}

      {view === 'dossier' && (
        <>
          {dossiers.isLoading && <Loading>Chargement des textes…</Loading>}
          {dossiers.isError && <ErrorPanel error={dossiers.error} />}
          {dossiers.data && (
            <>
              {dossiers.data.dossiers.length === 0 ? (
                <p className="rounded-xl border border-line bg-surface px-4 py-3 text-sm text-ink-soft shadow-card">
                  Aucun texte voté ne correspond à ces filtres.
                </p>
              ) : (
                <div className={`space-y-3 transition-opacity ${dossiers.isPlaceholderData ? 'opacity-50' : ''}`}>
                  {dossiers.data.dossiers.map((d) => (
                    <DossierCard
                      key={d.uid}
                      dossier={d}
                      today={today}
                      onFamily={(code) => navigate(`/dossiers?theme=${encodeURIComponent(code)}`)}
                      showSplit
                    />
                  ))}
                </div>
              )}
              <Pager
                offset={offset}
                count={dossiers.data.dossiers.length}
                total={dossiers.data.total}
                onChange={setOffset}
              />
            </>
          )}
        </>
      )}

      {view === 'scrutin' && isLoading && <Loading>Chargement des votes…</Loading>}
      {view === 'scrutin' && isError && <ErrorPanel error={error} />}

      {view === 'scrutin' && data && (
        <>
          {data.count === 0 ? (
            <p className="rounded-xl border border-line bg-surface px-4 py-3 text-sm text-ink-soft shadow-card">
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
        <SheetChoice label="Issue" options={outcomes} value={outcome} onChange={reset(setOutcome)} />
        <SheetChoice label="Type" options={ballotTypes} value={ballotType} onChange={reset(setBallotType)} />
        <SheetChoice label="Texte" options={dossierFilters} value={withDossier} onChange={reset(setWithDossier)} />
      </BottomSheet>
    </>
  )
}
