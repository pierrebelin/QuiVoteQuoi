import { useQuery } from '@tanstack/react-query'
import { Link, useSearchParams } from 'react-router'
import { PoliticalLogo } from '../components/PoliticalLogo'
import { Card, ErrorPanel, Loading, Note, PageHeader, SectionTitle } from '../components/ui'
import { PARTIES } from '../content/logos'
import { PORTRAITS } from '../content/portraits'
import { fetchJson } from '../lib/fetchJson'
import type {
  CandidateComparisonResponse,
  CandidateDto,
  CandidateParliamentaryGroupDto,
  CandidateProgramProposalDto,
} from '../types/candidates'
import type { FamiliesResponse } from '../types/themes'
import { formatDate } from '../types/scrutins'

/** Identité visuelle du premier parti déclaré : une candidature sans parti
 * sourcé n'en reçoit aucune, plutôt qu'une appartenance déduite. */
function partyOf(candidate: CandidateDto) {
  const organization = candidate.political_organizations[0]
  return organization ? PARTIES[organization.label] : undefined
}

/** Sans logo référencé, pas de plaque : un sigle reconstitué pour un parti
 * que l'Assemblée ne nomme pas serait une invention du site. */
function CandidateLogo({ candidate, className }: { candidate: CandidateDto; className?: string }) {
  const party = partyOf(candidate)
  if (!party) return null
  return <PoliticalLogo logo={party.logo} fallback="" color={party.color} className={className} />
}

/** Portrait libre de la candidature, ou silhouette neutre : la liste garde le
 * même alignement, qu'une photo existe ou non. */
function CandidatePortrait({ candidate, className }: { candidate: CandidateDto; className: string }) {
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

function CandidatePicker({
  candidates,
  selected,
  max,
  onToggle,
}: {
  candidates: CandidateDto[]
  selected: string[]
  max: number
  onToggle: (id: string) => void
}) {
  const full = selected.length >= max

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-xs font-medium text-ink-faint">
        Candidats comparés {selected.length}/{max}
      </span>
      {candidates.map((candidate) => {
        const isSelected = selected.includes(candidate.id)
        return (
          <button
            key={candidate.id}
            type="button"
            aria-pressed={isSelected}
            disabled={full && !isSelected}
            onClick={() => onToggle(candidate.id)}
            className={`flex items-center gap-2 rounded-full py-1 pr-1.5 pl-1 text-[13px] font-medium ring-1 ring-inset transition-colors ${
              isSelected
                ? 'bg-ink text-white ring-ink'
                : 'bg-surface text-ink-soft shadow-card ring-line hover:text-ink hover:ring-line-strong disabled:cursor-not-allowed disabled:opacity-45'
            }`}
          >
            <CandidatePortrait candidate={candidate} className="size-7" />
            {candidate.display_name}
            <CandidateLogo candidate={candidate} className="h-5 w-8 rounded p-0.5" />
          </button>
        )
      })}
    </div>
  )
}

function CandidateSources({ candidate }: { candidate: CandidateDto }) {
  return (
    <div className="mt-3 space-y-1.5 text-xs leading-relaxed text-ink-soft">
      <p>
        Candidature déclarée le {formatDate(candidate.declared_on)} ·{' '}
        <a className="text-accent underline" href={candidate.declaration_source_url} target="_blank" rel="noreferrer">
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
                <a className="text-accent underline" href={organization.official_url} target="_blank" rel="noreferrer">
                  {organization.label}
                </a>
              ) : (
                organization.label
              )}
              {' '}(
              <a className="text-accent underline" href={organization.source_url} target="_blank" rel="noreferrer">
                source
              </a>
              )
            </span>
          ))}
        </p>
      )}
      <div className="flex flex-wrap gap-x-3 gap-y-1">
        {candidate.official_site_url && (
          <a className="text-accent underline" href={candidate.official_site_url} target="_blank" rel="noreferrer">
            Site officiel
          </a>
        )}
        {candidate.program_url && (
          <a className="text-accent underline" href={candidate.program_url} target="_blank" rel="noreferrer">
            Lire le programme
          </a>
        )}
      </div>
    </div>
  )
}

function ProposalList({ proposals, selectedTheme }: { proposals: CandidateProgramProposalDto[]; selectedTheme: string }) {
  if (!selectedTheme) {
    return <p className="mt-3 text-sm leading-relaxed text-ink-faint">Choisissez un thème pour afficher les propositions qui y sont rattachées.</p>
  }
  if (proposals.length === 0) {
    return <p className="mt-3 text-sm leading-relaxed text-ink-faint">Aucun extrait de programme n’est encore référencé pour ce thème.</p>
  }
  return (
    <ul className="mt-3 space-y-3">
      {proposals.map((proposal, index) => (
        <li key={`${proposal.source_url}-${index}`} className="border-l-2 border-line pl-3 text-sm leading-relaxed text-ink-soft">
          <p>« {proposal.excerpt} »</p>
          <p className="mt-1 text-xs text-ink-faint">
            <a className="text-accent underline" href={proposal.source_url} target="_blank" rel="noreferrer">
              {proposal.source_label}
            </a>
            {proposal.source_published_on && ` · ${formatDate(proposal.source_published_on)}`}
          </p>
        </li>
      ))}
    </ul>
  )
}

function GroupLinks({ groups, theme }: { groups: CandidateParliamentaryGroupDto[]; theme: string }) {
  if (groups.length === 0) {
    return <p className="mt-3 text-sm leading-relaxed text-ink-faint">Aucun groupe parlementaire associé n’est encore référencé.</p>
  }
  return (
    <ul className="mt-3 space-y-2 text-sm text-ink-soft">
      {groups.map((group) => {
        const query = new URLSearchParams({ groupes: group.abbrev })
        if (theme) query.set('theme', theme)
        return (
          <li key={group.group_uid}>
            <Link className="font-medium text-accent underline" to={`/votes-par-groupe?${query}`}>
              Voir les votes du groupe {group.abbrev}
            </Link>
            <span> · {group.label} · association sourcée le {formatDate(group.linked_on)} · </span>
            <a className="text-accent underline" href={group.source_url} target="_blank" rel="noreferrer">
              {group.source_label}
            </a>
          </li>
        )
      })}
    </ul>
  )
}

/**
 * Comparaison factuelle : propositions attribuées à gauche, votes du groupe
 * explicitement associé accessibles à droite. Aucune comparaison n'est
 * transformée en score ou verdict.
 */
export default function CandidateComparisonPage() {
  const [params, setParams] = useSearchParams()
  const theme = params.get('theme') ?? ''
  const requested = (params.get('candidats') ?? '').split(',').map((value) => value.trim()).filter(Boolean)
  const query = new URLSearchParams()
  if (theme) query.set('theme', theme)
  if (requested.length > 0) query.set('candidats', requested.join(','))

  const candidates = useQuery<CandidateComparisonResponse>({
    queryKey: ['candidats-2027', query.toString()],
    queryFn: () => fetchJson<CandidateComparisonResponse>(`/api/candidats-2027?${query}`),
    retry: false,
  })
  const families = useQuery<FamiliesResponse>({
    queryKey: ['themes'],
    queryFn: () => fetchJson<FamiliesResponse>('/api/themes'),
    retry: false,
  })

  function update(next: { theme?: string; candidats?: string[] }) {
    const updated = new URLSearchParams()
    const nextTheme = next.theme ?? theme
    const nextCandidates = next.candidats ?? requested
    if (nextTheme) updated.set('theme', nextTheme)
    if (nextCandidates.length > 0) updated.set('candidats', nextCandidates.join(','))
    setParams(updated)
  }

  const data = candidates.data
  const selected = data?.selected ?? []
  const selectedIds = selected.map((candidate) => candidate.id)
  const max = data?.max_compared_candidates ?? 4

  function toggleCandidate(id: string) {
    const next = requested.includes(id)
      ? requested.filter((candidateId) => candidateId !== id)
      : [...requested, id].slice(0, max)
    update({ candidats: next })
  }

  return (
    <>
      <PageHeader
        title="Programmes et votes associés — 2027"
        lede="Comparez des extraits de programmes déclarés par thème, puis consultez les votes des groupes parlementaires explicitement associés à chaque candidature."
        aside={
          <label className="flex items-center gap-2 text-xs text-ink-faint">
            Thème
            <select
              value={theme}
              onChange={(event) => update({ theme: event.target.value })}
              className="rounded-lg border border-line bg-surface px-2.5 py-1.5 text-sm text-ink shadow-card focus:border-accent focus:ring-2 focus:ring-accent/15 focus:outline-none"
            >
              <option value="">Choisir un thème</option>
              {(families.data?.families ?? []).map((family) => <option key={family.code} value={family.code}>{family.label}</option>)}
            </select>
          </label>
        }
      />

      {candidates.isLoading && <Loading>Chargement des candidatures déclarées…</Loading>}
      {candidates.isError && <ErrorPanel error={candidates.error} />}
      {data && (
        <>
          {data.declaration_note && <Note>{data.declaration_note}</Note>}
          {data.candidates.length === 0 ? (
            <Card className="mt-4 px-5 py-5">
              <h3 className="text-base font-semibold">Aucune candidature déclarée n’est encore référencée</h3>
              <p className="mt-1 text-sm leading-relaxed text-ink-soft">La structure est prête : une candidature sera publiée avec sa déclaration source, son programme et les extraits rattachés aux thèmes.</p>
            </Card>
          ) : (
            <>
              <div className="mt-4"><CandidatePicker candidates={data.candidates} selected={selectedIds} max={max} onToggle={toggleCandidate} /></div>
              {(data.proposals_note || data.groups_note) && (
                <div className="mt-4 space-y-3">
                  {data.proposals_note && <Note>{data.proposals_note}</Note>}
                  {data.groups_note && <Note>{data.groups_note}</Note>}
                </div>
              )}
              {selected.length > 0 && (
                <section className={`mt-6 grid gap-4 ${selected.length > 1 ? 'lg:grid-cols-2' : ''}`}>
                  {selected.map((candidate) => (
                    <Card
                      key={candidate.id}
                      className={`px-4 py-4 sm:px-5 ${partyOf(candidate) ? 'border-t-4' : ''}`}
                      style={{ borderTopColor: partyOf(candidate)?.color }}
                    >
                      <div className="flex items-center gap-4">
                        <CandidatePortrait candidate={candidate} className="size-16" />
                        <div className="min-w-0">
                          <h3 className="text-lg font-semibold">{candidate.display_name}</h3>
                          {PORTRAITS[candidate.id]?.year && (
                            <p className="text-xs text-ink-faint">Photo de {PORTRAITS[candidate.id].year}</p>
                          )}
                        </div>
                        <CandidateLogo candidate={candidate} className="ml-auto h-12 w-20" />
                      </div>
                      <CandidateSources candidate={candidate} />
                      <section className="mt-5 border-t border-line pt-4">
                        <SectionTitle>Propositions pour ce thème</SectionTitle>
                        <ProposalList proposals={data.proposals.filter((proposal) => proposal.candidate_id === candidate.id)} selectedTheme={theme} />
                      </section>
                      <section className="mt-5 border-t border-line pt-4">
                        <SectionTitle>Votes des groupes associés</SectionTitle>
                        <GroupLinks groups={data.parliamentary_groups.filter((group) => group.candidate_id === candidate.id)} theme={theme} />
                      </section>
                    </Card>
                  ))}
                </section>
              )}
            </>
          )}
        </>
      )}
    </>
  )
}
