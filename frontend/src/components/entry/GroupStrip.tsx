import { useCallback, useState } from 'react'
import { PoliticalLogo } from '../PoliticalLogo'
import { GROUP_LOGOS } from '../../content/logos'
import type { FinalVoteDto, GroupDto } from '../../types/finalVotes'
import { findStance, groupPosition, stanceDescription } from '../../lib/entryFlow'
import GroupBreakdown from './GroupBreakdown'
import PositionGlyph from './PositionGlyph'
import { FOCUS_RING, SWATCH_CLASS } from './styles'

/**
 * Les douze groupes sur une seule ligne, dans l'ordre de l'hémicycle. Chaque
 * tuile : le logo sur une plaque blanche, puis la position en couleur, sans mot.
 * Un clic sur une tuile affiche la répartition des votes du groupe.
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
  const [opened, setOpened] = useState<{ group: GroupDto; anchor: HTMLElement } | null>(null)
  const close = useCallback(() => setOpened(null), [])

  return (
    <>
      <ul className="grid grid-cols-4 gap-1 md:grid-cols-12">
        {groups.map((group) => {
          const stance = findStance(vote, group)
          const position = groupPosition(stance)
          const on = followed.includes(group.abbrev)
          const isOpen = opened?.group.uid === group.uid
          const description = stanceDescription(group, stance)
          return (
            <li key={group.uid} className="min-w-0">
              <button
                type="button"
                aria-label={description}
                aria-haspopup="dialog"
                aria-expanded={isOpen}
                title={description}
                onClick={(event) =>
                  setOpened(isOpen ? null : { group, anchor: event.currentTarget })
                }
                className={`flex w-full cursor-pointer flex-col gap-1 rounded-xl border-2 p-[3px] transition-[opacity,background-color] hover:bg-surface-soft ${FOCUS_RING} ${
                  on ? 'border-ink bg-surface-soft' : 'border-transparent'
                } ${isOpen ? 'bg-surface-soft outline-2 outline-offset-1 outline-accent' : ''} ${
                  dimOthers && !on && !isOpen ? 'opacity-30 hover:opacity-70' : ''
                }`}
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
              </button>
            </li>
          )
        })}
      </ul>
      {opened && (
        <GroupBreakdown
          group={opened.group}
          stance={findStance(vote, opened.group)}
          anchor={opened.anchor}
          onClose={close}
        />
      )}
    </>
  )
}
