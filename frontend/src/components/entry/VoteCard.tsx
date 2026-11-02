import { useId, useState } from 'react'
import { Link } from 'react-router'
import { Card, Pill, VoteBar } from '../ui'
import type { FinalVoteDto, GroupDto } from '../../types/finalVotes'
import { formatDate } from '../../types/scrutins'
import {
  capitalized,
  findStance,
  groupPosition,
  groupShortName,
  stanceSentence,
  voteTitle,
} from '../../lib/entryFlow'
import GroupStrip from './GroupStrip'
import PositionGlyph from './PositionGlyph'
import { BADGE_CLASS, BADGE_WORD, BANNER_CLASS, FOCUS_RING } from './styles'

/*
 * « Adoptée » est l'issue publiée de ce scrutin, pour cette lecture : le texte
 * peut encore être examiné ailleurs. Les deux mentions sont affichées avec la
 * pastille, pas reléguées dans une page de méthode (README.md §7).
 */
const OUTCOME = {
  adopted: {
    label: 'Adoptée par l’Assemblée',
    caption: 'pas forcément la loi définitive',
    question: 'Pourquoi « pas forcément la loi définitive » ?',
    explanation:
      '« Adoptée » veut dire que l’Assemblée a voté pour ce texte à cette étape. Ce n’est pas forcément la loi définitive : le Sénat peut encore l’examiner et le modifier.',
  },
  rejected: {
    label: 'Rejetée par l’Assemblée',
    caption: 'pour cette lecture seulement',
    question: 'Que veut dire « rejetée » ?',
    explanation:
      '« Rejetée » est l’issue publiée de ce scrutin, pour cette lecture à l’Assemblée. Ce n’est pas l’état final du texte.',
  },
} as const

function assemblyLabel(vote: FinalVoteDto): string {
  const { votes_for, votes_against, abstentions, not_voting, voluntary_not_voting } = vote.synthesis
  const nonVoters = not_voting + voluntary_not_voting
  return [
    `Toute l’Assemblée : ${votes_for} pour`,
    `${votes_against} contre`,
    `${abstentions} abstention${abstentions > 1 ? 's' : ''}`,
    ...(nonVoters > 0 ? [`${nonVoters} non-votant${nonVoters > 1 ? 's' : ''}`] : []),
  ].join(', ')
}

/** Bandeau du groupe suivi quand un seul l'est : sa position, puis ses chiffres bruts. */
function FollowedBanner({ vote, group }: { vote: FinalVoteDto; group: GroupDto }) {
  const stance = findStance(vote, group)
  const position = groupPosition(stance)
  return (
    <div
      className={`flex flex-wrap items-center gap-x-3 gap-y-2 rounded-xl px-3 py-2.5 ${BANNER_CLASS[position]}`}
    >
      <span
        className={`min-w-[84px] shrink-0 rounded-lg px-3 py-1.5 text-center text-[15px] font-bold tracking-wide uppercase ${BADGE_CLASS[position]}`}
      >
        {BADGE_WORD[position]}
      </span>
      <p className="min-w-0 flex-1 text-[15px] font-semibold">
        <span className="sr-only">{groupShortName(group)} : </span>
        {stanceSentence(stance)}
      </p>
    </div>
  )
}

/**
 * Un vote sur l'ensemble : issue, date, titre, puis la position des douze
 * groupes. La carte est la même quel que soit le groupe suivi : le suivi change
 * la mise en valeur, jamais la sélection ni l'ordre des votes.
 */
export default function VoteCard({
  vote,
  groups,
  followed,
}: {
  vote: FinalVoteDto
  groups: GroupDto[]
  /** Sigles suivis ; vide = tous. */
  followed: string[]
}) {
  const [infoOpen, setInfoOpen] = useState(false)
  const infoId = useId()
  const outcome = vote.adopted ? OUTCOME.adopted : OUTCOME.rejected
  const followedGroups = groups.filter((group) => followed.includes(group.abbrev))
  const focus = followedGroups.length === 1 ? followedGroups[0] : null

  return (
    <Card className="flex flex-col gap-3.5 px-4 py-4 sm:px-[22px] sm:py-[18px]">
      <div className="space-y-1.5">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-1.5 text-[13px] text-ink-soft">
          <Pill tone={vote.adopted ? 'yes' : 'no'}>
            <PositionGlyph
              position={vote.adopted ? 'for' : 'against'}
              className="mr-1 size-3"
              strokeWidth={3.2}
            />
            {outcome.label}
          </Pill>
          <span className="inline-flex items-center gap-1.5">
            {outcome.caption}
            <button
              type="button"
              aria-expanded={infoOpen}
              aria-controls={infoId}
              aria-label={outcome.question}
              onClick={() => setInfoOpen((open) => !open)}
              // Le rond reste petit, la zone de toucher dépasse 44 px.
              className={`relative flex size-[22px] shrink-0 items-center justify-center rounded-full border border-line-strong bg-surface text-xs font-bold text-ink-soft hover:bg-surface-soft after:absolute after:-inset-3 after:content-[''] ${FOCUS_RING}`}
            >
              ?
            </button>
          </span>
          <span aria-hidden>·</span>
          <time dateTime={vote.date}>{formatDate(vote.date)}</time>
          {vote.reading && <Pill tone="info">{capitalized(vote.reading)}</Pill>}
        </div>

        <h3 className="text-[19px] leading-snug font-semibold tracking-tight">
          <Link
            to={`/scrutins/${vote.scrutin_uid}`}
            title={vote.text_label}
            className={`-my-2 block rounded-md py-2 hover:text-accent ${FOCUS_RING}`}
          >
            {voteTitle(vote)}
            <svg
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth={2.2}
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden
              className="ml-1.5 inline size-4 align-[-2px] text-accent"
            >
              <path d="M7 17L17 7M9 7h8v8" />
            </svg>
          </Link>
        </h3>
      </div>

      {infoOpen && (
        <p
          id={infoId}
          className="rounded-lg bg-surface-soft px-3 py-2.5 text-[13px] leading-relaxed text-ink-soft"
        >
          {outcome.explanation}
        </p>
      )}

      {focus && <FollowedBanner vote={vote} group={focus} />}

      <GroupStrip vote={vote} groups={groups} followed={followed} />

      <div className="flex items-center gap-2.5">
        <span className="shrink-0 text-xs font-semibold text-ink-soft">Toute l’Assemblée</span>
        <div className="min-w-0 flex-1">
          <VoteBar
            tally={{ ...vote.synthesis, not_voting: 0, voluntary_not_voting: 0 }}
            label={assemblyLabel(vote)}
          />
        </div>
      </div>
    </Card>
  )
}
