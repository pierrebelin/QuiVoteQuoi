import { useState } from 'react'
import { Link, useParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import GuideLink from '../components/GuideLink'
import Hemicycle from '../components/Hemicycle'
import VoteImageExport from '../components/VoteImageExport'
import { siegesDesGroupes, tallyDesVotes } from '../components/sieges'
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
import { GroupDot } from './GroupListPage'
import { amendmentCardPath } from '../types/amendments'
import type { GroupBreakdownDto, ScrutinDetailDto } from '../types/scrutins'
import {
  formatDate,
  positionClasses,
  positionLabels,
  tallySize,
} from '../types/scrutins'

function GroupBlock({
  group,
  largestGroup,
}: {
  group: GroupBreakdownDto
  largestGroup: number
}) {
  const [open, setOpen] = useState(false)
  const named = group.label ?? group.abbrev

  return (
    <Card className="overflow-hidden">
      <button
        onClick={() => setOpen(!open)}
        className="w-full px-4 py-3 text-left transition-colors hover:bg-surface-soft"
      >
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:gap-6">
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              {group.color && <GroupDot color={group.color} />}
              {/* Libellé officiel du groupe, jamais traduit en parti. */}
              <span className="text-sm font-semibold">
                {named ??
                  (group.group_uid
                    ? group.group_uid
                    : 'Positions sans groupe publié')}
              </span>
              {group.abbrev && group.label && (
                <span className="text-xs text-ink-faint">{group.abbrev}</span>
              )}
              {group.member_count !== null && (
                <span className="text-xs text-ink-faint">
                  {group.member_count} membres
                </span>
              )}
              {group.origin === 'reconstructed' && (
                <Pill tone="info">reconstituée</Pill>
              )}
            </div>
            {group.group_uid === null && (
              <p className="mt-1 text-xs text-ink-faint">
                La source ne rattache ces positions à aucun groupe. Elles sont
                affichées à part plutôt que réparties d'office.
              </p>
            )}
          </div>

          {/* Toutes les barres de la page partagent l'échelle du groupe le
              plus nombreux : la longueur dit le poids, les segments disent la
              répartition interne. */}
          <div className="shrink-0 space-y-1.5 sm:w-64">
            <VoteBar tally={group.tally} outOf={largestGroup} />
            <TallyLine tally={group.tally} />
          </div>

          <span className="shrink-0 text-xs font-medium text-accent sm:w-28 sm:text-right">
            {open
              ? 'Replier'
              : group.votes.length > 1
                ? `Voir les ${group.votes.length} votants`
                : `Voir le votant`}
          </span>
        </div>
      </button>

      {open && (
        <div className="divide-y divide-line border-t border-line">
          {group.votes.length === 0 ? (
            <p className="px-4 py-2.5 text-xs text-ink-faint">
              Aucune position nominale publiée pour cette ligne.
            </p>
          ) : (
            group.votes.map((v) => (
              <div
                key={v.actor_uid}
                className="flex items-center justify-between gap-3 px-4 py-1.5"
              >
                <span className="text-sm truncate">
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
                      <span className="text-ink-faint font-mono text-xs">
                        {v.actor_uid}
                      </span>
                    )
                  )}
                  {v.by_delegation && (
                    <span className="text-xs text-ink-faint ml-2">
                      par délégation
                    </span>
                  )}
                  {v.cause_code && (
                    <span className="text-xs text-ink-faint ml-2">
                      cause {v.cause_code}
                    </span>
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
    </Card>
  )
}

export default function ScrutinDetailPage() {
  const { uid } = useParams<{ uid: string }>()
  const [partage, setPartage] = useState(false)

  const { data, isLoading, isError, error } = useQuery<ScrutinDetailDto>({
    queryKey: ['scrutin', uid],
    queryFn: () =>
      fetch(`/api/scrutins/${uid}`).then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`)
        return res.json()
      }),
    enabled: !!uid,
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
  const methodNote = data.groups.find((g) => g.method_note)?.method_note
  const largestGroup = Math.max(...data.groups.map((g) => tallySize(g.tally)), 1)
  const votesNominaux = siegesDesGroupes(data.groups)

  return (
    <div>
      <Link
        to="/scrutins"
        className="mb-4 inline-block text-sm font-medium text-accent hover:underline"
      >
        ← Retour aux scrutins
      </Link>

      <div className="mb-5">
        <div className="mb-2 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-ink-faint">
          <Pill
            tone={data.outcome_code === 'adopté' ? 'yes' : 'no'}
            title={data.outcome_label}
          >
            {data.outcome_code}
          </Pill>
          <span>
            Scrutin n° {data.number} · {data.ballot_type_label}
          </span>
          <span>· {formatDate(data.date)}</span>
          {data.place && <span>· {data.place}</span>}
          {data.requester && <span>· demandé par : {data.requester}</span>}
        </div>

        {/*
          Le dossier en titre, l'intitulé mis aux voix juste dessous : la même
          hiérarchie que sur la liste, pour que la page ne surprenne pas celui
          qui vient d'y cliquer. L'intitulé reste entier — c'est lui qui dit
          sur quoi on a voté exactement.
        */}
        {data.dossier_uid ? (
          <>
            <h2 className="text-2xl font-semibold leading-tight tracking-tight">
              {data.dossier_label || data.dossier_uid}
            </h2>
            <p className="mt-1.5 max-w-4xl text-sm leading-relaxed text-ink-soft">
              {data.subject}
            </p>
          </>
        ) : (
          <h2 className="max-w-4xl text-2xl font-semibold leading-tight tracking-tight">
            {data.subject}
          </h2>
        )}

        <div className="mt-3 flex flex-wrap items-center gap-x-4 gap-y-1 text-sm">
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

        {/* Ce que le vote recouvre exactement, et ce qu'il ne recouvre pas :
            les deux mises en garde tiennent en une note, la prose complète
            reste sur « Comprendre ». */}
        <p className="mt-3 text-xs leading-relaxed text-ink-faint">
          Le vote porte sur cet objet, pas nécessairement sur l'ensemble du
          texte. <GuideLink ancre="q-scrutin-loi">Comment lire un scrutin</GuideLink>
          {data.majority_label && ` · ${data.majority_label}`}
          {!data.dossier_uid && (
            <>
              {' · '}La source ne rattache ce scrutin à aucun dossier
              législatif, comme la majorité des scrutins.{' '}
              <GuideLink ancre="q-sans-dossier">Pourquoi</GuideLink>
            </>
          )}
        </p>
      </div>

      {partage && (
        <VoteImageExport scrutin={data} onClose={() => setPartage(false)} />
      )}

      {/*
        La synthèse est ce que le lecteur vient chercher : elle occupe le haut
        de page, en chiffres lisibles de loin, avec la barre qui en donne la
        proportion d'un coup d'œil.
      */}
      <Card className="mb-5 px-5 py-4">
        <div className="flex flex-wrap items-end justify-between gap-x-8 gap-y-4">
          <div className="flex flex-wrap gap-x-8 gap-y-3">
            {[
              ['Pour', s.tally.votes_for, 'text-yes'],
              ['Contre', s.tally.votes_against, 'text-no'],
              ['Abstentions', s.tally.abstentions, 'text-abstain'],
              ['Non-votants', s.tally.not_voting, 'text-ink-soft'],
            ].map(([label, value, color]) => (
              <div key={label as string}>
                <p
                  className={`text-3xl font-semibold tracking-tight ${color as string}`}
                >
                  {value as number}
                </p>
                <p className="text-xs text-ink-faint">{label as string}</p>
              </div>
            ))}
          </div>
          <p className="text-xs text-ink-faint">
            {s.voters} votants · {s.expressed} suffrages exprimés · {s.required}{' '}
            requis
          </p>
        </div>

        <div className="mt-4">
          <VoteBar tally={s.tally} />
        </div>

        <p className="mt-3 text-xs leading-relaxed text-ink-faint">
          {s.announcement} — les abstentions n'entrent pas dans les suffrages
          exprimés, et « non-votant » ne se lit pas comme une absence.{' '}
          <GuideLink ancre="q-non-votant">
            Ce que recouvrent les positions
          </GuideLink>
        </p>
      </Card>

      {/*
        La synthèse dit combien, l'hémicycle dit qui et où. Les deux ensemble
        montrent ce qu'aucun total ne montre : les fractures à l'intérieur des
        groupes, un non-votant isolé au milieu d'un bloc, une abstention unique.
      */}
      {votesNominaux.length > 0 && (
        <Card className="mb-5 px-5 py-4">
          <div className="mb-1 flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1">
            <h3
              id="hemicycle-scrutin"
              className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint"
            >
              Chaque député à sa place
            </h3>
            <span className="text-xs text-ink-faint">
              Survolez un siège pour lire le nom, le groupe et la position.
            </span>
          </div>

          {/* Borné : en pleine largeur, l'arc mesure sept cents pixels de haut
              et repousse la répartition par groupe hors de l'écran. */}
          <div className="mx-auto max-w-3xl">
            <Hemicycle votes={votesNominaux} labelledBy="hemicycle-scrutin" />
          </div>

          <div className="flex justify-center">
            <TallyLine tally={tallyDesVotes(votesNominaux)} />
          </div>

          <p className="mt-4 border-t border-line pt-3 text-xs leading-relaxed text-ink-faint">
            Chaque député est à sa place réelle : la source publie le numéro de
            siège avec chaque position, et le{' '}
            <a
              href="https://www.assemblee-nationale.fr/dyn/vos-deputes/hemicycle"
              target="_blank"
              rel="noreferrer"
              className="text-accent hover:underline"
            >
              plan de l'hémicycle
            </a>{' '}
            donne l'emplacement de ce numéro. En gris, les sièges dont aucune
            position n'a été enregistrée sur ce scrutin. Le décompte ci-dessus
            est celui des positions nominales — la synthèse officielle compte les
            non-votants volontaires à part.
          </p>
        </Card>
      )}

      {(data.cited.length > 0 || data.unnumbered_identical_amendments) && (
        <section className="mb-5">
          <SectionTitle>Ce qui a été mis aux voix</SectionTitle>
          <div className="space-y-2">
            {data.cited.map((c) => (
              <Card key={c.rank}>
                {c.amendment ? (
                  <div>
                    <p className="text-sm font-medium">
                      Amendement n° {c.amendment.number} —{' '}
                      {c.amendment.target_title}
                    </p>
                    {c.amendment.author_name && (
                      <p className="text-xs text-ink-faint">
                        {c.amendment.author_name}
                        {c.amendment.author_group_abbrev &&
                          ` (${c.amendment.author_group_abbrev})`}
                      </p>
                    )}
                    {c.amendment.summary && (
                      <p className="mt-2 whitespace-pre-line text-sm leading-relaxed">
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
                    {c.cited_number
                      ? `Amendement n° ${c.cited_number} : `
                      : ''}
                    {c.cause_label}
                  </p>
                )}
              </Card>
            ))}
            {data.unnumbered_identical_amendments && (
              <p className="text-xs text-ink-faint">
                L'objet annonce des amendements identiques sans en donner le
                numéro.
              </p>
            )}
          </div>
        </section>
      )}

      <section className="mb-5">
        <SectionTitle>Répartition par groupe</SectionTitle>
        <p className="mb-3 text-xs text-ink-faint">
          Groupe du député à la date du scrutin. Un groupe n'est pas un parti.{' '}
          <GuideLink ancre="q-groupe">Pourquoi cette distinction</GuideLink>
        </p>
        {/* Mention de méthode portée une fois pour toute la section : les
            chiffres marqués « reconstituée » sont produits par le site, pas
            publiés par l'Assemblée. */}
        {methodNote && (
          <p className="mb-3 rounded-lg bg-info-soft px-3 py-2 text-xs leading-relaxed text-info ring-1 ring-inset ring-info/15">
            {methodNote}
          </p>
        )}
        <div className="space-y-2">
          {data.groups.map((g, i) => (
            <GroupBlock
              key={g.group_uid ?? `orphans-${i}`}
              group={g}
              largestGroup={largestGroup}
            />
          ))}
        </div>
      </section>

      {data.corrections.length > 0 && (
        <section className="mb-5">
          <SectionTitle>Mises au point</SectionTitle>
          {/* Déclarations postérieures des députés. Elles ne modifient aucun
              décompte publié par l'Assemblée. */}
          <p className="mb-3 text-xs text-ink-faint">
            Déclarations postérieures au scrutin. Elles ne modifient ni la
            synthèse ni la répartition ci-dessus.
          </p>
          <div className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card">
            {data.corrections.map((c) => (
              <div
                key={c.actor_uid}
                className="flex items-center justify-between gap-3 px-4 py-1.5"
              >
                <span className="text-sm truncate">
                  {c.full_name ?? (
                    <span className="text-ink-faint font-mono text-xs">
                      {c.actor_uid}
                    </span>
                  )}
                  {c.malfunction && (
                    <span className="text-xs text-ink-faint ml-2">
                      dysfonctionnement signalé
                    </span>
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
