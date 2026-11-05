import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { useEffect, useId, useRef, type ReactNode } from 'react'
import { fetchJson } from '../../lib/fetchJson'
import type { CandidateComparisonResponse, CandidateDto } from '../../types/candidates'
import { formatDate } from '../../types/scrutins'
import type { FamilyDto } from '../../types/themes'
import { FamilyIllustration } from '../FamilyIllustration'
import { Icon } from '../icons'
import { CandidatePortrait } from './CandidateIdentity'

const FIVE_MINUTES = 5 * 60_000

/**
 * Une ligne dépliable par thème : vignette, libellé, chevron. Un seul thème est
 * ouvert à la fois, le parent le décide (paramètre `theme` de l'URL). Ouvrir un
 * thème ramène sa ligne à l'écran si le dépliage de la précédente l'a déplacée.
 */
export function ThemeRow({
  family,
  open,
  onToggle,
  children,
}: {
  family: FamilyDto
  open: boolean
  onToggle: () => void
  children: ReactNode
}) {
  const buttonId = useId()
  const panelId = useId()
  const buttonRef = useRef<HTMLButtonElement>(null)

  useEffect(() => {
    if (open) buttonRef.current?.scrollIntoView({ block: 'nearest' })
  }, [open])

  return (
    <div className="bg-surface">
      <h4>
        <button
          ref={buttonRef}
          id={buttonId}
          type="button"
          aria-expanded={open}
          aria-controls={panelId}
          onClick={onToggle}
          className={`grid min-h-[54px] w-full scroll-mt-20 grid-cols-[38px_minmax(0,1fr)_18px] items-center gap-2.5 px-3.5 py-2 text-left text-[14.5px] leading-tight font-semibold text-ink transition-colors hover:bg-surface-soft focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent ${
            open ? 'bg-surface-soft' : ''
          }`}
        >
          <FamilyIllustration
            code={family.code}
            className="size-[38px] rounded-[10px]"
            glyphClassName="size-[22px]"
          />
          <span>{family.label}</span>
          <Icon
            name="chevron"
            className={`size-[18px] text-ink-soft transition-transform ${open ? 'rotate-90' : ''}`}
          />
        </button>
      </h4>
      {open && (
        <div id={panelId} role="region" aria-labelledby={buttonId} className="border-t border-line">
          {children}
        </div>
      )}
    </div>
  )
}

/** Un seul `useQuery` par ligne ouverte : tant que la ligne l'est, ses colonnes
 * restent montées et le changement de candidature ne les démonte pas. */
export function ThemeExcerpts({
  theme,
  candidates,
}: {
  theme: string
  /** Candidatures des cases remplies, dans l'ordre des cases. */
  candidates: CandidateDto[]
}) {
  const ids = candidates.map((candidate) => candidate.id).join(',')
  const query = useQuery<CandidateComparisonResponse>({
    queryKey: ['candidats-2027', 'extraits', theme, ids],
    queryFn: () =>
      fetchJson<CandidateComparisonResponse>(
        `/api/candidats-2027?${new URLSearchParams({ theme, candidats: ids })}`,
      ),
    enabled: ids !== '',
    // Quand seules les candidatures changent, les colonnes déjà chargées
    // restent affichées pendant que la requête suivante part.
    placeholderData: keepPreviousData,
    staleTime: FIVE_MINUTES,
    retry: false,
  })

  if (candidates.length === 0) {
    return (
      <p className="px-4 py-5 text-sm leading-relaxed text-ink-soft">
        Choisissez une candidature pour lire ses extraits sur ce thème.
      </p>
    )
  }

  const data = query.data
  if (query.isError && !data) {
    return (
      <p role="alert" className="px-4 py-5 text-sm leading-relaxed text-no">
        Les extraits ne se chargent pas pour le moment
        {query.error instanceof Error ? ` (${query.error.message})` : ''}.
      </p>
    )
  }

  // Pendant le remplacement d'une candidature, la réponse conservée ne couvre
  // pas la nouvelle : sa colonne attend la sienne au lieu d'afficher, à tort,
  // qu'elle n'a aucun extrait.
  const covered = new Set(data?.selected.map((candidate) => candidate.id))
  const refreshing = query.isPlaceholderData

  return (
    <div
      aria-busy={query.isFetching}
      className={`grid gap-px bg-line transition-opacity ${refreshing ? 'opacity-60' : ''}`}
      style={{ gridTemplateColumns: `repeat(${candidates.length}, minmax(0, 1fr))` }}
    >
      {candidates.map((candidate) => {
        const proposals = data?.proposals.filter((proposal) => proposal.candidate_id === candidate.id) ?? []
        const ready = covered.has(candidate.id)
        return (
          <section
            key={candidate.id}
            aria-label={candidate.display_name}
            className="flex min-w-0 flex-col gap-3 bg-surface p-3 md:p-4"
          >
            <div className="flex items-center gap-2">
              <CandidatePortrait candidate={candidate} className="size-6" />
              <h5 className="min-w-0 text-[13px] leading-tight font-semibold [overflow-wrap:anywhere] text-ink">
                {candidate.display_name}
              </h5>
            </div>
            {!ready ? (
              <p className="animate-pulse text-[13px] text-ink-faint">Chargement…</p>
            ) : proposals.length === 0 ? (
              <p className="text-[13px] leading-snug text-ink-soft">
                Aucun extrait référencé sur ce thème.
              </p>
            ) : (
              <ul className="space-y-3">
                {proposals.map((proposal, index) => (
                  <li
                    key={`${proposal.source_url}-${index}`}
                    className="border-l-2 border-line pl-2.5 md:pl-3"
                  >
                    <p className="text-[13.5px] leading-[1.45] text-ink">« {proposal.excerpt} »</p>
                    <p className="mt-1 text-xs leading-[1.3] text-ink-soft">
                      <a
                        className="inline-flex items-start gap-1 text-accent underline underline-offset-2 [overflow-wrap:anywhere]"
                        href={proposal.source_url}
                        target="_blank"
                        rel="noreferrer"
                      >
                        <Icon name="external" className="mt-px size-3" strokeWidth={2} />
                        <span className="line-clamp-2" title={proposal.source_label}>{proposal.source_label}</span>
                      </a>
                      {proposal.source_published_on && ` · ${formatDate(proposal.source_published_on)}`}
                    </p>
                  </li>
                ))}
              </ul>
            )}
          </section>
        )
      })}
    </div>
  )
}
