import { useEffect, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
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
        <p className="rounded-lg bg-surface-soft px-2.5 py-2 text-xs leading-snug text-ink-soft">
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
  const bubbleRef = useRef<HTMLDivElement>(null)
  const closeButtonRef = useRef<HTMLButtonElement>(null)
  const [place, setPlace] = useState({ top: 0, left: 0, arrow: 0, below: false })
  const turnout = stanceTurnout(stance)

  // Avant peinture : la hauteur dépend du groupe, aucun saut visible.
  useLayoutEffect(() => {
    const bubble = bubbleRef.current
    if (!bubble) return
    const tile = anchor.getBoundingClientRect()
    const center = tile.left + tile.width / 2
    const left = Math.max(
      VIEWPORT_MARGIN,
      Math.min(
        window.innerWidth - bubble.offsetWidth - VIEWPORT_MARGIN,
        center - bubble.offsetWidth / 2,
      ),
    )
    const above = tile.top - ANCHOR_GAP - bubble.offsetHeight
    const below = above < VIEWPORT_MARGIN
    setPlace({ top: below ? tile.bottom + ANCHOR_GAP : above, left, arrow: center - left, below })
  }, [anchor, stance])

  useEffect(() => {
    closeButtonRef.current?.focus({ preventScroll: true })
  }, [anchor])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose()
        anchor.focus({ preventScroll: true })
      } else if (event.key === 'Tab') {
        // Le seul élément focalisable est le bouton de fermeture.
        event.preventDefault()
      }
    }
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target as Node
      if (bubbleRef.current?.contains(target) || anchor.closest('ul')?.contains(target)) return
      onClose()
    }
    document.addEventListener('keydown', onKeyDown)
    document.addEventListener('pointerdown', onPointerDown)
    window.addEventListener('scroll', onClose, true)
    window.addEventListener('resize', onClose)
    return () => {
      document.removeEventListener('keydown', onKeyDown)
      document.removeEventListener('pointerdown', onPointerDown)
      window.removeEventListener('scroll', onClose, true)
      window.removeEventListener('resize', onClose)
    }
  }, [anchor, onClose])

  return createPortal(
    <div
      ref={bubbleRef}
      role="dialog"
      aria-label={`Répartition des votes : ${group.label}`}
      style={{ top: place.top, left: place.left }}
      className="fixed z-[60] w-80 rounded-[14px] border border-line-strong bg-surface px-4 py-3.5 text-ink shadow-[0_12px_34px_rgb(0_0_0/0.3)]"
    >
      <span
        aria-hidden
        style={{ left: place.arrow }}
        className={`absolute -ml-1.5 size-3 rotate-45 bg-surface ${
          place.below
            ? '-top-[7px] border-t border-l border-line-strong'
            : '-bottom-[7px] border-r border-b border-line-strong'
        }`}
      />
      <div className="mb-3 flex items-start gap-2.5">
        <GroupLogo group={group} />
        <div className="min-w-0 flex-1">
          <p className="text-sm leading-tight font-semibold">{group.label}</p>
          {turnout && <p className="mt-0.5 text-xs text-ink-soft">{turnout}</p>}
        </div>
        <button
          ref={closeButtonRef}
          type="button"
          aria-label="Fermer"
          onClick={() => {
            onClose()
            anchor.focus({ preventScroll: true })
          }}
          className={`flex size-7 shrink-0 cursor-pointer items-center justify-center rounded-lg text-ink-soft hover:bg-surface-soft hover:text-ink [&_svg]:size-4 ${FOCUS_RING}`}
        >
          <Icon name="close" />
        </button>
      </div>
      <BreakdownFigures stance={stance} />
    </div>,
    document.body,
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
