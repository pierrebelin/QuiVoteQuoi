import { useQuery } from '@tanstack/react-query'
import { Link } from 'react-router'
import Hemicycle from './Hemicycle'
import { siegesDesGroupes } from './sieges'
import { Card, ErrorPanel, Pill, VoteBar } from './ui'
import type { DossierGroupActionsResponse, FinalVoteDto } from '../types/dossierGroupActions'
import type { ScrutinDetailDto } from '../types/scrutins'
import type { ParcoursActDto } from './DossierParcours'
import { readingKey, readingLabel } from '../lib/dossierStatus'

function latestFinalVote(groups: DossierGroupActionsResponse['groups']) {
  const byUid = new Map<string, FinalVoteDto>()
  groups.flatMap((group) => group.final_votes).forEach((vote) => {
    byUid.set(vote.scrutin_uid, vote)
  })

  return [...byUid.values()].sort(
    (left, right) =>
      right.date.localeCompare(left.date) || Number(right.number) - Number(left.number),
  )[0]
}

/** Dernière décision de séance que la source rattache à un scrutin public. */
function latestDecisionVote(acts: ParcoursActDto[]) {
  return [...acts].reverse().find((act) => act.decision_scrutin_uid)
}

interface ShownVote {
  scrutinUid: string
  reading: string | null | undefined
  /** Date de la décision que la source rattache à ce scrutin, s'il y en a une. */
  decisionDate?: string
}

/**
 * Le vote qui a tranché, avant les éléments de navigation. D'abord celui que
 * l'Assemblée rattache elle-même à sa dernière décision de séance (`voteRefs`) :
 * il couvre les rejets d'une première partie budgétaire ou par motion de rejet
 * préalable, qu'aucun vote « sur l'ensemble » ne porte. Sinon, ou quand un vote
 * sur l'ensemble est plus récent (décision sans scrutin cité), ce dernier. Les
 * autres votes finaux restent dans l'onglet des actes par groupe.
 */
export default function DossierFinalScrutin({ uid, acts }: { uid: string; acts: ParcoursActDto[] }) {
  const groupActions = useQuery<DossierGroupActionsResponse>({
    queryKey: ['dossier-lecture-groupes', uid],
    queryFn: () =>
      fetch(`/api/dossiers/${uid}/lecture-groupes`).then((response) => {
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        return response.json()
      }),
    enabled: !!uid,
  })
  const finalVote = groupActions.data ? latestFinalVote(groupActions.data.groups) : undefined
  const decision = latestDecisionVote(acts)
  const shown: ShownVote | undefined =
    decision?.decision_scrutin_uid && (!finalVote || decision.date >= finalVote.date)
      ? {
          scrutinUid: decision.decision_scrutin_uid,
          reading: readingLabel(readingKey(decision.code)),
          decisionDate: decision.date,
        }
      : finalVote && { scrutinUid: finalVote.scrutin_uid, reading: finalVote.reading }
  const scrutin = useQuery<ScrutinDetailDto>({
    queryKey: ['scrutin', shown?.scrutinUid],
    queryFn: () =>
      fetch(`/api/scrutins/${shown?.scrutinUid}`).then((response) => {
        if (!response.ok) throw new Error(`HTTP ${response.status}`)
        return response.json()
      }),
    enabled: !!shown,
  })

  if (groupActions.isLoading || (shown && scrutin.isLoading)) {
    return <p className="mb-6 animate-pulse text-sm text-ink-faint">Chargement du vote final…</p>
  }

  if (groupActions.isError || scrutin.isError) {
    return (
      <div className="mb-6">
        <ErrorPanel error={groupActions.error ?? scrutin.error} />
      </div>
    )
  }

  if (!shown || !scrutin.data) {
    return (
      <p className="mb-6 rounded border border-line bg-surface px-4 py-3 text-sm text-ink-soft">
        Aucun vote final publié n'est rattaché à ce texte. Les votes éventuellement disponibles restent consultables dans l'onglet dédié.
      </p>
    )
  }

  const data = scrutin.data
  const votes = siegesDesGroupes(data.groups)
  const longDate = (iso: string) =>
    new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' })
  // Pour une motion de rejet, « pour » vaut rejet du texte : le dire, sans
  // quoi l'hémicycle se lit à l'envers.
  const rejectionMotion = /^la motion de rejet/i.test(data.subject.trim())

  return (
    <section className="mb-6" aria-labelledby="dossier-scrutin-final">
      <div className="mb-2 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <div>
          <h3 id="dossier-scrutin-final" className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
            {shown.decisionDate ? 'Vote décisif' : 'Vote final publié'}
          </h3>
          <p className="mt-1 text-sm text-ink-soft">
            <Link to={`/scrutins/${data.uid}`} className="font-medium link">
              Scrutin n° {data.number}
            </Link>
            {' '}— {longDate(data.date)}
            {shown.reading && <> · {shown.reading}</>}
          </p>
        </div>
        <Pill tone={data.outcome_code === 'adopted' ? 'yes' : data.outcome_code === 'rejected' ? 'no' : 'neutral'}>
          {data.outcome_label}
        </Pill>
      </div>

      <Card className="px-4 py-3 sm:px-5 sm:py-4">
        <p className="text-sm leading-snug text-ink">{data.subject}</p>
        {(shown.decisionDate || rejectionMotion) && (
          <p className="mt-1 text-xs leading-snug text-ink-soft">
            {shown.decisionDate &&
              `L'Assemblée rattache à ce scrutin sa décision de séance du ${longDate(shown.decisionDate)}.`}
            {shown.decisionDate && rejectionMotion && ' '}
            {rejectionMotion && 'Voter pour la motion, c’est voter pour rejeter le texte.'}
          </p>
        )}
        <div className="mt-3 grid gap-4 lg:grid-cols-[minmax(0,1fr)_18rem] lg:items-center">
          <div className="mx-auto w-full max-w-3xl">
            {votes.length > 0 ? (
              <Hemicycle votes={votes} labelledBy="dossier-scrutin-final" />
            ) : (
              <p className="rounded border border-line bg-surface-soft px-3 py-2 text-sm text-ink-soft">
                La source ne publie aucune position nominale exploitable pour représenter l'hémicycle de ce vote.
              </p>
            )}
          </div>
          <div className="space-y-3 border-t border-line pt-3 lg:border-t-0 lg:border-l lg:pl-4 lg:pt-0">
            <div className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
              <span><b className="font-semibold text-yes">{data.synthesis.tally.votes_for}</b> pour</span>
              <span><b className="font-semibold text-no">{data.synthesis.tally.votes_against}</b> contre</span>
              <span><b className="font-semibold text-abstain">{data.synthesis.tally.abstentions}</b> abstentions</span>
              <span><b className="font-semibold text-ink-soft">{data.synthesis.tally.not_voting}</b> non-votants</span>
            </div>
            <VoteBar tally={data.synthesis.tally} />
            <a href={data.official_url} target="_blank" rel="noreferrer" className="text-xs link">
              Source officielle ↗
            </a>
          </div>
        </div>
      </Card>
    </section>
  )
}
