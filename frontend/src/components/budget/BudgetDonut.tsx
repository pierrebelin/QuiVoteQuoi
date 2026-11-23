import type { ReactNode } from 'react'
import { sliceColor, type Slice } from '../../lib/budgetSlices'

/*
 * Anneau de répartition du budget : les plus grosses parts une à une, le reste
 * regroupé en une part grise. Au-delà de six parts, un anneau ne se lit plus ;
 * la liste complète reste sous l'anneau, rien n'est retiré (README §2).
 * Parts et couleurs : lib/budgetSlices.ts.
 */

const CENTER = 100
const OUTER = 98
const INNER = 60

function point(radius: number, turn: number): string {
  const angle = 2 * Math.PI * turn - Math.PI / 2
  return `${(CENTER + radius * Math.cos(angle)).toFixed(2)} ${(CENTER + radius * Math.sin(angle)).toFixed(2)}`
}

/** Secteur d'anneau entre deux fractions de tour (0 = midi, sens horaire). */
function sector(from: number, to: number): string {
  // Un arc SVG ne sait pas faire un tour complet : une part unique s'arrête juste avant.
  const end = Math.min(to, from + 0.99999)
  const large = end - from > 0.5 ? 1 : 0
  return [
    `M ${point(OUTER, from)}`,
    `A ${OUTER} ${OUTER} 0 ${large} 1 ${point(OUTER, end)}`,
    `L ${point(INNER, end)}`,
    `A ${INNER} ${INNER} 0 ${large} 0 ${point(INNER, from)}`,
    'Z',
  ].join(' ')
}

export function Donut<T>({
  slices,
  size,
  label,
  onCard = false,
  children,
}: {
  slices: Slice<T>[]
  size: number
  /** Nom accessible : les parts et leur poids, l'anneau n'ayant pas de texte. */
  label: string
  /** Anneau posé sur une carte grise : le liseré entre les parts prend son gris, non le blanc de la page. */
  onCard?: boolean
  /** Chiffre posé au centre de l'anneau. */
  children: ReactNode
}) {
  const total = slices.reduce((sum, slice) => sum + slice.euros, 0)
  let turn = 0
  const paths = slices.flatMap((slice) => {
    const from = turn
    turn += total > 0 ? slice.euros / total : 0
    if (turn - from <= 0) return []
    return [
      <path
        key={slice.key}
        d={sector(from, turn)}
        fill={sliceColor(slice.colorIndex)}
        stroke={onCard ? 'var(--color-surface)' : 'var(--color-canvas)'}
        strokeWidth={2}
        strokeLinejoin="round"
      />,
    ]
  })
  return (
    <div className="relative mx-auto shrink-0" style={{ width: size, height: size }}>
      <svg viewBox="0 0 200 200" role="img" aria-label={label} width={size} height={size}>
        {paths}
      </svg>
      <div
        aria-hidden="true"
        className="absolute inset-0 flex flex-col items-center justify-center text-center"
      >
        {children}
      </div>
    </div>
  )
}

export function Swatch({ colorIndex }: { colorIndex: number }) {
  return (
    <span
      aria-hidden="true"
      className="size-3 shrink-0 rounded-[4px]"
      style={{ backgroundColor: sliceColor(colorIndex) }}
    />
  )
}
