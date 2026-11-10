import { useState } from 'react'
import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { InfoTip } from './InfoTip'
import ScrutinList, { CoverageNote } from './ScrutinList'
import GuideLink from './GuideLink'
import { Button, SegmentedControl } from './ui'
import type { DossierScrutinsResponse } from '../types/scrutins'

const PAGE_SIZE = 5

const outcomes = [
  { value: '', label: 'Tous' },
  { value: 'adopté', label: 'Adoptés' },
  { value: 'rejeté', label: 'Rejetés' },
]

const ballotTypes = [
  { value: '', label: 'Tous' },
  { value: 'SPO', label: 'Ordinaires' },
  { value: 'SPS', label: 'Solennels' },
  { value: 'MOC', label: 'Motions de censure' },
]

/** Section paginée et filtrable des scrutins rattachés à un dossier. */
export default function DossierScrutins({ uid }: { uid: string }) {
  const [outcome, setOutcome] = useState('')
  const [ballotType, setBallotType] = useState('')
  const [search, setSearch] = useState('')
  const [draft, setDraft] = useState('')
  const [offset, setOffset] = useState(0)

  const params = new URLSearchParams({
    limit: String(PAGE_SIZE),
    offset: String(offset),
  })
  if (outcome) params.set('outcome', outcome)
  if (ballotType) params.set('ballot_type', ballotType)
  if (search) params.set('search', search)

  const { data, isLoading, isError, error } = useQuery<DossierScrutinsResponse>({
    queryKey: ['dossier-scrutins', uid, params.toString()],
    queryFn: () =>
      fetch(`/api/dossiers/${uid}/scrutins?${params}`).then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return res.json()
      }),
    enabled: !!uid,
    placeholderData: keepPreviousData,
  })

  const reset = (setter: (value: string) => void) => (value: string) => {
    setter(value)
    setOffset(0)
  }

  return (
    <section className="mb-6">
      <h3 className="mb-2 text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
        Votes
        {data && data.total > 0 && (
          <span className="ml-2 font-normal normal-case tracking-normal text-ink-faint">
            {data.total}
          </span>
        )}
        {data && (
          <InfoTip label="Votes absents de ce décompte" className="md:hidden">
            {data.coverage_note}
          </InfoTip>
        )}
      </h3>

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
            placeholder="Rechercher dans l'objet…"
            className="min-w-0 flex-1 rounded-lg border border-line bg-surface px-3 py-1.5 text-sm placeholder:text-ink-faint focus:border-accent focus:ring-2 focus:ring-accent/15 focus:outline-none sm:w-64 sm:flex-none"
          />
          <Button type="submit">Rechercher</Button>
        </form>
        <SegmentedControl label="Sort" options={outcomes} value={outcome} onChange={reset(setOutcome)} />
        <SegmentedControl label="Type" options={ballotTypes} value={ballotType} onChange={reset(setBallotType)} />
      </div>

      {isLoading && <p className="animate-pulse text-sm text-ink-faint">Chargement des votes…</p>}
      {isError && <p className="text-sm text-no">Erreur : {error instanceof Error ? error.message : 'inconnue'}</p>}

      {data && (
        <div className="space-y-2">
          {data.total === 0 ? (
            <p className="rounded-lg border border-line bg-surface px-4 py-3 text-sm text-ink-soft">
              {search || outcome || ballotType
                ? 'Aucun vote ne correspond à ces filtres.'
                : 'La source ne rattache aucun vote public à ce texte. Cela ne signifie pas qu’aucun vote n’a eu lieu.'}{' '}
              {!search && !outcome && !ballotType && (
                <GuideLink ancre="q-sans-dossier">Pourquoi un vote n'est pas toujours relié à son texte</GuideLink>
              )}
            </p>
          ) : (
            <>
              <ScrutinList scrutins={data.scrutins} showDossier={false} />
              <div className="flex items-center justify-between gap-3 pt-1">
                <span className="text-xs text-ink-faint">
                  {data.offset + 1}–{data.offset + data.count} sur {data.total}
                </span>
                <div className="flex gap-2">
                  <Button onClick={() => setOffset(Math.max(0, offset - PAGE_SIZE))} disabled={offset === 0}>
                    ← Précédents
                  </Button>
                  <Button onClick={() => setOffset(offset + PAGE_SIZE)} disabled={offset + data.count >= data.total}>
                    Suivants →
                  </Button>
                </div>
              </div>
            </>
          )}
          <CoverageNote note={data.coverage_note} />
        </div>
      )}
    </section>
  )
}
