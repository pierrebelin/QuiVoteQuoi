import { keepPreviousData, useQuery } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { excerptSourceUrl, organizationLabel } from '../../lib/candidates'
import { fetchJson } from '../../lib/fetchJson'
import type {
  CandidateComparisonResponse,
  CandidateDto,
  CandidateProgramProposalDto,
} from '../../types/candidates'
import { formatDate } from '../../types/scrutins'
import { Icon } from '../icons'
import { Loading } from '../ui'
import { CandidatePortrait } from './CandidateIdentity'

const FIVE_MINUTES = 5 * 60_000
const FOCUS =
  'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'
/** Portraits empilés sur la ligne repliée des candidatures sans extrait. */
const STACKED_PORTRAITS = 5

/** Ce que la vue fait de la sélection du face-à-face, tenue par la page (URL). */
export interface FaceToFace {
  ids: Set<string>
  /** Plus de place : le bouton d'ajout est désactivé et le dit. */
  full: boolean
  max: number
  toggle: (candidateId: string) => void
}

/**
 * Un sujet, toutes les candidatures : une ligne repliée par candidature ayant
 * au moins un extrait sur ce sujet, dans l'ordre reçu (tiré au hasard par la
 * page), puis les candidatures sans extrait, regroupées en bas. Aucune n'est
 * écartée, et aucune ligne ne compte ses extraits : un compteur se lirait comme
 * un classement.
 */
export function SubjectCandidates({
  theme,
  candidates,
  faceToFace,
  wide,
}: {
  theme: string
  /** Toutes les candidatures, dans l'ordre d'affichage. */
  candidates: CandidateDto[]
  faceToFace: FaceToFace
  wide: boolean
}) {
  const query = useQuery<CandidateComparisonResponse>({
    queryKey: ['candidats-2027', 'sujet', theme],
    queryFn: () =>
      fetchJson<CandidateComparisonResponse>(`/api/candidats-2027?${new URLSearchParams({ theme })}`),
    // Changer de sujet garde la liste précédente, estompée, le temps du chargement.
    placeholderData: keepPreviousData,
    staleTime: FIVE_MINUTES,
    retry: false,
  })

  const data = query.data
  if (query.isLoading) return <Loading>Chargement des extraits…</Loading>
  if (query.isError && !data) {
    return (
      <p role="alert" className="rounded-xl border border-line bg-surface px-4 py-4 text-sm leading-relaxed text-no">
        Les extraits ne se chargent pas pour le moment
        {query.error instanceof Error ? ` (${query.error.message})` : ''}.
      </p>
    )
  }

  const byCandidate = new Map<string, CandidateProgramProposalDto[]>()
  for (const proposal of data?.proposals ?? []) {
    const list = byCandidate.get(proposal.candidate_id) ?? []
    list.push(proposal)
    byCandidate.set(proposal.candidate_id, list)
  }
  const withExcerpt = candidates.filter((candidate) => byCandidate.has(candidate.id))
  const without = candidates.filter((candidate) => !byCandidate.has(candidate.id))

  return (
    <div
      aria-busy={query.isFetching}
      className={`transition-opacity ${query.isPlaceholderData ? 'opacity-60' : ''}`}
    >
      <p className="mb-2.5 text-[13px] leading-[18px] text-ink-soft">
        {withExcerpt.length === 1
          ? `1 candidature sur ${candidates.length} a un extrait sur ce sujet`
          : `${withExcerpt.length} candidatures sur ${candidates.length} ont un extrait sur ce sujet`}
        {' · ordre tiré au hasard à chaque visite'}
      </p>

      {withExcerpt.length > 0 && (
        <ul className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card">
          {withExcerpt.map((candidate) => (
            // La clé porte le sujet : une ligne ouverte se referme quand il change.
            <CandidateExcerptRow
              key={`${theme}-${candidate.id}`}
              candidate={candidate}
              proposals={byCandidate.get(candidate.id) ?? []}
              faceToFace={faceToFace}
            />
          ))}
        </ul>
      )}

      {without.length > 0 &&
        (wide ? <WithoutExcerptPills candidates={without} /> : <WithoutExcerptGroup key={theme} candidates={without} />)}
    </div>
  )
}

function withoutLabel(count: number) {
  return count === 1
    ? '1 candidature sans extrait sur ce sujet'
    : `${count} candidatures sans extrait sur ce sujet`
}

function CandidateExcerptRow({
  candidate,
  proposals,
  faceToFace,
}: {
  candidate: CandidateDto
  proposals: CandidateProgramProposalDto[]
  faceToFace: FaceToFace
}) {
  const [open, setOpen] = useState(false)
  const buttonId = useId()
  const panelId = useId()
  const selected = faceToFace.ids.has(candidate.id)
  const organization = organizationLabel(candidate)
  const first = proposals[0]

  return (
    <li>
      <h4>
        <button
          id={buttonId}
          type="button"
          aria-expanded={open}
          aria-controls={panelId}
          onClick={() => setOpen((value) => !value)}
          className={`flex w-full gap-2.5 px-3.5 py-3 text-left transition-colors hover:bg-surface-soft ${FOCUS} ${
            open ? 'items-center bg-surface-soft' : 'items-start'
          }`}
        >
          <CandidatePortrait candidate={candidate} className="size-9" />
          <span className="block min-w-0 flex-1">
            <span className="block text-[15px] leading-[19px] font-semibold text-ink">
              {candidate.display_name}
              {selected && (
                <span className="ml-1.5 inline-flex items-center gap-0.5 align-[1px] text-[11px] font-semibold text-accent">
                  <Icon name="check" className="size-3" strokeWidth={3} />
                  Face-à-face
                </span>
              )}
            </span>
            {organization && (
              <span className="block truncate text-[12.5px] leading-4 font-normal text-ink-soft">{organization}</span>
            )}
            {!open && first && (
              <span className="mt-1 line-clamp-2 text-sm leading-[21px] font-normal text-ink">
                « {first.excerpt} »
              </span>
            )}
          </span>
          <Icon
            name="down"
            className={`size-[18px] shrink-0 text-ink-soft transition-transform ${open ? 'rotate-180' : 'mt-2.5'}`}
          />
        </button>
      </h4>
      {open && (
        <div
          id={panelId}
          role="region"
          aria-labelledby={buttonId}
          className="flex flex-col gap-3 border-t border-line px-3.5 pt-3 pb-3.5"
        >
          {proposals.map((proposal, index) => (
            <figure key={`${proposal.source_url}-${index}`} className="border-l-2 border-line pl-3">
              <p className="text-sm leading-[1.5] text-ink">« {proposal.excerpt} »</p>
              <figcaption className="mt-1 text-xs leading-[1.3] text-ink-soft">
                <a
                  className="inline-flex items-start gap-1 text-accent underline underline-offset-2 [overflow-wrap:anywhere]"
                  href={excerptSourceUrl(proposal.source_url, proposal.excerpt)}
                  target="_blank"
                  rel="noreferrer"
                >
                  <Icon name="external" className="mt-px size-3 shrink-0" strokeWidth={2} />
                  <span>{proposal.source_label}</span>
                </a>
                {proposal.source_published_on && ` · ${formatDate(proposal.source_published_on)}`}
              </figcaption>
            </figure>
          ))}
          <FaceToFaceButton candidate={candidate} faceToFace={faceToFace} />
        </div>
      )}
    </li>
  )
}

function FaceToFaceButton({ candidate, faceToFace }: { candidate: CandidateDto; faceToFace: FaceToFace }) {
  const selected = faceToFace.ids.has(candidate.id)
  const blocked = !selected && faceToFace.full
  return (
    <div>
      <button
        type="button"
        disabled={blocked}
        aria-pressed={selected}
        onClick={() => faceToFace.toggle(candidate.id)}
        className={`flex h-10 w-full items-center justify-center gap-1.5 rounded-[10px] border border-line text-[13.5px] font-semibold transition-colors md:w-auto md:px-4 ${FOCUS} ${
          blocked ? 'cursor-not-allowed text-ink-faint' : 'text-ink hover:bg-surface-soft'
        }`}
      >
        <Icon name={selected ? 'close' : 'add'} className="size-4" />
        {selected ? 'Retirer du face-à-face' : 'Ajouter au face-à-face'}
      </button>
      {blocked && (
        <p className="mt-1.5 text-xs leading-snug text-ink-soft">
          Le face-à-face compte déjà {faceToFace.max} candidatures : retirez-en une pour ajouter celle-ci.
        </p>
      )}
    </div>
  )
}

/** Téléphone : une ligne repliée, portraits empilés ; dépliée, les noms. */
function WithoutExcerptGroup({ candidates }: { candidates: CandidateDto[] }) {
  const [open, setOpen] = useState(false)
  const panelId = useId()
  return (
    <div className="mt-3 overflow-hidden rounded-xl border border-line bg-surface shadow-card">
      <button
        type="button"
        aria-expanded={open}
        aria-controls={panelId}
        onClick={() => setOpen((value) => !value)}
        className={`flex w-full items-center gap-2.5 px-3.5 py-3 text-left transition-colors hover:bg-surface-soft ${FOCUS}`}
      >
        <span className="flex shrink-0">
          {candidates.slice(0, STACKED_PORTRAITS).map((candidate, index) => (
            <span key={candidate.id} className={`block rounded-full ring-2 ring-surface ${index > 0 ? '-ml-2' : ''}`}>
              <CandidatePortrait candidate={candidate} className="size-[26px]" />
            </span>
          ))}
        </span>
        <span className="flex-1 text-sm leading-[18px] font-semibold text-ink">{withoutLabel(candidates.length)}</span>
        <Icon name="down" className={`size-[18px] shrink-0 text-ink-soft transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>
      {open && (
        <ul id={panelId} className="divide-y divide-line border-t border-line">
          {candidates.map((candidate) => (
            <li key={candidate.id} className="flex items-center gap-2.5 px-3.5 py-2">
              <CandidatePortrait candidate={candidate} className="size-7" />
              <span className="min-w-0">
                <span className="block text-sm leading-[18px] font-medium text-ink">{candidate.display_name}</span>
                {organizationLabel(candidate) && (
                  <span className="block truncate text-xs leading-4 text-ink-soft">{organizationLabel(candidate)}</span>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

/** Ordinateur : la place ne manque pas, les noms restent visibles en pastilles. */
function WithoutExcerptPills({ candidates }: { candidates: CandidateDto[] }) {
  return (
    <section className="mt-4">
      <h4 className="mb-2 text-sm font-semibold text-ink">{withoutLabel(candidates.length)}</h4>
      <ul className="flex flex-wrap gap-1.5">
        {candidates.map((candidate) => (
          <li
            key={candidate.id}
            className="inline-flex items-center gap-1.5 rounded-full border border-line bg-surface py-1 pr-3 pl-1 text-[13px] text-ink"
          >
            <CandidatePortrait candidate={candidate} className="size-[22px]" />
            {candidate.display_name}
          </li>
        ))}
      </ul>
    </section>
  )
}
