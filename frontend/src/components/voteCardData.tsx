import { capitalized, inHemicycleOrder, typographic } from '../lib/entryFlow'
import type { FinalVoteDto, GroupDto, StanceDto } from '../types/finalVotes'
import type { ScrutinDetailDto, ScrutinSummaryDto } from '../types/scrutins'
import type { TextScrutinDto } from '../types/themes'
import CitedAuthor from './CitedAuthor'
import type { VoteCardProps } from './VoteCard'

/*
 * Données de la carte de vote, tirées des deux formes que sert l'API : un vote
 * sur l'ensemble (`/api/votes-finaux`) et un scrutin de la liste
 * (`/api/scrutins`). Aucun libellé n'est réécrit : le titre est l'intitulé
 * publié, seule sa première lettre passe en capitale (README.md §6).
 */

type CardData = Omit<VoteCardProps, 'size' | 'bare'>

/** Accord de l'issue avec le texte voté : « la proposition » / « le projet ». */
function agreed(adopted: boolean, label: string): string {
  const feminine = /^(proposition|motion|résolution)\b/i.test(label.trim())
  return `${adopted ? 'Adopté' : 'Rejeté'}${feminine ? 'e' : ''}`
}

export const WHOLE_TEXT_KICKER = 'Vote sur l’ensemble du texte'

/** Vote sur l'ensemble, avec la position de chaque groupe du référentiel. */
export function finalVoteCard(
  vote: FinalVoteDto,
  groups: GroupDto[],
  followed: string[] = [],
): CardData {
  return {
    to: `/scrutins/${vote.scrutin_uid}`,
    adopted: vote.adopted,
    outcome: agreed(vote.adopted, vote.text_label),
    outcomeTitle: capitalized(vote.outcome_label),
    reading: vote.reading,
    date: vote.date,
    kicker: WHOLE_TEXT_KICKER,
    title: capitalized(typographic(vote.text_label.trim())),
    tally: vote.synthesis,
    groups: { groups: inHemicycleOrder(groups), stances: vote.stances, followed },
  }
}

function ballotKind(label: string) {
  return label.replace(/^scrutin public /, '')
}

/** Un scrutin de la liste, en carte compacte. */
export function scrutinCard(scrutin: ScrutinSummaryDto): CardData {
  const adopted = scrutin.outcome_code === 'adopté'
  const cited = scrutin.cited[0]
  const kind = ballotKind(scrutin.ballot_type)
  const kicker = cited
    ? `Amendement n° ${cited.number} · ${cited.target_title}`
    : scrutin.short_label !== scrutin.subject
      ? capitalized(scrutin.short_label)
      : null
  const others = scrutin.cited.length - 1

  return {
    to: `/scrutins/${scrutin.uid}`,
    adopted,
    outcome: adopted ? 'Adopté' : 'Rejeté',
    outcomeTitle: capitalized(scrutin.outcome_label),
    date: scrutin.date,
    kicker: kicker && typographic(kicker),
    title: capitalized(typographic(scrutin.subject.trim())),
    author: cited && <CitedAuthor cited={cited} />,
    excerpt: cited?.summary_preview,
    notes: (kind !== 'ordinaire' || others > 0 || scrutin.has_reconstructed_tallies) && (
      <p className="flex flex-wrap gap-x-2 text-xs text-ink-faint">
        {kind !== 'ordinaire' && <span>Scrutin {kind}</span>}
        {others > 0 && (
          <span>
            et {others} autre{others > 1 ? 's' : ''} amendement{others > 1 ? 's' : ''} mis aux voix
            ensemble
          </span>
        )}
        {scrutin.has_reconstructed_tallies && (
          <span
            className="text-info"
            title="La source ne publie pas les groupes sur ce vote : la répartition est reconstituée."
          >
            Répartition reconstituée
          </span>
        )}
      </p>
    ),
    tally: scrutin.tally,
  }
}

/** Le scrutin porte-t-il sur l'ensemble du texte ? Lu sur l'objet publié. */
export function isWholeTextVote(subject: string): boolean {
  return /^l['’]ensemble\b/i.test(subject.trim())
}

/**
 * Positions des groupes d'un scrutin détaillé, sous la forme des votes sur
 * l'ensemble. La position de tête se lit sur les voix du groupe ; la position
 * publiée par l'Assemblée est gardée à part, comme ailleurs.
 */
function stancesOf(detail: ScrutinDetailDto): { groups: GroupDto[]; stances: StanceDto[] } {
  const groups: GroupDto[] = []
  const stances: StanceDto[] = []
  detail.groups.forEach((breakdown, index) => {
    const uid = breakdown.group_uid ?? breakdown.abbrev ?? `groupe-${index}`
    const abbrev = breakdown.abbrev ?? uid
    const label = breakdown.label ?? abbrev
    const { votes_for, votes_against, abstentions } = breakdown.tally
    const voters = votes_for + votes_against + abstentions
    const counts = [
      ['for', 'pour', votes_for],
      ['against', 'contre', votes_against],
      ['abstention', 'abstention', abstentions],
    ] as const
    const top = Math.max(votes_for, votes_against, abstentions)
    const leaders = counts.filter(([, , count]) => count === top)
    const percent = (count: number) => (voters > 0 ? Math.round((count * 100) / voters) : 0)
    const majority = breakdown.majority_position
    groups.push({ uid, abbrev, label, color: breakdown.color, final_vote_count: 0 })
    stances.push({
      group_uid: uid,
      abbrev,
      label,
      color: breakdown.color,
      member_count: breakdown.member_count,
      majority,
      majority_label: counts.find(([key]) => key === majority)?.[1] ?? null,
      tally: breakdown.tally,
      share:
        voters === 0
          ? null
          : {
              voters,
              for_percent: percent(votes_for),
              against_percent: percent(votes_against),
              abstention_percent: percent(abstentions),
              leading: leaders.length === 1 ? leaders[0][0] : null,
              leading_label: leaders.length === 1 ? leaders[0][1] : null,
              leading_percent: leaders.length === 1 ? percent(top) : null,
              tied_labels: leaders.map(([, word]) => word),
            },
    })
  })
  return { groups, stances }
}

/** Un scrutin détaillé en carte complète : ses groupes rangés par position. */
export function scrutinDetailCard(detail: ScrutinDetailDto): CardData {
  const adopted = detail.outcome_code === 'adopté'
  const { groups, stances } = stancesOf(detail)
  return {
    to: `/scrutins/${detail.uid}`,
    adopted,
    outcome: adopted ? 'Adopté' : 'Rejeté',
    outcomeTitle: capitalized(detail.outcome_label),
    reading: detail.reading,
    date: detail.date,
    kicker: isWholeTextVote(detail.subject) ? WHOLE_TEXT_KICKER : null,
    title: capitalized(typographic(detail.subject.trim())),
    tally: detail.synthesis.tally,
    groups: { groups: inHemicycleOrder(groups), stances },
  }
}

/** Un scrutin de la fiche de texte, en carte compacte. */
export function textScrutinCard(scrutin: TextScrutinDto): CardData {
  // La fiche de texte ne sert que le libellé de l'issue : « … n'a pas adopté ».
  const adopted = !/n['’]a pas adopt/i.test(scrutin.outcome_label)
  return {
    to: `/scrutins/${scrutin.uid}`,
    adopted,
    outcome: adopted ? 'Adopté' : 'Rejeté',
    outcomeTitle: capitalized(scrutin.outcome_label),
    date: scrutin.date,
    title: capitalized(typographic(scrutin.subject.trim())),
    tally: {
      votes_for: scrutin.votes_for,
      votes_against: scrutin.votes_against,
      abstentions: scrutin.abstentions,
      not_voting: 0,
    },
  }
}
