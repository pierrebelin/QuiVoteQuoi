import { Link } from 'react-router'
import { excerptSourceUrl } from '../../lib/candidates'
import type { CandidateProgramProposalDto } from '../../types/candidates'
import { formatDate } from '../../types/scrutins'
import { Icon } from '../icons'

/** Un extrait cité tel quel, avec sa source : jamais une synthèse. */
export function ProgramExcerpt({ proposal }: { proposal: CandidateProgramProposalDto }) {
  return (
    <figure className="border-l-2 border-line-strong pl-3 md:max-w-[760px]">
      <p className="text-[15px] leading-[1.5] text-pretty text-ink">« {proposal.excerpt} »</p>
      <figcaption className="mt-1.5 text-[13px] leading-[1.35] text-ink-soft">
        <a
          className="inline-flex items-start gap-1 link [overflow-wrap:anywhere]"
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
  )
}

/** Ouvre l'onglet Par candidature sur une candidature : tout son programme,
 * rangé par sujet. */
export function ProgramLink({ to }: { to: string }) {
  return (
    <Link
      to={to}
      className="flex h-11 w-full items-center justify-center gap-1.5 rounded-xl bg-ink px-4 text-sm font-semibold text-surface transition-colors hover:bg-ink/90 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent md:inline-flex md:w-auto"
    >
      Tout son programme
      <Icon name="chevron" className="size-4" />
    </Link>
  )
}
