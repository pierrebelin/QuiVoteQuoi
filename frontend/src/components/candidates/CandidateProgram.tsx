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
import { Card, ErrorPanel, Loading, SectionTitle } from '../ui'
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
      <ul className="divide-y divide-line border-y border-line">
        {withExcerpts.map((candidate) => (
          <li key={candidate.id}>
            <Link
              to={programHref(candidate.id)}
              className={`group grid min-h-16 grid-cols-[44px_minmax(0,1fr)_18px] items-center gap-3 py-2.5 ${FOCUS}`}
            >
              <CandidatePortrait candidate={candidate} className="size-11" />
              <span className="min-w-0">
                <span className="block text-base leading-5 font-[620] text-ink">
                  {candidate.display_name}
                </span>
                {organizationLabel(candidate) && (
                  <span className="block truncate text-[13px] leading-4 text-ink-faint">
                    {organizationLabel(candidate)}
                  </span>
                )}
              </span>
              <Icon name="chevron" className="size-[18px] text-ink-faint transition-colors group-hover:text-ink" />
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
        <Card className="px-4 py-4">
          <div className="flex items-center gap-3">
            <CandidatePortrait candidate={candidate} className="size-12 md:size-14" />
            <h3 className="min-w-0 flex-1 heading-5 text-ink">
              {candidate.display_name}
            </h3>
            <button
              type="button"
              aria-haspopup="dialog"
              onClick={() => setPicking(true)}
              className={`flex min-h-11 shrink-0 items-center gap-1 rounded-xl px-2 text-sm font-semibold text-accent md:hidden ${FOCUS}`}
            >
              Changer
              <Icon name="down" className="size-4" strokeWidth={2} />
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
              className={`hidden h-11 items-center justify-center rounded-xl bg-canvas px-4 text-sm font-semibold text-ink transition-colors hover:bg-surface-soft md:flex ${FOCUS}`}
            >
              Changer de candidature
            </button>
          </div>
        </Card>

        {covered.length > 0 && (
          <nav aria-label="Sujets du programme" className="mt-5 hidden md:block">
            <div className="px-3">
              <SectionTitle>Sujets</SectionTitle>
            </div>
            <ul className="space-y-0.5">
              {covered.map((family) => (
                <li key={family.code}>
                  <a
                    href={`#programme-${family.code}`}
                    className={`flex min-h-11 items-center gap-3 rounded-xl px-3 py-1 text-sm font-medium text-ink-soft transition-colors hover:bg-surface hover:text-ink ${FOCUS}`}
                  >
                    <FamilyIllustration
                      bare
                      code={family.code}
                      className="size-5 shrink-0"
                      glyphClassName="size-5"
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
          <div className="divide-y divide-line border-y border-line">
            {covered.map((family) => (
              <section
                key={family.code}
                id={`programme-${family.code}`}
                aria-labelledby={`programme-${family.code}-titre`}
                className="scroll-mt-20 py-5 md:py-6"
              >
                <h4
                  id={`programme-${family.code}-titre`}
                  className="mb-3 flex items-center gap-2.5 text-[17px] leading-[22px] font-[620] tracking-[-0.01em] text-ink md:text-lg md:leading-6"
                >
                  <FamilyIllustration
                    bare
                    code={family.code}
                    className="size-6 shrink-0"
                    glyphClassName="size-6"
                  />
                  {family.label}
                </h4>
                <div className="flex max-w-[720px] flex-col gap-4">
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
          <div className="border-y border-line py-3.5">
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
