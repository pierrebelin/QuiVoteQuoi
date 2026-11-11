import { useEffect, useRef } from 'react'
import { Popover } from '@base-ui/react/popover'
import { BottomSheet } from '../BottomSheet'
import { Icon } from '../icons'
import { PoliticalLogo } from '../PoliticalLogo'
import { VoteBar } from '../ui'
import { GROUP_LOGOS } from '../../content/logos'
import {
  groupPosition,
  publishedMajorityNote,
  stanceSentence,
  stanceTurnout,
} from '../../lib/entryFlow'
import { useMediaQuery } from '../../lib/useMediaQuery'
import type { GroupDto, StanceDto } from '../../types/finalVotes'
import { FOCUS_RING } from './styles'

const VIEWPORT_MARGIN = 12
const ANCHOR_GAP = 9

function GroupLogo({ group }: { group: GroupDto }) {
  return (
    <PoliticalLogo
      logo={GROUP_LOGOS[group.abbrev]}
      fallback={group.abbrev}
      color={null}
      className="h-7 w-14 shrink-0"
    />
  )
}

/**
 * Barre et lignes chiffrées d'un groupe sur un vote. Les parts sont celles de
 * l'API, rapportées aux votants : la barre écarte donc les non-votants, signalés
 * à part, pour rester alignée sur les pourcentages.
 */
function BreakdownFigures({ stance }: { stance: StanceDto | undefined }) {
  const position = groupPosition(stance)
  const note = publishedMajorityNote(stance)
  const nonVoters = stance?.tally.not_voting ?? 0

  return (
    <div className="space-y-2.5 text-sm">
      {!stance || position === 'nobody' || position === 'missing' ? (
        <p className="text-ink-soft">{stanceSentence(stance)}</p>
      ) : (
        <>
          <VoteBar tally={{ ...stance.tally, not_voting: 0 }} thick />
          <ul className="space-y-1">
            {[
              {
                key: 'for',
                label: 'Pour',
                count: stance.tally.votes_for,
                percent: stance.share?.for_percent,
                swatch: 'bg-yes-bar',
              },
              {
                key: 'against',
                label: 'Contre',
                count: stance.tally.votes_against,
                percent: stance.share?.against_percent,
                swatch: 'bg-no-bar',
              },
              {
                key: 'abstention',
                label: stance.tally.abstentions > 1 ? 'Abstentions' : 'Abstention',
                count: stance.tally.abstentions,
                percent: stance.share?.abstention_percent,
                swatch: 'bg-abstain-bar',
              },
            ].map((row) => (
              <li
                key={row.key}
                className={`grid grid-cols-[12px_1fr_auto_44px] items-center gap-2 ${
                  row.count === 0 ? 'text-ink-faint' : ''
                }`}
              >
                <span
                  aria-hidden
                  className={`size-2.5 rounded-[3px] ${row.swatch} ${row.count === 0 ? 'opacity-35' : ''}`}
                />
                <span>{row.label}</span>
                <span className={`tabular-nums ${row.count === 0 ? 'font-medium' : 'font-bold'}`}>
                  {row.count}
                </span>
                <span className="text-right text-[13px] text-ink-soft tabular-nums">
                  {row.percent ?? 0}&nbsp;%
                </span>
              </li>
            ))}
          </ul>
        </>
      )}
      {nonVoters > 0 && (
        <p className="text-xs text-ink-soft">
          {nonVoters}&nbsp;non-votant{nonVoters > 1 ? 's' : ''} en plus.
        </p>
      )}
      {note && (
        <p className="rounded bg-surface-soft px-2.5 py-2 text-xs leading-snug text-ink-soft">
          {note}.
        </p>
      )}
    </div>
  )
}

/**
 * Bulle ancrée au-dessus de la tuile, en dessous quand la place manque. Sans
 * fond qui capte les clics : une autre tuile reste cliquable et remplace la bulle.
 * Placée par rapport à l'écran, elle se ferme au défilement plutôt que de se
 * décoller de sa tuile.
 */
function BreakdownPopover({
  group,
  stance,
  anchor,
  onClose,
}: {
  group: GroupDto
  stance: StanceDto | undefined
  anchor: HTMLElement
  onClose: () => void
}) {
  const closeButtonRef = useRef<HTMLButtonElement>(null)
  const turnout = stanceTurnout(stance)

  useEffect(() => {
    window.addEventListener('scroll', onClose, true)
    window.addEventListener('resize', onClose)
    return () => {
      window.removeEventListener('scroll', onClose, true)
      window.removeEventListener('resize', onClose)
    }
  }, [onClose])

  return (
    <Popover.Root
      open
      // La bulle n'a pas de déclencheur à elle : la tuile reste gérée par la
      // carte. Une tape sur une tuile de la même rangée remplace la bulle au lieu
      // de la fermer ; Échap et « Fermer » rendent le focus à la tuile.
      modal="trap-focus"
      onOpenChange={(open, details) => {
        if (open) return
        if (
          details.reason === 'outside-press' &&
          anchor.closest('ul')?.contains(details.event.target as Node)
        ) {
          details.cancel()
          return
        }
        onClose()
        if (details.reason === 'escape-key' || details.reason === 'close-press') {
          anchor.focus({ preventScroll: true })
        }
      }}
    >
      <Popover.Portal>
        <Popover.Positioner
          anchor={anchor}
          positionMethod="fixed"
          side="top"
          sideOffset={ANCHOR_GAP}
          collisionPadding={VIEWPORT_MARGIN}
          className="z-[60]"
        >
          <Popover.Popup
            aria-label={`Répartition des votes : ${group.label}`}
            initialFocus={closeButtonRef}
            finalFocus={false}
            className="w-80 rounded border border-line-strong bg-surface px-4 py-3.5 text-ink shadow-[0_12px_34px_rgb(0_0_0/0.3)] outline-none"
          >
            <Popover.Arrow
              className={(state) =>
                `size-3 rotate-45 bg-surface ${
                  state.side === 'bottom'
                    ? '-top-[7px] border-t border-l border-line-strong'
                    : '-bottom-[7px] border-r border-b border-line-strong'
                }`
              }
            />
            <div className="mb-3 flex items-start gap-2.5">
              <GroupLogo group={group} />
              <div className="min-w-0 flex-1">
                <p className="text-sm leading-tight font-semibold">{group.label}</p>
                {turnout && <p className="mt-0.5 text-xs text-ink-soft">{turnout}</p>}
              </div>
              <Popover.Close
                ref={closeButtonRef}
                aria-label="Fermer"
                className={`flex size-7 shrink-0 cursor-pointer items-center justify-center rounded text-ink-soft hover:bg-surface-soft hover:text-ink [&_svg]:size-4 ${FOCUS_RING}`}
              >
                <Icon name="close" />
              </Popover.Close>
            </div>
            <BreakdownFigures stance={stance} />
          </Popover.Popup>
        </Popover.Positioner>
      </Popover.Portal>
    </Popover.Root>
  )
}

/** Répartition d'un groupe : bulle à partir de 768 px, tiroir bas en dessous. */
export default function GroupBreakdown({
  group,
  stance,
  anchor,
  onClose,
}: {
  group: GroupDto
  stance: StanceDto | undefined
  anchor: HTMLElement
  onClose: () => void
}) {
  const wide = useMediaQuery('(min-width: 768px)')
  const turnout = stanceTurnout(stance)

  if (wide) {
    return <BreakdownPopover group={group} stance={stance} anchor={anchor} onClose={onClose} />
  }

  return (
    <BottomSheet open onClose={onClose} title={group.label}>
      <div className="min-h-0 space-y-3 overflow-y-auto overscroll-contain px-4 pt-1 pb-5">
        <div className="flex items-center gap-2.5">
          <GroupLogo group={group} />
          {turnout && <p className="text-[13px] text-ink-soft">{turnout}</p>}
        </div>
        <BreakdownFigures stance={stance} />
      </div>
    </BottomSheet>
  )
}
