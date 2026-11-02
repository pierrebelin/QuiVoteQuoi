import { useState } from 'react'
import { useQuery, keepPreviousData } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import DossierCard, { type DossierCardDto } from '../components/DossierCard'
import { FamilyIllustration } from '../components/FamilyIllustration'
import {
  Button,
  ErrorPanel,
  Loading,
  PageHeader,
  SegmentedControl,
} from '../components/ui'
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
  Les cinq sorts que le domaine distingue, tous proposés : n'en offrir que
  trois laisserait croire que les deux autres n'existent pas. « Sans
  conclusion » est le libellé de la source, pas un jugement sur le dossier
  (README.md §6).
*/
const outcomes = [
  { value: '', label: 'Tous' },
  { value: 'promulgated', label: 'Promulgués' },
  { value: 'rejected', label: 'Rejetés' },
  { value: 'withdrawn', label: 'Retirés' },
  { value: 'merged_into', label: 'Absorbés' },
  { value: 'no_recorded_conclusion', label: 'Sans conclusion' },
]

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
  const outcome = params.get('outcome') ?? ''
  const initiative = params.get('initiative') ?? ''
  const theme = params.get('theme') ?? ''

  const [draft, setDraft] = useState(search)

  const filtered = Boolean(search || outcome || initiative || theme)

  const query = new URLSearchParams({
    page: String(page),
    per_page: String(PER_PAGE),
  })
  if (search) query.set('search', search)
  if (outcome) query.set('outcome', outcome)
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

  const goTo = (next: number) => apply({ page: next === 1 ? '' : String(next) })

  const reset = () => {
    setDraft('')
    setParams({})
    window.scrollTo({ top: 0 })
  }

  return (
    <>
      <PageHeader
        title="Dossiers législatifs"
        lede={
          !data
            ? 'Tous les dossiers ouverts sous cette législature, du plus récemment actif au plus ancien.'
            : filtered
              ? /*
                  Sous filtre, le total affiché n'est plus celui de la base :
                  le dire évite de laisser croire que la législature se réduit
                  à ces dossiers (README.md §2).
                */
                `${data.total.toLocaleString('fr-FR')} dossier${data.total > 1 ? 's' : ''} correspond${data.total > 1 ? 'ent' : ''} à ces filtres. Les filtres restreignent l'affichage, jamais la base.${theme ? ' Seuls les dossiers déjà rattachés à un thème y figurent.' : ''}`
              : `Les ${data.total.toLocaleString('fr-FR')} dossiers ouverts sous cette législature, du plus récemment actif au plus ancien. Rien n'est écarté.`
        }
        /*
          Les scrutins sont le détail d'un dossier avant d'être une rubrique :
          leur porte d'entrée est ici, à côté du titre, plutôt qu'en septième
          onglet de la barre de navigation.
        */
        aside={
          <Link
            to="/scrutins"
            className="text-sm font-medium text-accent hover:underline"
          >
            Accéder à tous les scrutins →
          </Link>
        }
      />

      {/* Recherche et filtres sur une seule ligne, comme sur les scrutins :
          la largeur du site les porte, et trois blocs empilés repousseraient
          la liste vers le bas. */}
      <div className="mb-4 flex flex-wrap items-center gap-x-4 gap-y-2">
        <form
          onSubmit={(e) => {
            e.preventDefault()
            apply({ search: draft.trim() })
          }}
          className="flex w-full gap-1.5 sm:w-auto"
        >
          <input
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            placeholder="Rechercher dans le titre du dossier…"
            className="min-w-0 flex-1 rounded-lg border border-line bg-surface px-3 py-1.5 text-sm shadow-card placeholder:text-ink-faint focus:border-accent focus:ring-2 focus:ring-accent/15 focus:outline-none sm:w-72 sm:flex-none"
          />
          <Button type="submit">Rechercher</Button>
        </form>

        <SegmentedControl
          label="Sort"
          options={outcomes}
          value={outcome}
          onChange={(value) => apply({ outcome: value })}
        />
        <SegmentedControl
          label="Initiative"
          options={initiatives}
          value={initiative}
          onChange={(value) => apply({ initiative: value })}
        />

        {filtered && (
          <button
            type="button"
            onClick={reset}
            className="text-xs font-medium text-accent hover:underline"
          >
            Tout afficher
          </button>
        )}
      </div>

      {/* Les thèmes en vignettes plutôt qu'en liste déroulante : on reconnaît
          un sujet à son pictogramme avant d'en lire le libellé. La rangée
          défile sur petit écran au lieu de repousser la liste. */}
      {families.data && (
        <div className="-mx-4 mb-6 flex gap-2 overflow-x-auto px-4 pb-1 sm:mx-0 sm:flex-wrap sm:overflow-visible sm:px-0">
          <button
            type="button"
            onClick={() => apply({ theme: '' })}
            aria-pressed={!theme}
            className={`shrink-0 rounded-xl border px-3 py-1.5 text-[13px] font-medium transition ${
              !theme
                ? 'border-ink bg-surface text-ink ring-1 ring-ink'
                : 'border-line bg-surface text-ink-soft hover:border-line-strong'
            }`}
          >
            Tous les thèmes
          </button>
          {families.data.families.map((family) => (
            <button
              key={family.code}
              type="button"
              onClick={() => apply({ theme: theme === family.code ? '' : family.code })}
              aria-pressed={theme === family.code}
              title={family.scope}
              className={`inline-flex shrink-0 items-center gap-2 rounded-xl border py-1 pr-3 pl-1 text-[13px] font-medium whitespace-nowrap transition ${
                theme === family.code
                  ? 'border-ink bg-surface text-ink ring-1 ring-ink'
                  : 'border-line bg-surface text-ink-soft hover:border-line-strong hover:text-ink'
              }`}
            >
              <FamilyIllustration
                code={family.code}
                className="size-7 rounded-lg"
                glyphClassName="size-[18px]"
              />
              {family.label}
            </button>
          ))}
        </div>
      )}

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
          className={`space-y-4 transition-opacity ${isPlaceholderData ? 'opacity-50' : ''}`}
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
    </>
  )
}
