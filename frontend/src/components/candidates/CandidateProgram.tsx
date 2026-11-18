import { useQuery } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'
import { Link } from 'react-router'
import { organizationLabel } from '../../lib/candidates'
import { fetchJson } from '../../lib/fetchJson'
import type {
  CandidateComparisonResponse,
  CandidateDto,
  CandidateProgramProposalDto,
} from '../../types/candidates'
import type { FamilyDto } from '../../types/themes'
import { BottomSheet } from '../BottomSheet'
import { FamilyIllustration } from '../FamilyIllustration'
import { Icon } from '../icons'
import { ErrorPanel, Loading, SectionTitle } from '../ui'
import { CandidateSources, GroupLinks } from './CandidateDetails'
import { CandidatePortrait } from './CandidateIdentity'
import { CandidatePicker } from './CandidateSlots'
import { ProgramExcerpt } from './ProgramExcerpt'
import { FaceToFaceButton, type FaceToFace } from './SubjectCandidates'

const FIVE_MINUTES = 5 * 60_000
const FOCUS =
  'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

/**
 * Onglet Par candidature, sans candidature choisie : la liste, dans l'ordre
 * tiré au hasard de la visite (aucune n'est toujours en tête), puis les
 * candidatures sans extrait, nommées en une ligne.
 */
export function CandidateChooser({
  candidates,
  programHref,
}: {
  /** Toutes les candidatures, dans l'ordre d'affichage. */
  candidates: CandidateDto[]
  programHref: (candidateId: string) => string
}) {
  const withExcerpts = candidates.filter((candidate) => candidate.has_program_excerpts)
  const without = candidates.filter((candidate) => !candidate.has_program_excerpts)
  return (
    <section className="md:max-w-2xl">
      <SectionTitle>Choisir une candidature</SectionTitle>
      <ul className="divide-y divide-line overflow-hidden rounded border border-line bg-surface shadow-card">
        {withExcerpts.map((candidate) => (
          <li key={candidate.id}>
            <Link
              to={programHref(candidate.id)}
              className={`grid min-h-[60px] grid-cols-[40px_minmax(0,1fr)_18px] items-center gap-3 px-3.5 py-2 transition-colors hover:bg-surface-soft ${FOCUS}`}
            >
              <CandidatePortrait candidate={candidate} className="size-10" />
              <span className="min-w-0">
                <span className="block text-[15px] leading-[19px] font-semibold text-ink">
                  {candidate.display_name}
                </span>
                {organizationLabel(candidate) && (
                  <span className="block truncate text-[12.5px] leading-4 text-ink-soft">
                    {organizationLabel(candidate)}
                  </span>
                )}
              </span>
              <Icon name="chevron" className="size-[18px] text-ink-soft" />
            </Link>
          </li>
        ))}
      </ul>
      {without.length > 0 && (
        <p className="mt-3.5 text-[13px] leading-relaxed text-ink-soft">
          Aucun extrait de programme relevé pour :{' '}
          {without.map((candidate) => candidate.display_name).join(', ')}.
        </p>
      )}
    </section>
  )
}

/**
 * Onglet Par candidature : tout ce qu'une candidature a déclaré, rangé par
 * sujet et entièrement déplié. Les sujets sans extrait sont nommés en une
 * ligne, sans compteur : un nombre d'extraits se lirait comme un classement.
 * À partir de 768 px, l'identité et le sommaire des sujets passent à gauche.
 */
export function CandidateProgram({
  candidate,
  candidates,
  families,
  faceToFace,
  onPick,
}: {
  candidate: CandidateDto
  /** Toutes les candidatures, dans l'ordre alphabétique du tiroir. */
  candidates: CandidateDto[]
  families: FamilyDto[]
  faceToFace: FaceToFace
  onPick: (candidateId: string) => void
}) {
  const [picking, setPicking] = useState(false)
  const rootRef = useRef<HTMLDivElement>(null)

  // Le composant est remonté à chaque candidature (clé posée par la page) : la
  // nouvelle se lit depuis son identité, pas au milieu des extraits de la précédente.
  useEffect(() => {
    const root = rootRef.current
    if (root && root.getBoundingClientRect().top < 0) root.scrollIntoView({ block: 'start' })
  }, [])

  // Sans thème, l'API renvoie tous les extraits de la candidature et ses groupes.
  const query = useQuery<CandidateComparisonResponse>({
    queryKey: ['candidats-2027', 'groupes', candidate.id],
    queryFn: () =>
      fetchJson<CandidateComparisonResponse>(
        `/api/candidats-2027?${new URLSearchParams({ candidats: candidate.id })}`,
      ),
    staleTime: FIVE_MINUTES,
    retry: false,
  })

  const byFamily = new Map<string, CandidateProgramProposalDto[]>()
  for (const proposal of query.data?.proposals ?? []) {
    if (proposal.candidate_id !== candidate.id) continue
    byFamily.set(proposal.theme_code, [...(byFamily.get(proposal.theme_code) ?? []), proposal])
  }
  const covered = families.filter((family) => byFamily.has(family.code))
  const uncovered = families.filter((family) => !byFamily.has(family.code))
  const groups =
    query.data?.parliamentary_groups.filter((group) => group.candidate_id === candidate.id) ?? null

  return (
    <div
      ref={rootRef}
      className="scroll-mt-20 md:grid md:grid-cols-[minmax(0,260px)_minmax(0,1fr)] md:items-start md:gap-6 lg:grid-cols-[300px_minmax(0,1fr)]"
    >
      <aside className="mb-5 md:mb-0">
        <div className="rounded border border-line bg-surface px-4 py-4 shadow-card">
          <div className="flex items-center gap-3">
            <CandidatePortrait candidate={candidate} className="size-12 md:size-14" />
            <h3 className="min-w-0 flex-1 text-base leading-tight font-semibold text-ink md:heading-5">
              {candidate.display_name}
            </h3>
            <button
              type="button"
              aria-haspopup="dialog"
              onClick={() => setPicking(true)}
              className={`flex min-h-11 shrink-0 items-center gap-1 rounded px-2 text-[13.5px] font-semibold text-ink transition-colors hover:bg-surface-soft md:hidden ${FOCUS}`}
            >
              Changer
              <Icon name="down" className="size-4 text-ink-soft" />
            </button>
          </div>
          <div className="mt-3">
            <CandidateSources candidate={candidate} />
          </div>
          <div className="mt-3.5 flex flex-col gap-2">
            <FaceToFaceButton candidate={candidate} faceToFace={faceToFace} />
            <button
              type="button"
              aria-haspopup="dialog"
              onClick={() => setPicking(true)}
              className={`hidden h-10 items-center justify-center rounded bg-surface-soft px-4 text-[13.5px] font-semibold text-ink transition-colors hover:bg-line md:flex ${FOCUS}`}
            >
              Changer de candidature
            </button>
          </div>
        </div>

        {covered.length > 0 && (
          <nav aria-label="Sujets du programme" className="mt-5 hidden md:block">
            <SectionTitle>Sujets</SectionTitle>
            <ul className="space-y-0.5">
              {covered.map((family) => (
                <li key={family.code}>
                  <a
                    href={`#programme-${family.code}`}
                    className={`flex min-h-9 items-center gap-2 rounded px-2 py-1 text-[13.5px] font-medium text-ink transition-colors hover:bg-surface-soft ${FOCUS}`}
                  >
                    <FamilyIllustration
                      code={family.code}
                      className="size-6 shrink-0 rounded"
                      glyphClassName="size-3.5"
                    />
                    {family.label}
                  </a>
                </li>
              ))}
            </ul>
          </nav>
        )}
      </aside>

      <section className="min-w-0" aria-label={`Programme de ${candidate.display_name}, par sujet`}>
        {query.isLoading && <Loading>Chargement du programme…</Loading>}
        {query.isError && <ErrorPanel error={query.error} />}
        {query.data && covered.length > 0 && (
          <div className="divide-y divide-line overflow-hidden rounded border border-line bg-surface shadow-card">
            {covered.map((family) => (
              <section
                key={family.code}
                id={`programme-${family.code}`}
                aria-labelledby={`programme-${family.code}-titre`}
                className="scroll-mt-20 px-4 pt-4 pb-4 md:px-6 md:pt-5"
              >
                <h4
                  id={`programme-${family.code}-titre`}
                  className="mb-3 flex items-center gap-2.5 font-heading text-[17px] leading-[22px] font-[460] text-ink md:text-xl md:leading-6"
                >
                  <FamilyIllustration
                    code={family.code}
                    className="size-7 shrink-0 rounded md:size-8"
                    glyphClassName="size-4 md:size-[18px]"
                  />
                  {family.label}
                </h4>
                <div className="flex max-w-[720px] flex-col gap-3">
                  {(byFamily.get(family.code) ?? []).map((proposal, index) => (
                    <ProgramExcerpt key={`${proposal.source_url}-${index}`} proposal={proposal} />
                  ))}
                </div>
              </section>
            ))}
          </div>
        )}
        {query.data && uncovered.length > 0 && (
          <p className="mt-3.5 text-[13px] leading-relaxed text-ink-soft">
            {covered.length === 0
              ? 'Aucun extrait de programme relevé pour cette candidature.'
              : `Aucun extrait du programme relevé sur : ${uncovered.map((family) => family.label).join(' · ')}.`}
          </p>
        )}

        <section className="mt-6">
          <SectionTitle>Votes des groupes associés</SectionTitle>
          <div className="rounded border border-line bg-surface px-4 py-3.5 shadow-card">
            <GroupLinks groups={groups} failed={query.isError} theme="" />
          </div>
        </section>
      </section>

      <BottomSheet open={picking} onClose={() => setPicking(false)} title="Choisir une candidature">
        <CandidatePicker
          candidates={candidates}
          current={candidate}
          taken={new Set()}
          onPick={(candidateId) => {
            onPick(candidateId)
            setPicking(false)
          }}
        />
      </BottomSheet>
    </div>
  )
}
