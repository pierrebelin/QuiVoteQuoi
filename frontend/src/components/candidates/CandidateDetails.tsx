import { Link } from 'react-router'
import { PORTRAITS } from '../../content/portraits'
import type { CandidateDto, CandidateParliamentaryGroupDto } from '../../types/candidates'
import { formatDate } from '../../types/scrutins'
import { Icon } from '../icons'
import { SectionTitle } from '../ui'
import { CandidateLogo, CandidatePortrait } from './CandidateIdentity'

const LINK = 'text-accent underline underline-offset-2'

/** Déclaration publique, organisations sourcées, site et programme : ce que
 * README.md §8.2 exige de publier pour chaque candidature. */
function CandidateSources({ candidate }: { candidate: CandidateDto }) {
  return (
    <div className="space-y-1.5 text-[13px] leading-relaxed text-ink-soft">
      <p>
        Candidature déclarée le {formatDate(candidate.declared_on)} ·{' '}
        <a className={LINK} href={candidate.declaration_source_url} target="_blank" rel="noreferrer">
          {candidate.declaration_source_label}
        </a>
      </p>
      {candidate.political_organizations.length > 0 && (
        <p>
          Parti ou soutien déclaré :{' '}
          {candidate.political_organizations.map((organization, index) => (
            <span key={organization.source_url}>
              {index > 0 && ' · '}
              {organization.official_url ? (
                <a className={LINK} href={organization.official_url} target="_blank" rel="noreferrer">
                  {organization.label}
                </a>
              ) : (
                organization.label
              )}{' '}
              (
              <a className={LINK} href={organization.source_url} target="_blank" rel="noreferrer">
                source
              </a>
              )
            </span>
          ))}
        </p>
      )}
      {(candidate.official_site_url || candidate.program_url) && (
        <div className="flex flex-wrap gap-x-4 gap-y-1">
          {candidate.official_site_url && (
            <a className={LINK} href={candidate.official_site_url} target="_blank" rel="noreferrer">
              Site officiel
            </a>
          )}
          {candidate.program_url && (
            <a className={LINK} href={candidate.program_url} target="_blank" rel="noreferrer">
              Lire le programme
            </a>
          )}
        </div>
      )}
      {PORTRAITS[candidate.id]?.year && (
        <p className="text-xs text-ink-faint">Photo de {PORTRAITS[candidate.id].year}</p>
      )}
    </div>
  )
}

/** Groupes associés par une source, avec le lien vers leurs votes (au thème
 * ouvert, le cas échéant). Un groupe n'apparaît que si une source le relie. */
function GroupLinks({
  groups,
  failed,
  theme,
}: {
  /** `null` tant que la réponse n'est pas arrivée. */
  groups: CandidateParliamentaryGroupDto[] | null
  /** La requête a échoué : dire que les groupes manquent, pas qu'il n'y en a pas. */
  failed: boolean
  theme: string
}) {
  if (groups === null) {
    return failed ? (
      <p role="alert" className="text-[13px] leading-relaxed text-no">
        Les groupes associés ne se chargent pas pour le moment.
      </p>
    ) : (
      <p className="animate-pulse text-[13px] text-ink-faint">Chargement des groupes associés…</p>
    )
  }
  if (groups.length === 0) {
    return (
      <p className="text-[13px] leading-relaxed text-ink-soft">
        Aucun groupe parlementaire associé n’est encore référencé.
      </p>
    )
  }
  return (
    <ul className="space-y-2 text-[13px] leading-relaxed text-ink-soft">
      {groups.map((group) => {
        const query = new URLSearchParams({ groupes: group.abbrev })
        if (theme) query.set('theme', theme)
        return (
          <li key={group.group_uid}>
            <Link className={`font-medium ${LINK}`} to={`/votes-par-groupe?${query}`}>
              Voir les votes du groupe {group.abbrev}
            </Link>
            <span> · {group.label} · association sourcée le {formatDate(group.linked_on)} · </span>
            <a className={LINK} href={group.source_url} target="_blank" rel="noreferrer">
              {group.source_label}
            </a>
          </li>
        )
      })}
    </ul>
  )
}

/**
 * Tout ce qui concerne une candidature, regroupé dans une section dépliable :
 * la déclaration et sa source, les organisations sourcées, le lien vers le
 * programme et les groupes parlementaires associés.
 */
export function CandidateDetails({
  candidate,
  groups,
  groupsFailed,
  theme,
}: {
  candidate: CandidateDto
  groups: CandidateParliamentaryGroupDto[] | null
  groupsFailed: boolean
  theme: string
}) {
  return (
    <details className="group bg-surface">
      <summary className="flex min-h-14 cursor-pointer list-none items-center gap-3 px-3.5 py-2 transition-colors marker:hidden hover:bg-surface-soft focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent [&::-webkit-details-marker]:hidden">
        <CandidatePortrait candidate={candidate} className="size-10" />
        <span className="min-w-0 flex-1">
          <span className="block text-[15px] leading-tight font-semibold text-ink">
            {candidate.display_name}
          </span>
          <span className="block text-[12.5px] leading-tight text-ink-soft">
            Déclaration, sources et groupes associés
          </span>
        </span>
        <CandidateLogo candidate={candidate} className="hidden h-8 w-12 rounded p-0.5 sm:flex" />
        <Icon
          name="chevron"
          className="size-[18px] text-ink-soft transition-transform group-open:rotate-90"
        />
      </summary>
      <div className="space-y-4 border-t border-line px-4 py-4">
        <CandidateSources candidate={candidate} />
        <section>
          <SectionTitle>Votes des groupes associés</SectionTitle>
          <GroupLinks groups={groups} failed={groupsFailed} theme={theme} />
        </section>
      </div>
    </details>
  )
}
