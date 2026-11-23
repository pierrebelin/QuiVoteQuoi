import { useEffect, useState, type ReactNode } from 'react'
import {
  AbsoluteFill,
  continueRender,
  delayRender,
  interpolate,
  interpolateColors,
  spring,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion'
import '@fontsource-variable/inter'
import { LOGO_TRACES, LOGO_VUE } from '../../frontend/src/components/logoTraces'
import { PLAN_SIEGES, PLAN_VUE, RAYON_SIEGE } from '../../frontend/src/components/planHemicycle'
import { COLORS, FONT, SAFE } from './theme'
import type { ScrutinDetailDto } from './types'

/**
 * Briques animées du Reel de votes : fond plein, entrées à ressort, nombres qui
 * défilent, hémicycle au plan réel de l'Assemblée et logo sur fond sombre.
 */

export const NAVY = '#16233f'
export const CREAM = '#ded3bd'
export const PAPER = '#faf9f6'
export const SEAT_EMPTY = '#e7e5e0'
export const NOT_VOTING = '#a1a1aa'
/** Sort et décompte sur fond bleu nuit : teintes éclaircies, même sens que `COLORS`. */
export const ON_NAVY = { adopted: '#34d399', rejected: '#f87171' }

export const POSITION_COLORS: Record<string, string> = {
  for: COLORS.adopted,
  against: COLORS.rejected,
  abstention: COLORS.other,
  not_voting: NOT_VOTING,
}

export const capitalize = (text: string) => text.charAt(0).toUpperCase() + text.slice(1)

/** Couleur d'un sort publié (`adopté`, `rejeté`, autre). */
export function outcomeColor(code: string) {
  if (code.startsWith('adopt')) return COLORS.adopted
  if (code.startsWith('rejet')) return COLORS.rejected
  return COLORS.other
}

/** Attend la police Inter avant la première image. */
export function useFonts() {
  const [handle] = useState(() => delayRender('Police Inter'))
  useEffect(() => {
    document.fonts
      .load('900 100px "Inter Variable"')
      .then(() => document.fonts.ready)
      .then(() => continueRender(handle))
  }, [handle])
}

/** Entrée d'un élément : opacité et glissement, avec un ressort. */
export function useEnter(delay: number, distance = 60) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const p = spring({ frame: frame - delay, fps, config: { damping: 200 } })
  return { opacity: p, transform: `translateY(${(1 - p) * distance}px)` }
}

/** Nombre publié, qui défile de zéro jusqu'à sa valeur à partir de `start`. */
export function BigCount({ value, start, duration = 30 }: { value: number; start: number; duration?: number }) {
  const frame = useCurrentFrame()
  const t = interpolate(frame, [start, start + duration], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
  return <>{Math.round(value * (1 - Math.pow(1 - t, 3))).toLocaleString('fr-FR')}</>
}

/**
 * Zone de contenu du Reel de votes : plus haute que `SAFE` (gabarit `vertical`
 * du site), elle ne laisse libres que l'en-tête des réseaux (200 px) et le
 * nom du compte, la légende repliée et les boutons (380 px en bas).
 */
export const REEL_SAFE = { top: 200, bottom: 380, left: SAFE.left, right: SAFE.right }

/** Fond plein et zone de contenu hors des zones couvertes par les réseaux. */
export function Safe({ children, background, color }: { children: ReactNode; background: string; color: string }) {
  return (
    <AbsoluteFill style={{ backgroundColor: background, color, fontFamily: FONT }}>
      <div
        style={{
          position: 'absolute',
          top: REEL_SAFE.top,
          bottom: REEL_SAFE.bottom,
          left: REEL_SAFE.left,
          right: REEL_SAFE.right,
          display: 'flex',
          flexDirection: 'column',
        }}
      >
        {children}
      </div>
    </AbsoluteFill>
  )
}

/** Fondu d'entrée et de sortie d'une scène de `length` images. */
export function Fade({ length, enter = false, children }: { length: number; enter?: boolean; children: ReactNode }) {
  const frame = useCurrentFrame()
  const opacity = interpolate(frame, [0, 6, length - 8, length], [enter ? 0 : 1, 1, 1, 0], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  })
  return <AbsoluteFill style={{ opacity }}>{children}</AbsoluteFill>
}

type Seat = { n: number; x: number; y: number; delay: number; vote?: { group: string; position: string } }

/** Sièges du plan, chacun avec le vote publié de son occupant ; délai d'allumage de gauche à droite. */
function seatsOf(detail: ScrutinDetailDto): Seat[] {
  const votes = new Map<number, { group: string; position: string }>()
  for (const group of detail.groups) {
    for (const vote of group.votes) {
      if (vote.seat != null) votes.set(vote.seat, { group: group.color ?? NOT_VOTING, position: vote.position })
    }
  }
  const cx = PLAN_VUE.largeur / 2
  const cy = PLAN_VUE.hauteur
  return Object.entries(PLAN_SIEGES).map(([key, [x, y]]) => {
    const angle = Math.atan2(cy - y, x - cx)
    return { n: Number(key), x, y, delay: ((Math.PI - angle) / Math.PI) * 45, vote: votes.get(Number(key)) }
  })
}

/**
 * Hémicycle au plan réel : chaque député qui a pris part au vote s'allume à son
 * siège, à la couleur de son groupe à partir de `light`, puis prend la couleur
 * de sa position à partir de `flip` (`flip` à 0 : couleur de position d'emblée).
 */
export function Hemicycle({ detail, light, flip, stagger = 1 }: { detail: ScrutinDetailDto; light: number; flip: number; stagger?: number }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const base = interpolate(frame, [0, 12], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
  return (
    <svg viewBox={`0 0 ${PLAN_VUE.largeur} ${PLAN_VUE.hauteur}`} style={{ width: '100%', display: 'block', overflow: 'visible' }}>
      {seatsOf(detail).map((seat) => {
        const empty = <circle cx={seat.x} cy={seat.y} r={RAYON_SIEGE * 0.7} fill={SEAT_EMPTY} opacity={base} />
        if (!seat.vote) return <g key={seat.n}>{empty}</g>
        const pop = spring({ frame: frame - light - seat.delay * stagger, fps, config: { damping: 10, stiffness: 180 } })
        const at = flip + seat.delay * 0.5 * stagger
        const turn = interpolate(frame, [at, at + 10], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
        const fill = interpolateColors(turn, [0, 1], [seat.vote.group, POSITION_COLORS[seat.vote.position] ?? NOT_VOTING])
        return (
          <g key={seat.n}>
            {empty}
            <circle cx={seat.x} cy={seat.y} r={RAYON_SIEGE * 1.25 * pop} fill={fill} />
          </g>
        )
      })}
    </svg>
  )
}

/** Logo sur fond sombre : les rayons bleu nuit passent en blanc, les autres gardent leur teinte. */
export function Logo({ height }: { height: number }) {
  const width = (height * LOGO_VUE.largeur) / LOGO_VUE.hauteur
  return (
    <svg width={width} height={height} style={{ marginLeft: -width * 0.18, display: 'block' }} viewBox={`0 0 ${LOGO_VUE.largeur} ${LOGO_VUE.hauteur}`}>
      {LOGO_TRACES.map(([d, color], i) => (
        <path key={i} d={d} fill={color === NAVY ? '#ffffff' : color} />
      ))}
    </svg>
  )
}
