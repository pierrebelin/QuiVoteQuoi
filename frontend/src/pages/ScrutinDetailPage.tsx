import { useEffect, useRef, useState } from 'react'
import { Link, useParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { REJECTION_NOT_FINAL } from '../lib/dossierStatus'
import { FamilyIllustration } from '../components/FamilyIllustration'
import CountUp from '../components/CountUp'
import GuideLink from '../components/GuideLink'
import Hemicycle from '../components/Hemicycle'
import { PoliticalLogo } from '../components/PoliticalLogo'
import { InfoBox, InitiatorBox, InitiatorsBox } from '../components/InitiatorBox'
import VoteImageExport from '../components/VoteImageExport'
import { VoteCardHeader } from '../components/VoteCard'
import { FOCUS_RING } from '../components/entry/styles'
import { siegesDesGroupes } from '../components/sieges'
import { InfoTip } from '../components/InfoTip'
import { CoverageNote } from '../components/ScrutinList'
import {
  BackLink,
  Card,
  ErrorPanel,
  Loading,
  Note,
  Pill,
  SectionTitle,
  VoteBar,
} from '../components/ui'
import { GROUP_LOGOS } from '../content/logos'
import { fetchJson } from '../lib/fetchJson'
import { capitalized, themeLink, typographic } from '../lib/entryFlow'
import { amendmentCardPath } from '../types/amendments'
import { textKind, type InitiatorDto, type OutcomeDto } from '../types/dossiers'
import type {
  CitedAmendmentDto,
  Explanation,
  Explanations,
  GroupBreakdownDto,
  LeadingPosition,
  ScrutinDetailDto,
  TallyDto,
  VoteDto,
} from '../types/scrutins'
import {
  explanationsStateLabels,
  leadingPosition,
  positionClasses,
  positionLabels,
} from '../types/scrutins'

/** Ce que la page lit du dossier : qui l'a déposé, où il en est. */
interface DossierContextDto {
  procedure: string
  initiators: InitiatorDto[]
  outcome: OutcomeDto
}

function shortDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/**
 * Sort du texte, distinct de l'issue du scrutin : un texte adopté ici peut
 * encore être modifié ailleurs. Rien n'est affiché quand la source ne conclut
 * rien — « en cours » serait une affirmation que personne n'a écrite.
 */
/** Ce que le vote recouvre exactement, et ce qu'il ne recouvre pas. */
function ScrutinScopeHelp({ dossierUid }: { dossierUid: string | null }) {
  return (
    <>
      Le vote porte sur cet objet, pas nécessairement sur l'ensemble du texte.{' '}
      <GuideLink ancre="q-scrutin-loi">Comment lire un vote</GuideLink>
      {!dossierUid && (
        <>
          {' · '}La source ne rattache ce vote à aucun texte, comme la
          majorité des votes.{' '}
          <GuideLink ancre="q-sans-dossier">Pourquoi</GuideLink>
        </>
      )}
    </>
  )
}

function ExpressedVotesHelp() {
  return (
    <>
      Les abstentions n'entrent pas dans les suffrages exprimés, et
      « non-votant » ne se lit pas comme une absence.{' '}
      <GuideLink ancre="q-non-votant">Ce que recouvrent les positions</GuideLink>
    </>
  )
}

function HemicycleHelp() {
  return (
    <>
      Chaque député à sa place réelle (
      <a
        href="https://www.assemblee-nationale.fr/dyn/vos-deputes/hemicycle"
        target="_blank"
        rel="noreferrer"
        className="link"
      >
        plan de l'hémicycle
      </a>
      ).{' '}
      {/* Au doigt, pas de survol : la phrase sur la lecture d'un siège disparaît. */}
      <span className="[@media(hover:none)]:hidden">
        Survolez un siège pour lire le nom, le groupe et la position ; en gris,
        aucune position enregistrée.
      </span>
      <span className="[@media(hover:hover)]:hidden">En gris, aucune position enregistrée.</span>
    </>
  )
}

function OutcomeBox({ outcome, dossierUid }: { outcome: OutcomeDto; dossierUid: string }) {
  if (outcome.kind === 'no_recorded_conclusion') return null
  const tone =
    outcome.kind === 'promulgated'
      ? 'bg-yes-soft text-yes'
      : outcome.kind === 'rejected'
        ? 'bg-info-soft text-info'
        : 'bg-surface text-ink-soft ring-1 ring-inset ring-line'
  return (
    <InfoBox
      visual={
        <span className={`flex size-11 shrink-0 items-center justify-center rounded ${tone}`}>
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={1.8}
            strokeLinecap="round"
            strokeLinejoin="round"
            aria-hidden
            className="size-6"
          >
            <path d="M12 3v18M7 21h10M5 7h14M5 7l-2.5 6a2.5 2.5 0 0 0 5 0zM19 7l-2.5 6a2.5 2.5 0 0 0 5 0z" />
          </svg>
        </span>
      }
    >
      <p className="text-xs text-ink-faint">Où en est le texte</p>
      <p className="font-semibold">
        {outcome.label}
        {outcome.law_code && ` : loi n° ${outcome.law_code}`}
        {outcome.kind === 'rejected' && (
          <span className="font-normal text-ink-soft"> · {REJECTION_NOT_FINAL}</span>
        )}
      </p>
      <p className="text-xs">
        {outcome.date && <span className="text-ink-faint">{shortDate(outcome.date)} · </span>}
        <Link to={`/dossiers/${dossierUid}`} className="font-medium link">
          Voir le texte et son parcours →
        </Link>
      </p>
    </InfoBox>
  )
}

/**
 * L'amendement mis aux voix : numéro, article visé, signataire aux couleurs
 * publiées de son groupe au dépôt (RM-10), puis l'exposé en citation. Un
 * exposé long s'ouvre sur son aperçu ; le texte déplié reste verbatim (RM-03).
 */
function CitedAmendment({
  amendment,
  methodLabels,
}: {
  amendment: CitedAmendmentDto
  methodLabels: string[]
}) {
  const [open, setOpen] = useState(false)
  const collapsible = amendment.summary_preview_truncated && amendment.summary_preview != null

  return (
    <div>
      <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
        <span className="text-xs text-ink-faint">
          Amendement <span className="font-mono">n° {amendment.number}</span>
        </span>
        <span className="text-[15px] font-semibold text-ink">{amendment.target_title}</span>
      </div>

      {amendment.author_name && (
        <p className="mt-1 text-sm text-ink-soft">
          <span className="font-medium text-ink">{amendment.author_name}</span>
          {amendment.author_group_abbrev &&
            (amendment.author_group_color ? (
              <span
                className="ml-1.5 rounded px-1.5 py-0.5 text-xs font-semibold ring-1 ring-inset ring-badge-edge"
                style={{
                  backgroundColor: amendment.author_group_color,
                  color: amendment.author_text_tone === 'white' ? '#ffffff' : '#000000',
                }}
              >
                {amendment.author_group_abbrev}
              </span>
            ) : (
              <span className="ml-1.5">({amendment.author_group_abbrev})</span>
            ))}
          {amendment.cosignatory_count > 0 && (
            <span className="text-ink-faint">
              {' '}
              et {amendment.cosignatory_count} cosignataire
              {amendment.cosignatory_count > 1 ? 's' : ''}
            </span>
          )}
        </p>
      )}

      {amendment.summary ? (
        <blockquote className="mt-3 max-w-3xl border-l-[3px] border-line-strong pl-3 text-sm leading-relaxed text-ink-soft">
          <span className="mb-1 block text-xs text-ink-faint">
            Exposé sommaire de l’auteur, repris tel quel
          </span>
          <p className="whitespace-pre-line">
            {collapsible && !open ? amendment.summary_preview : amendment.summary}
            {collapsible && (
              <button
                type="button"
                onClick={() => setOpen(!open)}
                className={`ml-1 text-xs font-medium link ${FOCUS_RING}`}
              >
                {open ? 'Replier' : 'Lire la suite'}
              </button>
            )}
          </p>
        </blockquote>
      ) : (
        <p className="mt-3 text-xs text-ink-faint">
          La source ne publie pas d'exposé sommaire pour cet amendement.
        </p>
      )}

      <div className="mt-3 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1 text-xs">
        <span className="text-ink-faint">Lien établi par : {methodLabels.join(' + ')}</span>
        {/* Pas de page officielle de l'amendement (AMENDEMENTS H8) :
            le renvoi mène à sa carte, dans son dossier. */}
        <Link
          to={amendmentCardPath(amendment.dossier_uid, amendment.number)}
          className="font-medium link"
        >
          Voir l'amendement dans son texte →
        </Link>
      </div>
    </div>
  )
}

/** Version du compte rendu, telle que publiée par la source, sans libellé inventé (RM-11). */
/** Version du compte rendu dite en clair : la source publie « avant_JO » pour la version provisoire. */
function reportLabel(version: string | null): string {
  return version === 'avant_JO' ? 'compte rendu provisoire de la séance' : 'compte rendu de la séance'
}

const SPEAKER_POSITION_COLOR: Record<VoteDto['position'], string> = {
  for: 'text-yes',
  against: 'text-no',
  abstention: 'text-abstain',
  not_voting: 'text-ink-soft',
}

/**
 * Une explication de vote sous la ligne de son groupe, reprise telle quelle du
 * compte rendu (RM-03) : orateur et vote réel (RM-07), aperçu dépliable sur le
 * texte entier, lien vers le compte rendu officiel (RM-12). Ouverte d'emblée
 * quand l'adresse la vise (lien depuis la fiche dossier).
 */
function ExplanationBlock({ explanation }: { explanation: Explanation }) {
  const anchor = `explication-${explanation.rank}`
  const targeted = useRef(window.location.hash === `#${anchor}`)
  const [open, setOpen] = useState(targeted.current)
  const blockRef = useRef<HTMLDivElement>(null)
  // La fiche arrive après le chargement : le navigateur n'a pas pu viser l'ancre.
  // On vise la ligne du groupe, pour que son nom reste à l'écran.
  useEffect(() => {
    if (!targeted.current) return
    const block = blockRef.current
    const target = block?.closest<HTMLElement>('li') ?? block
    if (target) target.style.scrollMarginTop = '5rem'
    target?.scrollIntoView({ block: 'start' })
  }, [])
  const collapsible = explanation.preview_truncated && explanation.preview != null
  const folded = collapsible && !open
  const position = explanation.speaker_position

  return (
    <div id={anchor} ref={blockRef} className="scroll-mt-20 pb-3.5 sm:px-2">
      <p className="text-xs text-ink-faint">
        {explanation.speaker_name ?? explanation.speaker_uid}
        {explanation.group_uid == null && ' (groupe non établi)'}
        {', '}
        {position ? (
          <span className={`font-medium ${SPEAKER_POSITION_COLOR[position]}`}>
            a voté {positionLabels[position].toLowerCase()}
          </span>
        ) : (
          'position non publiée'
        )}
      </p>
      <p
        className={`mt-1 max-w-3xl text-sm leading-relaxed whitespace-pre-line text-ink-soft ${folded ? 'line-clamp-3' : ''}`}
      >
        {/* L'aperçu porte déjà ses points de suspension. */}« {folded ? explanation.preview : `${explanation.text} »`}
      </p>
      <p className="mt-1.5 flex flex-wrap gap-x-4 gap-y-1 text-xs">
        {collapsible && (
          <button
            type="button"
            onClick={() => setOpen(!open)}
            aria-expanded={open}
            className={`font-medium link ${FOCUS_RING}`}
          >
            {open ? 'Replier' : 'Lire l’explication en entier'}
          </button>
        )}
        {!folded && (
          <a
            href={explanation.official_url}
            target="_blank"
            rel="noreferrer"
            className="font-medium link"
          >
            Compte rendu officiel →
          </a>
        )}
      </p>
    </div>
  )
}

/** Source des explications, dite une fois sous la liste des groupes. */
function ExplanationsSource({ explanations }: { explanations: Explanations }) {
  return (
    <p className="mt-4 border-t border-line pt-3 text-xs text-ink-faint">
      Explications de vote reprises telles quelles du {reportLabel(explanations.report_version)}.
      {explanations.source_url && (
        <>
          {' '}
          <a
            href={explanations.source_url}
            target="_blank"
            rel="noreferrer"
            className="font-medium text-ink link"
          >
            Compte rendu officiel →
          </a>
        </>
      )}
    </p>
  )
}

/**
 * Vote sur l'ensemble sans explication à montrer : un état fermé, jamais une
 * section muette (RM-09).
 */
function ExplanationsAbsent({ explanations }: { explanations: Explanations }) {
  if (explanations.state === 'explained') return null
  return (
    <Card className="mt-4 px-4 py-4 sm:px-6 sm:py-5">
      <h3 className="mb-3 text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
        Explications de vote
      </h3>
      <p className="text-sm text-ink-soft">
        {explanationsStateLabels[explanations.state]}
        {explanations.source_url && (
          <>
            {' '}
            <a
              href={explanations.source_url}
              target="_blank"
              rel="noreferrer"
              className="font-medium link"
            >
              Compte rendu officiel →
            </a>
          </>
        )}
      </p>
    </Card>
  )
}

/** « 15 contre · 16 abst. · 1 non-votant » : les voix hors position de tête, vide sans elles. */
function otherVoices(tally: TallyDto, lead: LeadingPosition | null): string {
  const parts = [
    lead !== 'for' && tally.votes_for > 0 && `${tally.votes_for} pour`,
    lead !== 'against' && tally.votes_against > 0 && `${tally.votes_against} contre`,
    lead !== 'abstention' && tally.abstentions > 0 && `${tally.abstentions} abst.`,
    tally.not_voting > 0 && `${tally.not_voting} non-votant${tally.not_voting > 1 ? 's' : ''}`,
  ].filter(Boolean)
  return parts.join(' · ')
}

const LEAD_FIGURE: Record<LeadingPosition, { word: string; color: string }> = {
  for: { word: 'pour', color: 'text-yes' },
  against: { word: 'contre', color: 'text-no' },
  abstention: { word: 'abst.', color: 'text-abstain' },
}

/**
 * Un groupe sur une ligne : son nom et ses autres voix à gauche, le chiffre de
 * sa position de tête à droite. La ligne s'ouvre sur la liste nominative.
 * Sous elle, l'explication de vote de son orateur, ou la mention qu'il n'y en
 * a pas (`silent`).
 */
function GroupRow({
  group,
  explanations = [],
  silent = false,
}: {
  group: GroupBreakdownDto
  explanations?: Explanation[]
  silent?: boolean
}) {
  const [open, setOpen] = useState(false)
  const named = group.label ?? group.abbrev ?? group.group_uid ?? 'Positions sans groupe publié'
  const lead = leadingPosition(group.tally)
  const others = otherVoices(group.tally, lead)
  const voters = group.tally.votes_for + group.tally.votes_against + group.tally.abstentions
  const published =
    group.majority_position && group.majority_position in positionLabels
      ? positionLabels[group.majority_position as keyof typeof positionLabels].toLowerCase()
      : null
  const figure = lead
    ? {
        text: `${lead === 'for' ? group.tally.votes_for : lead === 'against' ? group.tally.votes_against : group.tally.abstentions} ${LEAD_FIGURE[lead].word}`,
        color: LEAD_FIGURE[lead].color,
      }
    : { text: voters === 0 ? 'aucun votant' : 'égalité', color: 'text-ink-soft' }

  return (
    <li className="border-t border-line first:border-t-0">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        className={`flex w-full items-center gap-3 rounded py-2.5 text-left transition-colors hover:bg-surface-soft sm:px-2 ${FOCUS_RING}`}
      >
        <PoliticalLogo
          logo={group.abbrev ? GROUP_LOGOS[group.abbrev] : undefined}
          fallback={group.abbrev ?? '—'}
          color={group.color}
          className="h-8 w-12 shrink-0"
        />
        <span className="min-w-0 flex-1">
          {/* Libellé officiel du groupe, jamais traduit en parti. */}
          <span className="block text-[14px] leading-snug font-semibold text-ink">{named}</span>
          {others && <span className="block text-xs text-ink-faint">{others}</span>}
          {published && group.majority_position !== lead && (
            <span className="block text-xs text-ink-faint">
              Position publiée par l’Assemblée : {published}
            </span>
          )}
          {group.origin === 'reconstructed' && (
            <span className="mt-1 block">
              <Pill tone="info">répartition reconstituée</Pill>
            </span>
          )}
        </span>
        <span className={`shrink-0 text-right text-[15px] font-semibold tabular-nums ${figure.color}`}>
          {figure.text}
        </span>
        <svg
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth={2}
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
          className={`size-4 shrink-0 text-ink-faint transition-transform ${open ? 'rotate-180' : ''}`}
        >
          <path d="M6 9l6 6 6-6" />
        </svg>
      </button>

      {open && (
        <div className="mb-2 divide-y divide-line rounded border border-line bg-surface-soft/40">
          {group.group_uid === null && (
            <p className="px-3 py-2 text-xs text-ink-faint">
              La source ne rattache ces positions à aucun groupe. Elles sont
              affichées à part plutôt que réparties d'office.
            </p>
          )}
          {group.votes.length === 0 ? (
            <p className="px-3 py-2 text-xs text-ink-faint">
              Aucune position nominale publiée pour cette ligne.
            </p>
          ) : (
            group.votes.map((v) => (
              <div key={v.actor_uid} className="flex items-center justify-between gap-3 px-3 py-1.5">
                <span className="truncate text-sm">
                  {v.official_url ? (
                    <a
                      href={v.official_url}
                      target="_blank"
                      rel="noreferrer"
                      className="hover:link"
                    >
                      {v.full_name}
                    </a>
                  ) : (
                    /* Acteur absent du référentiel : identifiant brut, aucun nom deviné. */
                    v.full_name ?? (
                      <span className="font-mono text-xs text-ink-faint">{v.actor_uid}</span>
                    )
                  )}
                  {v.by_delegation && (
                    <span className="ml-2 text-xs text-ink-faint">par délégation</span>
                  )}
                  {v.cause_code && (
                    <span className="ml-2 text-xs text-ink-faint">cause {v.cause_code}</span>
                  )}
                </span>
                <span
                  className={`shrink-0 rounded px-1.5 py-0.5 text-[11px] font-medium ring-1 ring-inset ${positionClasses[v.position]}`}
                >
                  {positionLabels[v.position]}
                </span>
              </div>
            ))
          )}
        </div>
      )}

      {explanations.map((explanation) => (
        <ExplanationBlock key={explanation.rank} explanation={explanation} />
      ))}
      {silent && (
        <p className="pb-3 text-xs text-ink-faint sm:px-2">Pas d’explication de vote.</p>
      )}
    </li>
  )
}

/*
 * Les groupes sont rangés selon leurs voix : le titre de la rangée dit ce que
 * la plupart de leurs députés ont voté, la ligne de chaque groupe donne ses
 * autres voix. Dans une rangée, l'ordre est celui de la source : aucun
 * classement. La position majoritaire publiée par la source n'est jamais
 * recalculée : quand elle diffère, la ligne l'affiche à part (RM-02).
 */
const SECTIONS = [
  { key: 'for', title: 'Majoritairement pour', dot: 'bg-yes-bar' },
  { key: 'against', title: 'Majoritairement contre', dot: 'bg-no-bar' },
  { key: 'abstention', title: 'Majoritairement abstention', dot: 'bg-abstain-bar' },
  { key: null, title: 'Partagés à égalité, ou sans votant', dot: 'bg-line-strong' },
] as const

/**
 * Les explications de vote sont rangées sous la ligne de leur groupe. Un
 * orateur qu'aucune ligne ne porte (groupe non établi, ou absent du scrutin)
 * reste affiché à part : aucune explication n'est omise.
 */
function GroupRows({
  groups,
  explanations,
}: {
  groups: GroupBreakdownDto[]
  explanations: Explanations | null
}) {
  const explained = explanations?.state === 'explained' ? explanations : null
  const rowGroups = new Set(groups.map((g) => g.group_uid))
  const byGroup = new Map<string, Explanation[]>()
  const unplaced: Explanation[] = []
  for (const item of explained?.items ?? []) {
    if (item.group_uid != null && rowGroups.has(item.group_uid)) {
      byGroup.set(item.group_uid, [...(byGroup.get(item.group_uid) ?? []), item])
    } else {
      unplaced.push(item)
    }
  }
  const silent = new Set(explained?.groups_without_speaker.map((g) => g.group_uid))

  return (
    <div className="space-y-4">
      {SECTIONS.map((section) => {
        const members = groups.filter((g) => leadingPosition(g.tally) === section.key)
        if (members.length === 0) return null
        return (
          <div key={section.title}>
            <h4 className="mb-1 flex items-center gap-2 text-[13px] font-semibold">
              <span aria-hidden className={`size-2.5 rounded-[3px] ${section.dot}`} />
              {section.title}
              <span className="font-normal text-ink-faint">
                · {members.length} groupe{members.length > 1 ? 's' : ''}
              </span>
            </h4>
            <ul>
              {members.map((g, i) => (
                <GroupRow
                  key={g.group_uid ?? `orphans-${i}`}
                  group={g}
                  explanations={g.group_uid ? byGroup.get(g.group_uid) : undefined}
                  silent={g.group_uid != null && silent.has(g.group_uid)}
                />
              ))}
            </ul>
          </div>
        )
      })}
      {unplaced.length > 0 && (
        <div>
          <h4 className="mb-2 text-[13px] font-semibold">Autres explications de vote</h4>
          {unplaced.map((explanation) => (
            <ExplanationBlock key={explanation.rank} explanation={explanation} />
          ))}
        </div>
      )}
    </div>
  )
}

export default function ScrutinDetailPage() {
  const { uid } = useParams<{ uid: string }>()
  const [partage, setPartage] = useState(false)

  const { data, isLoading, isError, error } = useQuery<ScrutinDetailDto>({
    queryKey: ['scrutin', uid],
    queryFn: () => fetchJson(`/api/scrutins/${uid}`),
    enabled: !!uid,
  })

  // L'auteur et le sort du texte viennent du dossier. Leur absence ne bloque
  // pas la page : le scrutin se lit sans eux.
  const dossierUid = data?.dossier_uid ?? null
  const { data: dossier } = useQuery<DossierContextDto>({
    queryKey: ['dossier', dossierUid],
    queryFn: () => fetchJson(`/api/dossiers/${dossierUid}`),
    enabled: !!dossierUid,
  })

  if (isLoading) return <Loading>Chargement du vote…</Loading>

  if (isError) {
    return (
      <div>
        <BackLink to="/scrutins" className="mb-4">Retour aux votes</BackLink>
        <ErrorPanel error={error} />
      </div>
    )
  }

  if (!data) return null

  const s = data.synthesis
  const adopted = data.outcome_code === 'adopté'
  const methodNote = data.groups.find((g) => g.method_note)?.method_note
  const votesNominaux = siegesDesGroupes(data.groups)
  const unknownActorsNote =
    data.unknown_actors > 0 &&
    `${data.unknown_actors} votant${data.unknown_actors > 1 ? 's' : ''} ne figure${data.unknown_actors > 1 ? 'nt' : ''} pas au référentiel des acteurs : leur identifiant est affiché tel quel, aucun nom n'est deviné.`
  const hasCited = data.cited.length > 0 || data.unnumbered_identical_amendments
  const hasDossierContext = Boolean(
    dossier && (dossier.initiators.length > 0 || dossier.outcome.kind !== 'no_recorded_conclusion'),
  )

  return (
    <div>
      <BackLink to="/scrutins" className="mb-4">Retour aux votes</BackLink>

      {/*
        Ordre de lecture : d'abord ce qui a été mis aux voix, puis le résultat,
        puis les groupes. Le résultat en premier a été écarté : un décompte se
        lit mal avant de savoir sur quoi il porte.
      */}
      <header className="space-y-2.5">
        <VoteCardHeader
          adopted={adopted}
          outcome={adopted ? 'Adopté' : 'Rejeté'}
          outcomeTitle={capitalized(data.outcome_label)}
          reading={data.reading}
          date={data.date}
        />
        <h2 className="heading-2">
          {data.dossier_uid ? data.dossier_label || data.dossier_uid : capitalized(data.subject)}
        </h2>
        {data.families.length > 0 && (
          <div className="flex flex-wrap gap-2">
            {data.families.map((family) => (
              <Link
                key={family.code}
                to={themeLink(family.code)}
                className={`inline-flex items-center gap-2 rounded-full border border-line bg-surface py-0.5 pr-3 pl-0.5 text-[13px] font-semibold text-ink transition-colors hover:border-line-strong ${FOCUS_RING}`}
              >
                <FamilyIllustration
                  code={family.code}
                  className="size-6 shrink-0 rounded-full"
                  glyphClassName="size-3.5"
                />
                {typographic(family.label)}
              </Link>
            ))}
          </div>
        )}
      </header>

      {/*
        Une seule carte pour ce qui a été mis aux voix : l'amendement, son
        auteur et son exposé tels que publiés, l'intitulé officiel, puis le
        texte — où il en est, qui l'a déposé.
      */}
      {(hasCited || data.dossier_uid || hasDossierContext) && (
        <Card className="mt-4 px-4 py-4 sm:px-6 sm:py-5">
          <h3 className="mb-3 text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
            Ce qui a été mis aux voix
          </h3>
          {hasCited && (
            <div className="divide-y divide-line">
              {data.cited.map((c) => (
                <div key={c.rank} className="py-4 first:pt-0 last:pb-0">
                  {c.amendment ? (
                    <CitedAmendment amendment={c.amendment} methodLabels={c.method_labels} />
                  ) : (
                    <p className="text-sm text-ink-faint">
                      {c.cited_number ? `Amendement n° ${c.cited_number} : ` : ''}
                      {c.cause_label}
                    </p>
                  )}
                </div>
              ))}
              {data.unnumbered_identical_amendments && (
                <p className="py-3 text-xs text-ink-faint">
                  L'objet annonce des amendements identiques sans en donner le numéro.
                </p>
              )}
            </div>
          )}
          {data.dossier_uid &&
            (hasCited ? (
              <p className="mt-4 border-t border-line pt-3 text-[13px] leading-relaxed text-ink-soft">
                <span className="text-ink-faint">Intitulé officiel : </span>
                {capitalized(data.subject)}
              </p>
            ) : (
              <p className="text-[15px] leading-relaxed text-ink">{capitalized(data.subject)}</p>
            ))}
          {hasDossierContext && dossier && (
            <div className="mt-4 border-t border-line pt-3">
              <p className="mb-2 text-xs font-semibold text-ink-faint">Le texte</p>
              <div className="grid gap-2.5 sm:grid-cols-2">
                {data.dossier_uid && (
                  <OutcomeBox outcome={dossier.outcome} dossierUid={data.dossier_uid} />
                )}
                {dossier.initiators.length === 1 ? (
                  <InitiatorBox initiator={dossier.initiators[0]} kind={textKind(dossier.procedure)} />
                ) : dossier.initiators.length > 1 ? (
                  <InitiatorsBox initiators={dossier.initiators} kind={textKind(dossier.procedure)} />
                ) : null}
              </div>
            </div>
          )}
        </Card>
      )}

      {/*
        Ce qu'a décidé l'Assemblée : les chiffres, puis l'hémicycle. Les deux
        ensemble montrent ce qu'aucun total ne montre : les fractures à
        l'intérieur des groupes, un non-votant isolé, une abstention unique.
      */}
      <Card className="mt-4 px-4 py-4 sm:px-6 sm:py-5">
        <h3
          id="hemicycle-scrutin"
          className="mb-3 text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase"
        >
          Résultat
        </h3>
        <div className="grid items-center gap-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,3fr)]">
          <div>
            <div className="flex flex-wrap gap-x-7 gap-y-3">
              {(
                [
                  ['pour', s.tally.votes_for, 'text-yes'],
                  ['contre', s.tally.votes_against, 'text-no'],
                  ['abstentions', s.tally.abstentions, 'text-abstain'],
                ] as const
              ).map(([label, value, color]) => (
                <div key={label}>
                  <p className={`figure text-[48px] ${color}`}>
                    <CountUp value={value} />
                  </p>
                  <p className="mt-1 text-xs text-ink-faint">{label}</p>
                </div>
              ))}
            </div>
            <div className="mt-4">
              <VoteBar tally={s.tally} />
            </div>
            <p className="mt-3 text-xs leading-relaxed text-ink-faint">
              {s.required} voix requises sur {s.expressed} suffrages exprimés
              {data.majority_label && ` (${data.majority_label.toLowerCase()})`}
              {s.tally.not_voting > 0 &&
                ` · ${s.tally.not_voting} non-votant${s.tally.not_voting > 1 ? 's' : ''}`}
              .{' '}
              <span className="hidden md:inline">
                <ExpressedVotesHelp />
              </span>
              <InfoTip label="Comment lire ce résultat" className="md:hidden">
                <p>
                  <ExpressedVotesHelp />
                </p>
                {votesNominaux.length > 0 && (
                  <p>
                    <HemicycleHelp />
                  </p>
                )}
              </InfoTip>
            </p>
          </div>

          {votesNominaux.length > 0 && (
            <div>
              <Hemicycle votes={votesNominaux} labelledBy="hemicycle-scrutin" />
              <p className="mt-1 hidden text-center text-xs text-ink-faint md:block">
                <HemicycleHelp />
              </p>
            </div>
          )}
        </div>
      </Card>

      <Card className="mt-4 px-4 py-4 sm:px-6 sm:py-5">
        <h3 className="mb-1 text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
          Qui a voté quoi
          <InfoTip label="Comment lire ces votes" className="md:hidden">
            <p>
              Groupe du député à la date du vote. Un groupe n'est pas un parti.{' '}
              <GuideLink ancre="q-groupe">Pourquoi cette distinction</GuideLink>
            </p>
            {methodNote && <p>{methodNote}</p>}
            {unknownActorsNote && <p>{unknownActorsNote}</p>}
            <p>{data.coverage_note}</p>
          </InfoTip>
        </h3>
        <p className="mb-3 text-xs text-ink-faint">
          Touchez un groupe pour le vote de chaque député.
          <span className="hidden md:inline">
            {' '}
            Groupe du député à la date du vote. Un groupe n'est pas un parti.{' '}
            <GuideLink ancre="q-groupe">Pourquoi cette distinction</GuideLink>
          </span>
        </p>
        {/* Mention de méthode portée une fois pour toute la section : les
            chiffres marqués « reconstituée » sont produits par le site, pas
            publiés par l'Assemblée. */}
        {methodNote && (
          <p className="mb-4 hidden rounded bg-info-soft px-3 py-2 text-xs leading-relaxed text-info ring-1 ring-info/15 ring-inset md:block">
            {methodNote}
          </p>
        )}
        <GroupRows groups={data.groups} explanations={data.explanations} />
        {data.explanations?.state === 'explained' && (
          <ExplanationsSource explanations={data.explanations} />
        )}
      </Card>

      {data.explanations && <ExplanationsAbsent explanations={data.explanations} />}

      {data.corrections.length > 0 && (
        <section className="mt-4">
          <SectionTitle info="Déclarations postérieures au vote. Elles ne modifient ni le résultat ni la répartition ci-dessus.">
            Mises au point · {data.corrections.length}
          </SectionTitle>
          {/* Déclarations postérieures des députés. Elles ne modifient aucun
              décompte publié par l'Assemblée. */}
          <p className="mb-3 hidden text-xs text-ink-faint md:block">
            Déclarations postérieures au vote. Elles ne modifient ni le
            résultat ni la répartition ci-dessus.
          </p>
          <div className="divide-y divide-line overflow-hidden rounded border border-line bg-surface shadow-card">
            {data.corrections.map((c) => (
              <div key={c.actor_uid} className="flex items-center justify-between gap-3 px-4 py-1.5">
                <span className="truncate text-sm">
                  {c.full_name ?? (
                    <span className="font-mono text-xs text-ink-faint">{c.actor_uid}</span>
                  )}
                  {c.malfunction && (
                    <span className="ml-2 text-xs text-ink-faint">dysfonctionnement signalé</span>
                  )}
                </span>
                <span className="shrink-0 text-xs text-ink-soft">
                  souhaitait voter{' '}
                  <span
                    className={`rounded px-1.5 py-0.5 text-[11px] font-medium ring-1 ring-inset ${positionClasses[c.claimed_position]}`}
                  >
                    {positionLabels[c.claimed_position]}
                  </span>
                </span>
              </div>
            ))}
          </div>
        </section>
      )}

      {/* Les liens et la référence officielle, en bas : on y va après avoir lu. */}
      <nav
        aria-label="Pour aller plus loin"
        className="mt-6 divide-y divide-line overflow-hidden rounded border border-line bg-surface shadow-card"
      >
        {data.dossier_uid && (
          <Link
            to={`/dossiers/${data.dossier_uid}`}
            className={`flex min-h-12 items-center justify-between gap-3 px-4 text-sm font-medium link hover:bg-surface-soft ${FOCUS_RING}`}
          >
            Le texte et tout son parcours
            <span aria-hidden>→</span>
          </Link>
        )}
        <a
          href={data.official_url}
          target="_blank"
          rel="noreferrer"
          className={`flex min-h-12 items-center justify-between gap-3 px-4 text-sm font-medium link hover:bg-surface-soft ${FOCUS_RING}`}
        >
          Page officielle du scrutin
          <span aria-hidden>↗</span>
        </a>
        <button
          type="button"
          onClick={() => setPartage(!partage)}
          aria-expanded={partage}
          className={`flex min-h-12 w-full items-center justify-between gap-3 px-4 text-left text-sm font-medium link hover:bg-surface-soft ${FOCUS_RING}`}
        >
          Partager en image
          <span aria-hidden>{partage ? '−' : '+'}</span>
        </button>
      </nav>

      {partage && <VoteImageExport scrutin={data} onClose={() => setPartage(false)} />}

      {/* Ce que le vote recouvre exactement, et ce qu'il ne recouvre pas. */}
      <p className="mt-3 text-xs leading-relaxed text-ink-faint">
        Scrutin n° {data.number} · {data.ballot_type_label}
        {data.requester && `, demandé par : ${data.requester}`}.{' '}
        <span className="hidden md:inline">
          <ScrutinScopeHelp dossierUid={data.dossier_uid} />
        </span>
        <InfoTip label="Ce que ce vote recouvre" className="md:hidden">
          <ScrutinScopeHelp dossierUid={data.dossier_uid} />
        </InfoTip>
      </p>

      <div className="mt-4 space-y-2">
        {unknownActorsNote && <Note desktopOnly>{unknownActorsNote}</Note>}
        <CoverageNote note={data.coverage_note} />
      </div>
    </div>
  )
}
