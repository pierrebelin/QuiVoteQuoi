import { useState } from 'react'
import { useQuery, keepPreviousData } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { BottomSheet } from '../components/BottomSheet'
import DossierCard, { type DossierCardDto } from '../components/DossierCard'
import DossierFilters from '../components/DossierFilters'
import { Icon } from '../components/icons'
import { FOCUS, SheetButton } from '../components/SheetControls'
import { Button, ErrorPanel, Loading, PageHeader } from '../components/ui'
import { fetchJson } from '../lib/fetchJson'
import type { FamiliesResponse } from '../types/themes'

interface DossierPageResponse {
  page: number
  per_page: number
  total: number
  total_pages: number
  dossiers: DossierCardDto[]
}

const PER_PAGE = 20

/*
  Les cinq sorts que le domaine distingue, tous proposés et cumulables :
  n'en offrir que trois laisserait croire que les deux autres n'existent pas.
  Par défaut, la liste montre les dossiers terminés (promulgués, retirés,
  absorbés) : les ouverts, sans acte de conclusion, sont presque tous vides et
  n'apprennent rien. « En cours » les ajoute sur demande. Il désigne ici les
  dossiers sans conclusion enregistrée par la source, pas un jugement sur
  leur avenir (README.md §6).
*/
const outcomes = [
  { value: 'promulgated', label: 'Promulgués' },
  { value: 'withdrawn', label: 'Retirés' },
  { value: 'merged_into', label: 'Absorbés' },
  { value: 'rejected', label: 'Rejetés' },
  { value: 'no_recorded_conclusion', label: 'En cours' },
]

const DEFAULT_OUTCOMES = ['promulgated', 'withdrawn', 'merged_into']

const initiatives = [
  { value: '', label: 'Toutes' },
  { value: 'government', label: 'Projets de loi' },
  { value: 'parliamentary', label: 'Propositions' },
]

type Criterion = 'search' | 'outcome' | 'initiative' | 'theme' | 'page'

export default function DossierListPage() {
  const [params, setParams] = useSearchParams()
  const page = Math.max(1, Number(params.get('page')) || 1)
  const search = params.get('search') ?? ''
  // Sorts retenus, séparés par des virgules dans l'URL ; sans paramètre, les
  // dossiers terminés.
  const outcomeParam = params.get('outcome') ?? ''
  const selected = outcomeParam
    ? outcomeParam.split(',').filter(Boolean)
    : DEFAULT_OUTCOMES
  const initiative = params.get('initiative') ?? ''
  const theme = params.get('theme') ?? ''

  const [draft, setDraft] = useState(search)
  const [sheetOpen, setSheetOpen] = useState(false)

  const filtered = Boolean(search || outcomeParam || initiative || theme)
  // Filtres posés dans le tiroir (la recherche a son champ) : sorts autres que
  // ceux par défaut, initiative, thème. C'est le nombre affiché sur « Filtres ».
  const outcomesChanged = !(
    selected.length === DEFAULT_OUTCOMES.length &&
    DEFAULT_OUTCOMES.every((v) => selected.includes(v))
  )
  const activeCount = (outcomesChanged ? 1 : 0) + (initiative ? 1 : 0) + (theme ? 1 : 0)

  const query = new URLSearchParams({
    page: String(page),
    per_page: String(PER_PAGE),
  })
  if (search) query.set('search', search)
  query.set('outcome', selected.join(','))
  if (initiative) query.set('initiative', initiative)
  if (theme) query.set('family', theme)

  const { data, isLoading, isError, error, isPlaceholderData } =
    useQuery<DossierPageResponse>({
      queryKey: ['dossiers', query.toString()],
      queryFn: () =>
        fetch(`/api/dossiers?${query}`).then((res) => {
          if (!res.ok) throw new Error(`HTTP ${res.status}`)
          return res.json()
        }),
      // Garde la page précédente affichée pendant le chargement de la suivante :
      // la liste ne clignote pas à chaque changement de page.
      placeholderData: keepPreviousData,
    })

  const families = useQuery<FamiliesResponse>({
    queryKey: ['themes'],
    queryFn: () => fetchJson<FamiliesResponse>('/api/themes'),
    retry: false,
  })

  const today = new Date().toISOString().slice(0, 10)

  /*
    Les filtres vivent dans l'URL, comme la pagination : une liste filtrée se
    partage et se retrouve avec le bouton « précédent » du navigateur.
  */
  const apply = (changes: Partial<Record<Criterion, string>>) => {
    const next = new URLSearchParams(params)
    for (const [key, value] of Object.entries(changes)) {
      if (value) next.set(key, value)
      else next.delete(key)
    }
    // Un critère qui change ramène à la première page : la page 7 d'une liste
    // filtrée n'a rien à voir avec la page 7 de la précédente.
    if (!('page' in changes)) next.delete('page')
    setParams(next)
    window.scrollTo({ top: 0 })
  }

  // Cumulables, mais jamais aucun : une liste sans sort n'a pas de sens, le
  // dernier choix reste donc allumé. Revenir au défaut efface le paramètre.
  const toggleOutcome = (value: string) => {
    const next = selected.includes(value)
      ? selected.filter((v) => v !== value)
      : [...selected, value]
    if (next.length === 0) return
    const ordered = outcomes.map((o) => o.value).filter((v) => next.includes(v))
    const isDefault =
      ordered.length === DEFAULT_OUTCOMES.length &&
      DEFAULT_OUTCOMES.every((v) => ordered.includes(v))
    apply({ outcome: isDefault ? '' : ordered.join(',') })
  }

  const goTo = (next: number) => apply({ page: next === 1 ? '' : String(next) })

  const reset = () => {
    setDraft('')
    setParams({})
    window.scrollTo({ top: 0 })
  }

  // Le tiroir efface les filtres qu'il porte et laisse la recherche, qui reste
  // visible dans son champ derrière lui.
  const resetFilters = () => apply({ outcome: '', initiative: '', theme: '' })

  const setTheme = (code: string) => {
    if (code !== theme) apply({ theme: code })
  }
  const setInitiative = (value: string) => {
    if (value !== initiative) apply({ initiative: value })
  }

  // Tout ce qui restreint la liste est dit en clair sous la recherche, tiroir
  // fermé : aucun filtre actif n'est caché derrière le bouton.
  const outcomeNames = outcomes
    .filter((o) => selected.includes(o.value))
    .map((o) => o.label)
    .join(', ')
  const summary = [
    outcomeNames || selected.join(', '),
    initiatives.find((i) => i.value === initiative)?.label,
    theme && (families.data?.families.find((f) => f.code === theme)?.label ?? theme),
  ]
    .filter(Boolean)
    .join(' · ')

  const seeLabel =
    !data || isPlaceholderData
      ? 'Voir les dossiers'
      : data.total === 0
        ? 'Aucun dossier ne correspond'
        : `Voir ${data.total.toLocaleString('fr-FR')} dossier${data.total > 1 ? 's' : ''}`

  const resetButton = filtered && (
    <button
      type="button"
      onClick={reset}
      className={`inline-flex min-h-11 shrink-0 items-center rounded-lg px-2 text-[13px] font-medium text-accent hover:underline ${FOCUS}`}
    >
      Tout afficher
    </button>
  )

  return (
    <>
      <PageHeader
        title="Dossiers législatifs"
        lede={
          !data
            ? 'Les dossiers législatifs de cette législature, du plus récemment actif au plus ancien.'
            : filtered
              ? /*
                  Sous filtre, le total affiché n'est plus celui de la base :
                  le dire évite de laisser croire que la législature se réduit
                  à ces dossiers (README.md §2).
                */
                `${data.total.toLocaleString('fr-FR')} dossier${data.total > 1 ? 's' : ''} correspond${data.total > 1 ? 'ent' : ''} à ces filtres. Les filtres restreignent l'affichage, jamais la base.${theme ? ' Seuls les dossiers déjà rattachés à un thème y figurent.' : ''}`
              : `Les ${data.total.toLocaleString('fr-FR')} dossiers terminés sous cette législature, du plus récemment actif au plus ancien. Les dossiers en cours, ou tous, sont à un clic.`
        }
        /*
          Les scrutins sont le détail d'un dossier avant d'être une rubrique :
          leur porte d'entrée est ici, à côté du titre, plutôt qu'en septième
          onglet de la barre de navigation.
        */
        aside={
          <Link
            to="/scrutins"
            className="inline-flex min-h-11 items-center text-sm font-medium text-accent hover:underline"
          >
            Accéder à tous les scrutins →
          </Link>
        }
      />

      {/*
        Sous 1024 px : la recherche, un bouton « Filtres » qui ouvre le tiroir,
        et une ligne qui dit ce qui est filtré. À partir de 1024 px, le tiroir
        n'a plus lieu d'être : les mêmes filtres tiennent dans une colonne à
        gauche des cartes, que la page garde sous les yeux en défilant.
      */}
      <div className="lg:grid lg:grid-cols-[296px_minmax(0,1fr)] lg:items-start lg:gap-6">
        <div className="mb-4 flex flex-col gap-2 lg:sticky lg:top-[4.5rem] lg:mb-0 lg:max-h-[calc(100dvh-5.5rem)] lg:gap-0 lg:overflow-y-auto lg:rounded-2xl lg:border lg:border-line lg:bg-surface lg:p-4 lg:shadow-card">
          <form
            role="search"
            onSubmit={(e) => {
              e.preventDefault()
              apply({ search: draft.trim() })
            }}
            className="flex gap-2"
          >
            {/* Le champ mesure 44 px comme le cadre : il recouvre sa bordure d'un
                pixel de chaque côté, pour que toute la hauteur soit touchable. */}
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
                placeholder="Rechercher un titre…"
                aria-label="Rechercher dans le titre du dossier"
                className="h-11 min-w-0 flex-1 bg-transparent pr-3 text-base text-ink outline-none placeholder:text-ink-faint md:text-sm"
              />
            </div>
            <button
              type="button"
              aria-haspopup="dialog"
              onClick={() => setSheetOpen(true)}
              className={`inline-flex min-h-11 shrink-0 items-center gap-1.5 rounded-xl bg-surface px-3.5 text-sm font-semibold text-ink ring-1 ring-line-strong ring-inset transition-colors hover:bg-surface-soft lg:hidden ${FOCUS}`}
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
          </form>

          <div className="flex items-center gap-2 lg:hidden">
            <p className="flex min-w-0 flex-1 items-start gap-1.5 text-[13px] leading-snug text-ink-soft">
              <Icon name="check" className="mt-0.5 size-3.5" strokeWidth={3} />
              <span>
                <span className="sr-only">Filtres appliqués : </span>
                {summary}
              </span>
            </p>
            {resetButton}
          </div>

          <div className="hidden lg:block">
            <DossierFilters
              layout="aside"
              outcomes={outcomes}
              selectedOutcomes={selected}
              onToggleOutcome={toggleOutcome}
              initiatives={initiatives}
              initiative={initiative}
              onInitiative={setInitiative}
              families={families.data?.families}
              theme={theme}
              onTheme={setTheme}
            />
            {resetButton && <div className="mt-3">{resetButton}</div>}
          </div>
        </div>

        <div className="min-w-0">
          {isLoading && <Loading>Chargement des dossiers…</Loading>}
          {isError && <ErrorPanel error={error} />}

          {data && data.dossiers.length === 0 && (
            <p className="rounded-xl border border-line bg-surface px-4 py-3 text-sm text-ink-soft shadow-card">
              Aucun dossier ne correspond à ces filtres.
            </p>
          )}

          {/*
            Une carte par dossier, aérée : l'état en bandeau, le titre en entier,
            les thèmes, puis qui l'a déposé, quand, où il en est et s'il a été voté.
            Moins de dossiers par écran, mais chacun se lit sans ouvrir sa page.
          */}
          {data && data.dossiers.length > 0 && (
            <div
              className={`space-y-3 transition-opacity ${isPlaceholderData ? 'opacity-50' : ''}`}
            >
              {data.dossiers.map((d) => (
                <DossierCard
                  key={d.uid}
                  dossier={d}
                  today={today}
                  onFamily={(code) => apply({ theme: code })}
                />
              ))}
            </div>
          )}

          {data && data.total_pages > 1 && (
            <nav className="mt-5 flex items-center justify-between gap-4">
              <Button onClick={() => goTo(page - 1)} disabled={page <= 1}>
                ← Précédent
              </Button>
              <span className="text-xs text-ink-faint">
                Page {data.page} sur {data.total_pages}
              </span>
              <Button
                onClick={() => goTo(page + 1)}
                disabled={page >= data.total_pages}
              >
                Suivant →
              </Button>
            </nav>
          )}
        </div>
      </div>

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
        <DossierFilters
          layout="sheet"
          outcomes={outcomes}
          selectedOutcomes={selected}
          onToggleOutcome={toggleOutcome}
          initiatives={initiatives}
          initiative={initiative}
          onInitiative={setInitiative}
          families={families.data?.families}
          theme={theme}
          onTheme={setTheme}
        />
      </BottomSheet>
    </>
  )
}
