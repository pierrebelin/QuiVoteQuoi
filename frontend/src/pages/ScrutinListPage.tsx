import { useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import ScrutinFeed from '../components/ScrutinFeed'
import { Button, ErrorPanel, Loading, SegmentedControl } from '../components/ui'
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

export default function ScrutinListPage() {
  const [outcome, setOutcome] = useState('')
  const [ballotType, setBallotType] = useState('')
  const [withDossier, setWithDossier] = useState('')
  const [search, setSearch] = useState('')
  const [draft, setDraft] = useState('')
  const [offset, setOffset] = useState(0)

  const params = new URLSearchParams({
    limit: String(PAGE_SIZE),
    offset: String(offset),
  })
  if (outcome) params.set('outcome', outcome)
  if (ballotType) params.set('ballot_type', ballotType)
  if (withDossier) params.set('with_dossier', withDossier)
  if (search) params.set('search', search)

  const { data, isLoading, isError, error } = useQuery<ScrutinListResponse>({
    queryKey: ['scrutins', params.toString()],
    queryFn: () =>
      fetch(`/api/scrutins?${params}`).then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return res.json()
      }),
    // Les résultats précédents restent affichés pendant qu'un filtre charge :
    // la page ne se vide pas à chaque clic.
    placeholderData: keepPreviousData,
  })

  function reset(setter: (v: string) => void) {
    return (v: string) => {
      setter(v)
      setOffset(0)
    }
  }

  return (
    <>
      <div className="mb-5 flex flex-wrap items-end justify-between gap-4">
        <div>
          <h1 className="text-[28px] leading-tight font-semibold tracking-tight">Scrutins</h1>
          <p className="mt-1 max-w-2xl text-ink-soft">
            Tous les scrutins publics de la législature, dans l'ordre où ils ont eu lieu. Aucune sélection,
            aucun classement.
          </p>
        </div>
        {data && (
          <div className="text-right">
            <p className="text-3xl leading-none font-semibold tracking-tight tabular-nums">
              {data.total.toLocaleString('fr-FR')}
            </p>
            <p className="mt-1 text-xs text-ink-faint">
              scrutin{data.total > 1 ? 's' : ''}
              {outcome || ballotType || withDossier || search ? ' pour ces filtres' : ' publics'}
            </p>
          </div>
        )}
      </div>

      {/* Recherche et filtres dans une seule carte, au-dessus du fil. */}
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2.5 rounded-2xl border border-line bg-surface px-3.5 py-3 shadow-card">
        <form
          onSubmit={(e) => {
            e.preventDefault()
            setSearch(draft)
            setOffset(0)
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
              placeholder="Rechercher dans l'objet du scrutin…"
              aria-label="Rechercher dans l'objet du scrutin"
              className="min-w-0 flex-1 bg-transparent text-sm placeholder:text-ink-faint focus:outline-none"
            />
          </label>
          <Button type="submit">Rechercher</Button>
        </form>

        <SegmentedControl label="Issue" options={outcomes} value={outcome} onChange={reset(setOutcome)} />
        <SegmentedControl label="Type" options={ballotTypes} value={ballotType} onChange={reset(setBallotType)} />
        <SegmentedControl
          label="Dossier"
          options={dossierFilters}
          value={withDossier}
          onChange={reset(setWithDossier)}
        />
      </div>

      {data && (
        <p className="mt-3 mb-5 flex items-center gap-2 px-0.5 text-[12.5px] text-ink-faint">
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

      {isLoading && <Loading>Chargement des scrutins…</Loading>}
      {isError && <ErrorPanel error={error} />}

      {data && (
        <>
          {data.count === 0 ? (
            <p className="rounded-xl border border-line bg-surface px-4 py-3 text-sm text-ink-soft shadow-card">
              Aucun scrutin ne correspond à ces filtres.
            </p>
          ) : (
            <ScrutinFeed scrutins={data.scrutins} />
          )}

          <div className="mt-6 flex items-center justify-between gap-3">
            <Button
              onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))}
              disabled={offset === 0}
            >
              ← Précédents
            </Button>
            {data.total > 0 && (
              <span className="text-xs text-ink-faint">
                {(data.offset + 1).toLocaleString('fr-FR')} à {(data.offset + data.count).toLocaleString('fr-FR')} sur{' '}
                {data.total.toLocaleString('fr-FR')}
              </span>
            )}
            <Button
              onClick={() => setOffset(offset + PAGE_SIZE)}
              disabled={offset + data.count >= data.total}
            >
              Suivants →
            </Button>
          </div>
        </>
      )}
    </>
  )
}
