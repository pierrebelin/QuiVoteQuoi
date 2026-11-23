import type { GroupPosition } from '../../lib/entryFlow'
import { SWATCH_CLASS } from './styles'

/** Glyphe d'une position ; vide pour « personne n'a voté » et « aucune ligne ». */
export default function PositionGlyph({
  position,
  className = 'size-[18px]',
  strokeWidth = 3,
}: {
  position: GroupPosition
  className?: string
  strokeWidth?: number
}) {
  const path = {
    for: 'M5 12.5l4.5 4.5L19 7.5',
    against: 'M6 6l12 12M18 6L6 18',
    abstention: 'M6 12h12',
    split: 'M6 9h12M6 15h12',
    nobody: null,
    missing: null,
  }[position]
  if (!path) return null
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={className}
    >
      <path d={path} />
    </svg>
  )
}

/**
 * Pastille d'état de la refonte A : un disque plein de 18 px, le glyphe de la
 * position dedans. Elle ne dit que l'état (pour, contre, abstention) ; le mot
 * qui la suit porte la même couleur, la couleur seule ne suffit pas.
 */
export function StateDot({ position }: { position: GroupPosition }) {
  return (
    <span
      aria-hidden
      className={`inline-flex size-[18px] shrink-0 items-center justify-center rounded-full ${SWATCH_CLASS[position]}`}
    >
      <PositionGlyph position={position} className="size-3" />
    </span>
  )
}
