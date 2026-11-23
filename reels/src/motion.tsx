import { useEffect, useState, type ReactNode } from 'react'
import {
  AbsoluteFill,
  continueRender,
  Easing,
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

/**
 * Scène qui entre en poussant la précédente : elle glisse depuis le bas (ou la
 * droite, `axis="x"`) pendant `overlap` images, puis, passé `length`, recule et
 * s'assombrit sous la suivante. Un léger zoom continu évite l'image figée.
 * La `Sequence` qui la porte dure `length + overlap` images, sauf la dernière.
 */
export function Push({
  length,
  overlap,
  axis = 'y',
  first = false,
  last = false,
  children,
}: {
  length: number
  overlap: number
  axis?: 'x' | 'y'
  first?: boolean
  last?: boolean
  children: ReactNode
}) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const enter = first ? 1 : spring({ frame, fps, config: { damping: 200 }, durationInFrames: overlap })
  const exit = last
    ? 0
    : interpolate(frame, [length, length + overlap], [0, 1], {
        extrapolateLeft: 'clamp',
        extrapolateRight: 'clamp',
        easing: Easing.inOut(Easing.cubic),
      })
  const offset = (1 - enter) * 100 - exit * 35
  const drift = 1 + 0.025 * Math.min(1, frame / (length + overlap))
  return (
    <AbsoluteFill style={{ transform: axis === 'y' ? `translateY(${offset}%)` : `translateX(${offset}%)`, overflow: 'hidden' }}>
      <AbsoluteFill style={{ transform: `scale(${drift})` }}>{children}</AbsoluteFill>
      {exit > 0 && <AbsoluteFill style={{ backgroundColor: '#000000', opacity: exit * 0.45 }} />}
    </AbsoluteFill>
  )
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

/** Centre et rayon extérieur du plan, en unités de `PLAN_VUE`. */
const PLAN_CENTRE = { x: PLAN_VUE.largeur / 2, y: PLAN_VUE.hauteur }
const PLAN_RAYON = Math.max(...Object.values(PLAN_SIEGES).map(([x, y]) => Math.hypot(x - PLAN_CENTRE.x, PLAN_CENTRE.y - y)))

export type ArcLabel = { uid: string; abbrev: string; color: string; left: number; top: number; angle: number }

/**
 * Étiquette de chaque groupe votant, posée hors de l'arc au droit de ses sièges
 * (angle moyen des sièges de ses votants), écartée de ses voisines si besoin.
 * `left` et `top` en pourcentage de la boîte de l'hémicycle.
 */
export function arcLabels(detail: ScrutinDetailDto, gap = 0.2): ArcLabel[] {
  const labels = detail.groups.flatMap((group) => {
    const angles = group.votes
      .map((vote) => (vote.seat != null ? PLAN_SIEGES[vote.seat] : undefined))
      .filter((xy): xy is readonly [number, number] => xy !== undefined)
      .map(([x, y]) => Math.atan2(PLAN_CENTRE.y - y, x - PLAN_CENTRE.x))
    if (angles.length === 0 || !group.abbrev) return []
    const angle = angles.reduce((sum, a) => sum + a, 0) / angles.length
    return [{ uid: group.group_uid ?? group.abbrev, abbrev: group.abbrev, color: group.color ?? NOT_VOTING, angle }]
  })
  labels.sort((a, b) => a.angle - b.angle)
  const min = 0.4
  const max = Math.PI - 0.4
  for (let i = 0; i < labels.length; i++) labels[i].angle = Math.max(labels[i].angle, i === 0 ? min : labels[i - 1].angle + gap)
  for (let i = labels.length - 1; i >= 0; i--) labels[i].angle = Math.min(labels[i].angle, i === labels.length - 1 ? max : labels[i + 1].angle - gap)
  const radius = PLAN_RAYON + 13
  return labels.map((label) => ({
    ...label,
    left: ((PLAN_CENTRE.x + radius * Math.cos(label.angle)) / PLAN_VUE.largeur) * 100,
    top: ((PLAN_CENTRE.y - radius * Math.sin(label.angle)) / PLAN_VUE.hauteur) * 100,
  }))
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

/** Taille de l'adresse du site pour qu'elle tienne sur une ligne (Inter 900 : environ 0,6 em par caractère). */
function siteSize(site: string) {
  return Math.min(112, Math.floor((1080 - REEL_SAFE.left - REEL_SAFE.right) / (site.length * 0.6)))
}

/** Carte finale commune aux Reels : logo, nom, adresse du site et chemin de la page. */
export function BrandOutro({ site, linkPath, note }: { site: string; linkPath: string | null; note: string | null }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const logo = spring({ frame, fps, config: { damping: 14 } })
  const name = useEnter(6, 40)
  const url = useEnter(16, 40)
  return (
    <Safe background={NAVY} color="#ffffff">
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 90 }}>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 30 }}>
          <div style={{ transform: `scale(${logo})`, transformOrigin: 'left bottom' }}>
            <Logo height={270} />
          </div>
          <div style={{ ...name, fontSize: 140, fontWeight: 900, letterSpacing: -4, lineHeight: 1 }}>
            QuiVote<span style={{ color: '#60a5fa' }}>Quoi</span>
          </div>
        </div>
        <div style={{ ...url, display: 'flex', flexDirection: 'column', gap: 16 }}>
          <span style={{ fontSize: 46, fontWeight: 700, color: 'rgba(255,255,255,0.7)' }}>Tous les chiffres, avec leurs sources</span>
          <span style={{ fontSize: siteSize(site), fontWeight: 900, letterSpacing: -2, color: CREAM, lineHeight: 1, whiteSpace: 'nowrap' }}>{site}</span>
          {linkPath && <span style={{ fontSize: 36, color: 'rgba(255,255,255,0.6)', wordBreak: 'break-all' }}>{linkPath}</span>}
          {note && <span style={{ fontSize: 28, lineHeight: 1.3, color: 'rgba(255,255,255,0.7)' }}>{note}</span>}
        </div>
      </div>
    </Safe>
  )
}
