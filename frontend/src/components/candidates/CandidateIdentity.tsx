import { PoliticalLogo } from '../PoliticalLogo'
import { PARTIES } from '../../content/logos'
import { PORTRAITS } from '../../content/portraits'
import type { CandidateDto } from '../../types/candidates'

/** Identité visuelle du premier parti déclaré : une candidature sans parti
 * sourcé n'en reçoit aucune, plutôt qu'une appartenance déduite. */
function partyOf(candidate: CandidateDto) {
  const organization = candidate.political_organizations[0]
  return organization ? PARTIES[organization.label] : undefined
}

/** Sans logo référencé, pas de plaque : un sigle reconstitué pour un parti
 * que l'Assemblée ne nomme pas serait une invention du site. */
export function CandidateLogo({ candidate, className }: { candidate: CandidateDto; className?: string }) {
  const party = partyOf(candidate)
  if (!party) return null
  return <PoliticalLogo logo={party.logo} fallback="" color={party.color} className={className} />
}

/** Portrait libre de la candidature, ou silhouette neutre : la liste garde le
 * même alignement, qu'une photo existe ou non. */
export function CandidatePortrait({ candidate, className }: { candidate: CandidateDto; className: string }) {
  const portrait = PORTRAITS[candidate.id]
  return (
    <span aria-hidden className={`flex shrink-0 items-center justify-center overflow-hidden rounded-full bg-surface-soft ring-1 ring-line ${className}`}>
      {portrait ? (
        <img src={portrait.src} alt="" loading="lazy" className="size-full object-cover object-[50%_20%]" />
      ) : (
        <svg viewBox="0 0 24 24" fill="currentColor" className="size-3/4 translate-y-[12%] text-ink-faint/50">
          <circle cx="12" cy="8" r="4.5" />
          <path d="M3 22c0-5 4-8.5 9-8.5s9 3.5 9 8.5z" />
        </svg>
      )}
    </span>
  )
}
