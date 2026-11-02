import { PoliticalLogo } from '../PoliticalLogo'
import { GROUP_LOGOS } from '../../content/logos'
import type { FinalVoteDto, GroupDto } from '../../types/finalVotes'
import { findStance, groupPosition, stanceDescription } from '../../lib/entryFlow'
import PositionGlyph from './PositionGlyph'
import { SWATCH_CLASS } from './styles'

/**
 * Les douze groupes sur une seule ligne, dans l'ordre de l'hémicycle. Chaque
 * tuile : le logo sur une plaque blanche, puis la position en couleur, sans mot.
 *
 * Les groupes suivis portent un anneau sombre ; quand certains sont suivis, les
 * autres s'estompent sans disparaître : rien n'est retiré (README.md §2).
 */
export default function GroupStrip({
  vote,
  groups,
  followed,
}: {
  vote: FinalVoteDto
  groups: GroupDto[]
  followed: string[]
}) {
  const dimOthers = followed.length > 0

  return (
    <ul className="grid grid-cols-4 gap-1 md:grid-cols-12">
      {groups.map((group) => {
        const stance = findStance(vote, group)
        const position = groupPosition(stance)
        const on = followed.includes(group.abbrev)
        const description = stanceDescription(group, stance)
        return (
          <li key={group.uid} className="min-w-0">
            <div
              role="img"
              aria-label={description}
              title={description}
              className={`flex flex-col gap-1 rounded-xl border-2 p-[3px] transition-opacity ${
                on ? 'border-ink bg-surface-soft' : 'border-transparent'
              } ${dimOthers && !on ? 'opacity-30' : ''}`}
            >
              <PoliticalLogo
                logo={GROUP_LOGOS[group.abbrev]}
                fallback={group.abbrev}
                color={null}
                className="h-7 w-full"
              />
              <span
                className={`flex h-[34px] items-center justify-center rounded-lg ${SWATCH_CLASS[position]}`}
              >
                <PositionGlyph position={position} />
              </span>
            </div>
          </li>
        )
      })}
    </ul>
  )
}
