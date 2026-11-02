import { useState, type ReactNode } from 'react'
import { Link, useParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { FamilyIllustration } from '../components/FamilyIllustration'
import GuideLink from '../components/GuideLink'
import Hemicycle from '../components/Hemicycle'
import { PoliticalLogo } from '../components/PoliticalLogo'
import VoteImageExport from '../components/VoteImageExport'
import PositionGlyph from '../components/entry/PositionGlyph'
import { FOCUS_RING } from '../components/entry/styles'
import { siegesDesGroupes } from '../components/sieges'
import { CoverageNote } from '../components/ScrutinList'
import {
  Card,
  ErrorPanel,
  Loading,
  Note,
  Pill,
  SectionTitle,
  TallyLine,
  VoteBar,
} from '../components/ui'
import { GROUP_LOGOS } from '../content/logos'
import { fetchJson } from '../lib/fetchJson'
import { capitalized, typographic } from '../lib/entryFlow'
import { amendmentCardPath } from '../types/amendments'
import type { InitiatorDto, OutcomeDto } from '../types/dossiers'
import type { GroupBreakdownDto, ScrutinDetailDto } from '../types/scrutins'
import {
  formatDate,
  positionClasses,
  positionLabels,
  tallySize,
} from '../types/scrutins'

/** Ce que la page lit du dossier : qui l'a déposé, où il en est. */
interface DossierContextDto {
  procedure: string
  initiators: InitiatorDto[]
  outcome: OutcomeDto
}

/** « Proposition de loi » ou « Projet de loi », lu sur la procédure ; « Texte » sinon. */
function textKind(procedure: string): string {
  if (procedure.startsWith('Projet de loi')) return 'Projet de loi'
  if (procedure.startsWith('Proposition de loi')) return 'Proposition de loi'
  return 'Texte'
}

function shortDate(iso: string) {
  return new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    month: 'short',
    year: 'numeric',
  })
}

/** Vignette d'information de l'en-tête : un visuel à gauche, trois lignes à droite. */
function InfoBox({ visual, children }: { visual: ReactNode; children: ReactNode }) {
  return (
    <div className="flex items-center gap-3 rounded-xl bg-surface-soft px-3 py-2.5 text-sm leading-snug">
      {visual}
      <div className="min-w-0">{children}</div>
    </div>
  )
}

/**
 * Auteur du texte, avec le logo de son groupe. Le groupe est celui lu à la
 * date du dépôt, et cette date l'accompagne toujours : l'appartenance est
 * datée, et un groupe n'est pas un parti (README.md §3).
 */
function InitiatorBox({ initiator, kind }: { initiator: InitiatorDto; kind: string }) {
  const group = initiator.reference_date ? initiator.group : null
  return (
    <InfoBox
      visual={
        group ? (
          <PoliticalLogo
            logo={GROUP_LOGOS[group.abbrev]}
            fallback={group.abbrev}
            color={null}
            className="h-11 w-16"
          />
        ) : (
          <span className="flex h-11 w-16 shrink-0 items-center justify-center rounded-lg bg-surface text-xs font-semibold text-ink-soft ring-1 ring-inset ring-line">
            {initiator.role ?? '—'}
          </span>
        )
      }
    >
      <p className="text-xs text-ink-faint">{kind} déposé par</p>
      <p>
        {initiator.official_url ? (
          <a
            href={initiator.official_url}
            target="_blank"
            rel="noreferrer"
            className="font-semibold hover:text-accent hover:underline"
          >
            {initiator.full_name}
          </a>
        ) : (
          <span className="font-semibold">{initiator.full_name}</span>
        )}
        {group && <span className="text-ink-soft"> · {group.label}</span>}
      </p>
      {group && initiator.reference_date && (
        <p className="text-xs text-ink-faint">
          groupe au {shortDate(initiator.reference_date)}, date du dépôt
        </p>
      )}
    </InfoBox>
  )
}

/**
 * Sort du texte, distinct de l'issue du scrutin : un texte adopté ici peut
 * encore être modifié ailleurs. Rien n'est affiché quand la source ne conclut
 * rien — « en cours » serait une affirmation que personne n'a écrite.
 */
function OutcomeBox({ outcome, dossierUid }: { outcome: OutcomeDto; dossierUid: string }) {
  if (outcome.kind === 'no_recorded_conclusion') return null
  const tone =
    outcome.kind === 'promulgated'
      ? 'bg-yes-soft text-yes'
      : outcome.kind === 'rejected'
        ? 'bg-no-soft text-no'
        : 'bg-surface text-ink-soft ring-1 ring-inset ring-line'
  return (
    <InfoBox
      visual={
        <span className={`flex size-11 shrink-0 items-center justify-center rounded-lg ${tone}`}>
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
      </p>
      <p className="text-xs">
        {outcome.date && <span className="text-ink-faint">{shortDate(outcome.date)} · </span>}
        <Link to={`/dossiers/${dossierUid}`} className="font-medium text-accent hover:underline">
          Voir le dossier législatif →
        </Link>
      </p>
    </InfoBox>
  )
}

/**
 * Un groupe dans sa colonne : son logo, la barre de ses positions sur son
 * effectif — la part vide dit les membres sans position —, puis ses chiffres.
 * La tuile s'ouvre sur la liste nominative.
 */
function GroupTile({ group }: { group: GroupBreakdownDto }) {
  const [open, setOpen] = useState(false)
  const named = group.label ?? group.abbrev ?? group.group_uid ?? 'Positions sans groupe publié'
  const outOf = group.member_count ?? tallySize(group.tally)
  const published =
    group.majority_position && group.majority_position in positionLabels
      ? positionLabels[group.majority_position as keyof typeof positionLabels].toLowerCase()
      : null

  return (
    <li className="overflow-hidden rounded-xl border border-line bg-surface">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        aria-expanded={open}
        className={`flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-surface-soft ${FOCUS_RING}`}
      >
        <PoliticalLogo
          logo={group.abbrev ? GROUP_LOGOS[group.abbrev] : undefined}
          fallback={group.abbrev ?? '—'}
          color={group.color}
          className="h-9 w-14"
        />
        <span className="min-w-0 flex-1 space-y-1.5">
          {/* Libellé officiel du groupe, jamais traduit en parti. */}
          <span className="flex items-baseline justify-between gap-2">
            <span className="truncate text-[13px] font-semibold" title={named}>
              {named}
            </span>
            {group.member_count !== null && (
              <span className="shrink-0 text-[11px] text-ink-faint">
                {group.member_count} membres
              </span>
            )}
          </span>
          <VoteBar tally={group.tally} outOf={outOf} />
          <span className="flex items-center justify-between gap-2">
            <TallyLine tally={group.tally} />
            {group.origin === 'reconstructed' && <Pill tone="info">reconstituée</Pill>}
          </span>
          {published && group.majority_position !== leadingPosition(group) && (
            <span className="block text-[11px] text-ink-faint">
              Position majoritaire publiée par l'Assemblée : {published}
            </span>
          )}
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
        <div className="divide-y divide-line border-t border-line">
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
                      className="hover:text-accent hover:underline"
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
                  className={`shrink-0 rounded-md px-1.5 py-0.5 text-[11px] font-medium ring-1 ring-inset ${positionClasses[v.position]}`}
                >
                  {positionLabels[v.position]}
                </span>
              </div>
            ))
          )}
        </div>
      )}
    </li>
  )
}

type Lead = 'for' | 'against' | 'abstention'

/**
 * Position qui a réuni le plus de députés du groupe, lue sur ses comptes bruts.
 * `null` quand personne ne s'est prononcé ou que deux positions sont à égalité :
 * le groupe va alors à part, jamais dans une colonne choisie au hasard.
 */
function leadingPosition(group: GroupBreakdownDto): Lead | null {
  const { votes_for, votes_against, abstentions } = group.tally
  const counts: [Lead, number][] = [
    ['for', votes_for],
    ['against', votes_against],
    ['abstention', abstentions],
  ]
  const top = Math.max(votes_for, votes_against, abstentions)
  const leaders = counts.filter(([, n]) => n === top)
  return top > 0 && leaders.length === 1 ? leaders[0][0] : null
}

/*
 * Les groupes sont rangés selon leurs voix : la colonne dit ce que la plupart
 * de leurs députés ont voté, la barre de chaque tuile dit s'ils étaient
 * divisés. La position majoritaire publiée par la source n'est jamais
 * recalculée : quand elle diffère, la tuile l'affiche à part (RM-02).
 */
const COLUMNS = [
  { key: 'for', title: 'Majoritairement pour', dot: 'bg-yes-bar' },
  { key: 'against', title: 'Majoritairement contre', dot: 'bg-no-bar' },
  { key: 'abstention', title: 'Majoritairement abstention', dot: 'bg-abstain-bar' },
] as const

function GroupColumns({ groups }: { groups: GroupBreakdownDto[] }) {
  const others = groups.filter((g) => leadingPosition(g) === null)

  return (
    <>
      <div className="grid gap-4 md:grid-cols-3">
        {COLUMNS.map((column) => {
          const members = groups.filter((g) => leadingPosition(g) === column.key)
          return (
            <div key={column.key}>
              <h4 className="mb-2 flex items-center gap-2 text-[13px] font-semibold">
                <span className={`size-2.5 rounded-[3px] ${column.dot}`} />
                {column.title}
                <span className="font-normal text-ink-faint">{members.length}</span>
              </h4>
              {members.length === 0 ? (
                <p className="text-xs text-ink-faint">Aucun groupe</p>
              ) : (
                <ul className="space-y-2">
                  {members.map((g, i) => (
                    <GroupTile key={g.group_uid ?? `orphans-${i}`} group={g} />
                  ))}
                </ul>
              )}
            </div>
          )
        })}
      </div>

      {others.length > 0 && (
        <div className="mt-5">
          <h4 className="mb-2 text-[13px] font-semibold text-ink-soft">
            Partagés à égalité, ou sans votant
          </h4>
          <ul className="grid gap-2 md:grid-cols-3">
            {others.map((g, i) => (
              <GroupTile key={g.group_uid ?? `others-${i}`} group={g} />
            ))}
          </ul>
        </div>
      )}
    </>
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

  if (isLoading) return <Loading>Chargement du scrutin…</Loading>

  if (isError) {
    return (
      <div>
        <Link to="/scrutins" className="mb-4 inline-block text-sm text-accent">
          ← Retour aux scrutins
        </Link>
        <ErrorPanel error={error} />
      </div>
    )
  }

  if (!data) return null

  const s = data.synthesis
  const adopted = data.outcome_code === 'adopté'
  const methodNote = data.groups.find((g) => g.method_note)?.method_note
  const votesNominaux = siegesDesGroupes(data.groups)

  return (
    <div>
      <Link
        to="/scrutins"
        className="mb-4 inline-block text-sm font-medium text-accent hover:underline"
      >
        ← Retour aux scrutins
      </Link>

      {/*
        De quoi il s'agit : l'issue, la date, la lecture et les thèmes sur une
        ligne ; le dossier en titre, l'intitulé exact mis aux voix en citation ;
        puis qui a déposé le texte et où il en est.
      */}
      <Card className="mb-4 px-4 py-4 sm:px-6 sm:py-5">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-2 text-[13px] text-ink-soft">
          <Pill tone={adopted ? 'yes' : 'no'} title={data.outcome_label}>
            <PositionGlyph
              position={adopted ? 'for' : 'against'}
              className="mr-1 size-3"
              strokeWidth={3.2}
            />
            {adopted ? 'Adopté par l’Assemblée' : 'Rejeté par l’Assemblée'}
          </Pill>
          <time dateTime={data.date}>{formatDate(data.date)}</time>
          {data.reading && <Pill tone="info">{capitalized(data.reading)}</Pill>}
          {data.families.map((family) => (
            <Link
              key={family.code}
              to={`/themes/${family.code}`}
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

        {data.dossier_uid ? (
          <>
            <h2 className="mt-3 text-2xl leading-tight font-semibold tracking-tight sm:text-[28px]">
              {data.dossier_label || data.dossier_uid}
            </h2>
            <blockquote className="mt-3 max-w-4xl border-l-[3px] border-line-strong pl-3 text-[15px] leading-relaxed text-ink-soft">
              <span className="block text-xs text-ink-faint">Mis aux voix</span>
              {capitalized(data.subject)}
            </blockquote>
          </>
        ) : (
          <h2 className="mt-3 max-w-4xl text-2xl leading-tight font-semibold tracking-tight">
            {capitalized(data.subject)}
          </h2>
        )}

        {dossier && (dossier.initiators.length > 0 || dossier.outcome.kind !== 'no_recorded_conclusion') && (
          <div className="mt-4 grid gap-2.5 sm:grid-cols-2">
            {dossier.initiators.map((initiator, i) => (
              <InitiatorBox key={i} initiator={initiator} kind={textKind(dossier.procedure)} />
            ))}
            {data.dossier_uid && (
              <OutcomeBox outcome={dossier.outcome} dossierUid={data.dossier_uid} />
            )}
          </div>
        )}

        <div className="mt-4 flex flex-wrap items-center gap-x-5 gap-y-1 text-sm">
          <a
            href={data.official_url}
            target="_blank"
            rel="noreferrer"
            className="font-medium text-accent hover:underline"
          >
            Page officielle du scrutin ↗
          </a>
          {data.dossier_uid && (
            <Link
              to={`/dossiers/${data.dossier_uid}`}
              className="font-medium text-accent hover:underline"
            >
              Dossier législatif →
            </Link>
          )}
          <button
            type="button"
            onClick={() => setPartage(!partage)}
            aria-expanded={partage}
            className="font-medium text-accent hover:underline"
          >
            Partager en image
          </button>
        </div>

        {/* Ce que le vote recouvre exactement, et ce qu'il ne recouvre pas. */}
        <p className="mt-3 text-xs leading-relaxed text-ink-faint">
          Scrutin n° {data.number} · {data.ballot_type_label}
          {data.requester && ` · demandé par : ${data.requester}`}. Le vote porte
          sur cet objet, pas nécessairement sur l'ensemble du texte.{' '}
          <GuideLink ancre="q-scrutin-loi">Comment lire un scrutin</GuideLink>
          {!data.dossier_uid && (
            <>
              {' · '}La source ne rattache ce scrutin à aucun dossier
              législatif, comme la majorité des scrutins.{' '}
              <GuideLink ancre="q-sans-dossier">Pourquoi</GuideLink>
            </>
          )}
        </p>
      </Card>

      {partage && <VoteImageExport scrutin={data} onClose={() => setPartage(false)} />}

      {/*
        Ce qu'a décidé l'Assemblée : les chiffres à gauche, l'hémicycle à
        droite. Les deux ensemble montrent ce qu'aucun total ne montre : les
        fractures à l'intérieur des groupes, un non-votant isolé, une abstention
        unique.
      */}
      <Card className="mb-4 px-4 py-4 sm:px-6 sm:py-5">
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
                  ['non-votants', s.tally.not_voting, 'text-ink-soft'],
                ] as const
              ).map(([label, value, color]) => (
                <div key={label}>
                  <p className={`text-[32px] leading-none font-semibold tracking-tight ${color}`}>
                    {value}
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
              {data.majority_label && ` (${data.majority_label.toLowerCase()})`}.
              Les abstentions n'entrent pas dans les suffrages exprimés, et
              « non-votant » ne se lit pas comme une absence.{' '}
              <GuideLink ancre="q-non-votant">Ce que recouvrent les positions</GuideLink>
            </p>
          </div>

          {votesNominaux.length > 0 && (
            <div>
              <Hemicycle votes={votesNominaux} labelledBy="hemicycle-scrutin" />
              <p className="mt-1 text-center text-xs text-ink-faint">
                Chaque député à sa place réelle (
                <a
                  href="https://www.assemblee-nationale.fr/dyn/vos-deputes/hemicycle"
                  target="_blank"
                  rel="noreferrer"
                  className="text-accent hover:underline"
                >
                  plan de l'hémicycle
                </a>
                ). Survolez un siège pour lire le nom, le groupe et la position ; en
                gris, aucune position enregistrée.
              </p>
            </div>
          )}
        </div>
      </Card>

      {(data.cited.length > 0 || data.unnumbered_identical_amendments) && (
        <section className="mb-4">
          <SectionTitle>Ce qui a été mis aux voix</SectionTitle>
          <div className="space-y-2">
            {data.cited.map((c) => (
              <Card key={c.rank}>
                {c.amendment ? (
                  <div>
                    <p className="text-sm font-medium">
                      Amendement n° {c.amendment.number} — {c.amendment.target_title}
                    </p>
                    {c.amendment.author_name && (
                      <p className="text-xs text-ink-faint">
                        {c.amendment.author_name}
                        {c.amendment.author_group_abbrev && ` (${c.amendment.author_group_abbrev})`}
                      </p>
                    )}
                    {c.amendment.summary && (
                      <p className="mt-2 text-sm leading-relaxed whitespace-pre-line">
                        {c.amendment.summary}
                      </p>
                    )}
                    <p className="mt-2 text-xs text-ink-faint">
                      Lien établi par : {c.method_labels.join(' + ')}
                    </p>
                    {/* Pas de page officielle de l'amendement (AMENDEMENTS H8) :
                        le renvoi mène à sa carte, dans son dossier. */}
                    <Link
                      to={amendmentCardPath(c.amendment.dossier_uid, c.amendment.number)}
                      className="mt-1 inline-block text-xs font-medium text-accent hover:underline"
                    >
                      Voir la carte de l'amendement dans le dossier
                    </Link>
                  </div>
                ) : (
                  <p className="text-sm text-ink-faint">
                    {c.cited_number ? `Amendement n° ${c.cited_number} : ` : ''}
                    {c.cause_label}
                  </p>
                )}
              </Card>
            ))}
            {data.unnumbered_identical_amendments && (
              <p className="text-xs text-ink-faint">
                L'objet annonce des amendements identiques sans en donner le numéro.
              </p>
            )}
          </div>
        </section>
      )}

      <Card className="mb-4 px-4 py-4 sm:px-6 sm:py-5">
        <h3 className="mb-1 text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">
          Qui a voté quoi
        </h3>
        <p className="mb-4 text-xs text-ink-faint">
          Groupe du député à la date du scrutin. Un groupe n'est pas un parti.{' '}
          <GuideLink ancre="q-groupe">Pourquoi cette distinction</GuideLink>
        </p>
        {/* Mention de méthode portée une fois pour toute la section : les
            chiffres marqués « reconstituée » sont produits par le site, pas
            publiés par l'Assemblée. */}
        {methodNote && (
          <p className="mb-4 rounded-lg bg-info-soft px-3 py-2 text-xs leading-relaxed text-info ring-1 ring-info/15 ring-inset">
            {methodNote}
          </p>
        )}
        <GroupColumns groups={data.groups} />
      </Card>

      {data.corrections.length > 0 && (
        <section className="mb-4">
          <SectionTitle>Mises au point</SectionTitle>
          {/* Déclarations postérieures des députés. Elles ne modifient aucun
              décompte publié par l'Assemblée. */}
          <p className="mb-3 text-xs text-ink-faint">
            Déclarations postérieures au scrutin. Elles ne modifient ni le
            résultat ni la répartition ci-dessus.
          </p>
          <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card">
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
                    className={`rounded-md px-1.5 py-0.5 text-[11px] font-medium ring-1 ring-inset ${positionClasses[c.claimed_position]}`}
                  >
                    {positionLabels[c.claimed_position]}
                  </span>
                </span>
              </div>
            ))}
          </div>
        </section>
      )}

      <div className="space-y-2">
        {data.unknown_actors > 0 && (
          <Note>
            {data.unknown_actors} votant{data.unknown_actors > 1 ? 's' : ''} ne
            figure{data.unknown_actors > 1 ? 'nt' : ''} pas au référentiel des
            acteurs : leur identifiant est affiché tel quel, aucun nom n'est
            deviné.
          </Note>
        )}
        <CoverageNote note={data.coverage_note} />
      </div>
    </div>
  )
}
