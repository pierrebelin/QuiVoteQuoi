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
import '@fontsource-variable/bricolage-grotesque/opsz.css'
import { LOGO_TRACES, LOGO_VUE } from '../../frontend/src/components/logoTraces'
import { PLAN_SIEGES, PLAN_VUE, RAYON_SIEGE } from '../../frontend/src/components/planHemicycle'
import { FONT, FONT_DISPLAY, HEIGHT, NIGHT, SAFE, WIDTH } from './theme'
import type { ScrutinDetailDto } from './types'

/**
 * Briques animées des Reels : fond plein, entrées à ressort, nombres qui
 * défilent, puis le décor bleu nuit du Reel de votes et de la carte finale
 * (fond, en-tête, hémicycle au plan réel de l'Assemblée).
 */

export const NAVY = '#16233f'
export const CREAM = '#ded3bd'
export const PAPER = '#faf9f6'
export const SEAT_EMPTY = '#e7e5e0'

/** Couleur d'une position sur le fond bleu nuit, et son fond doux. */
export const POSITION_COLORS: Record<string, string> = {
  for: NIGHT.for,
  against: NIGHT.against,
  abstention: NIGHT.abstention,
  not_voting: NIGHT.notVoting,
}
export const POSITION_SOFT: Record<string, string> = {
  for: NIGHT.forSoft,
  against: NIGHT.againstSoft,
  abstention: NIGHT.abstentionSoft,
}

export const capitalize = (text: string) => text.charAt(0).toUpperCase() + text.slice(1)

/** Sort publié ramené à sa couleur : `adopté` comme un pour, `rejeté` comme un contre, autre sort à part. */
export function outcomeTone(code: string): 'for' | 'against' | 'abstention' {
  if (code.startsWith('adopt')) return 'for'
  if (code.startsWith('rejet')) return 'against'
  return 'abstention'
}

/** Attend les polices Inter et Bricolage Grotesque avant la première image. */
export function useFonts() {
  const [handle] = useState(() => delayRender('Polices'))
  useEffect(() => {
    Promise.all([document.fonts.load('900 100px "Inter Variable"'), document.fonts.load('800 100px "Bricolage Grotesque Variable"')])
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

/** Post en 4:5 : aucune interface ne recouvre l'image, marges égales. */
export const POST_SAFE = { top: 64, bottom: 64, left: SAFE.left, right: SAFE.left }

/** Fond plein et zone de contenu hors des zones couvertes par les réseaux. */
export function Safe({ children, background, color }: { children: ReactNode; background: string; color: string }) {
  const zone = useVideoConfig().height < HEIGHT ? POST_SAFE : REEL_SAFE
  return (
    <AbsoluteFill style={{ backgroundColor: background, color, fontFamily: FONT }}>
      <div
        style={{
          position: 'absolute',
          top: zone.top,
          bottom: zone.bottom,
          left: zone.left,
          right: zone.right,
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

/* ------------------------------------------------------------- décor bleu nuit */

/** Hauteur de l'hémicycle rapportée à sa largeur. */
export const PLAN_RATIO = PLAN_VUE.hauteur / PLAN_VUE.largeur

/** Hauteur de l'en-tête (barre de progression, logo et nom), réservée en haut de la scène. */
export const TOP_BAR = 86
/** Durée du fondu de sortie d'une scène. */
export const FADE = 8

/**
 * Géométrie du décor bleu nuit, pour le Reel comme pour le post en 4:5 : zone
 * de contenu sous l'en-tête, et hémicycle « héros » posé au pied de cette zone,
 * à la même place à l'accroche et sur la carte finale pour que la boucle ne se
 * voie pas.
 */
export function useLayout() {
  const { height } = useVideoConfig()
  const post = height < HEIGHT
  const zone = post ? POST_SAFE : REEL_SAFE
  const width = WIDTH - zone.left - zone.right
  const heroWidth = post ? 820 : width + 92
  const heroHeight = (heroWidth * PLAN_VUE.hauteur) / PLAN_VUE.largeur
  return {
    post,
    zone,
    width,
    stage: { top: zone.top + TOP_BAR, height: height - zone.top - zone.bottom - TOP_BAR },
    hero: { left: zone.left + (width - heroWidth) / 2, top: height - zone.bottom - heroHeight + (post ? 38 : 33), width: heroWidth, height: heroHeight },
  }
}

/** Fond commun : bleu du logo, halo cyan derrière le contenu, arcs concentriques au pied de l'hémicycle. */
export function Night({ children }: { children: ReactNode }) {
  const { hero } = useLayout()
  const foot = { x: hero.left + hero.width / 2, y: hero.top + hero.height - 30 }
  return (
    <AbsoluteFill
      style={{
        backgroundColor: NIGHT.background,
        backgroundImage: `radial-gradient(ellipse 900px 760px at 50% 62%, ${NIGHT.glow}, rgba(22, 35, 63, 0) 70%), linear-gradient(180deg, ${NIGHT.backgroundDeep} 0%, ${NIGHT.background} 26%, ${NIGHT.background} 70%, ${NIGHT.backgroundDeep} 100%)`,
        color: NIGHT.ink,
        fontFamily: FONT,
        overflow: 'hidden',
      }}
    >
      {[620, 760, 900, 1040].map((radius) => (
        <div
          key={radius}
          style={{
            position: 'absolute',
            left: foot.x - radius,
            top: foot.y - radius,
            width: radius * 2,
            height: radius * 2,
            borderRadius: '50%',
            border: '2px solid rgba(255, 255, 255, 0.06)',
          }}
        />
      ))}
      {children}
    </AbsoluteFill>
  )
}

/** Zone de contenu d'une scène, sous l'en-tête et hors des zones couvertes par les réseaux. */
export function Stage({ children }: { children: ReactNode }) {
  const { zone, width, stage } = useLayout()
  return <div style={{ position: 'absolute', top: stage.top, height: stage.height, left: zone.left, width, display: 'flex', flexDirection: 'column' }}>{children}</div>
}

/**
 * Scène posée sur le décor commun : elle monte en apparaissant, puis s'efface
 * pendant ses `FADE` dernières images. Le fond, lui, ne bouge pas. Sur le post,
 * pas de sortie : l'image du carrousel est la dernière de la scène.
 */
export function Fade({ length, first = false, last = false, children }: { length: number; first?: boolean; last?: boolean; children: ReactNode }) {
  const frame = useCurrentFrame()
  const { post } = useLayout()
  const clamp = { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' } as const
  const enter = first ? 1 : interpolate(frame, [0, FADE + 2], [0, 1], { ...clamp, easing: Easing.out(Easing.cubic) })
  const exit = last || post ? 0 : interpolate(frame, [length - FADE, length], [0, 1], { ...clamp, easing: Easing.in(Easing.cubic) })
  return <AbsoluteFill style={{ opacity: enter * (1 - exit), transform: `translateY(${(1 - enter) * 44 - exit * 44}px)` }}>{children}</AbsoluteFill>
}

/** Logo du site sur son carré crème, comme sur les images de partage. */
function LogoTile({ size }: { size: number }) {
  const width = size * 0.82
  return (
    <div style={{ position: 'relative', width: size, height: size, borderRadius: size * 0.19, background: CREAM, flexShrink: 0 }}>
      <svg
        width={width}
        height={(width * LOGO_VUE.hauteur) / LOGO_VUE.largeur}
        viewBox={`0 0 ${LOGO_VUE.largeur} ${LOGO_VUE.hauteur}`}
        style={{ position: 'absolute', left: size * 0.09, top: size * 0.262 }}
      >
        {LOGO_TRACES.map(([d, color], i) => (
          <path key={i} d={d} fill={color} />
        ))}
      </svg>
    </div>
  )
}

/**
 * En-tête commun : une barre par scène, celle de la scène en cours se remplit
 * au fil de ses images ; dessous, le logo, le nom du site et un repère à droite.
 * `marks` porte la première image de chaque scène, `total` la durée du Reel.
 */
export function TopBar({ marks, total, labels }: { marks: number[]; total: number; labels: string[] }) {
  const frame = useCurrentFrame()
  const { zone, width } = useLayout()
  const current = Math.max(0, marks.findLastIndex((mark) => frame >= mark))
  return (
    <div style={{ position: 'absolute', top: zone.top, left: zone.left, width, display: 'flex', flexDirection: 'column', gap: 22 }}>
      <div style={{ display: 'flex', gap: 10 }}>
        {marks.map((mark, index) => {
          const end = marks[index + 1] ?? total
          const fill = interpolate(frame, [mark, end - 1], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
          return (
            <div key={mark} style={{ flex: 1, height: 8, borderRadius: 4, background: 'rgba(255, 255, 255, 0.2)', overflow: 'hidden' }}>
              <div style={{ width: `${fill * 100}%`, height: '100%', borderRadius: 4, background: index === current ? NIGHT.accent : 'rgba(255, 255, 255, 0.62)' }} />
            </div>
          )
        })}
      </div>
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', height: 56 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <LogoTile size={56} />
          <span style={{ fontFamily: FONT_DISPLAY, fontSize: 38, fontWeight: 700, whiteSpace: 'nowrap' }}>QuiVoteQuoi</span>
        </div>
        <span style={{ fontSize: 32, fontWeight: 600, color: NIGHT.inkSoft, whiteSpace: 'nowrap' }}>{labels[current] ?? ''}</span>
      </div>
    </div>
  )
}

/** Pastille d'intertitre, à la couleur d'accent du site. */
export function Kicker({ children }: { children: ReactNode }) {
  return (
    <span
      style={{
        alignSelf: 'flex-start',
        borderRadius: 999,
        background: NIGHT.accent,
        color: NIGHT.backgroundDeep,
        padding: '10px 28px 12px',
        fontSize: 32,
        fontWeight: 800,
        lineHeight: 1.2,
        letterSpacing: 2.5,
        textTransform: 'uppercase',
        whiteSpace: 'nowrap',
      }}
    >
      {children}
    </span>
  )
}

/** Coche d'un sort adopté, croix d'un sort rejeté ; rien pour un autre sort. */
export function OutcomeIcon({ code, size, color }: { code: string; size: number; color: string }) {
  const tone = outcomeTone(code)
  if (tone === 'abstention') return null
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke={color} strokeWidth={3.2} strokeLinecap="round" strokeLinejoin="round" style={{ display: 'block', flexShrink: 0 }}>
      <path d={tone === 'for' ? 'M5 12.5 L10 17.5 L19 7' : 'M6.5 6.5 L17.5 17.5 M17.5 6.5 L6.5 17.5'} />
    </svg>
  )
}

type Seat = { n: number; x: number; y: number; delay: number; vote?: { group: string; position: string } }

/** Sièges du plan, chacun avec le vote publié de son occupant ; délai d'allumage de gauche à droite. */
function seatsOf(detail: ScrutinDetailDto | null): Seat[] {
  const votes = new Map<number, { group: string; position: string }>()
  for (const group of detail?.groups ?? []) {
    for (const vote of group.votes) {
      if (vote.seat != null) votes.set(vote.seat, { group: group.color ?? NIGHT.notVoting, position: vote.position })
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
 * Hémicycle au plan réel, sur fond bleu nuit : chaque député qui a pris part au
 * vote s'allume à son siège, à la couleur de son groupe à partir de `light`
 * (négatif : déjà allumé), puis prend la couleur de sa position à partir de
 * `flip` (`null` : jamais ; 0 : d'emblée). Sans `detail`, le plan seul, pâle.
 */
export function Hemicycle({ detail, light, flip, stagger = 1 }: { detail: ScrutinDetailDto | null; light: number; flip: number | null; stagger?: number }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  return (
    <svg viewBox={`0 0 ${PLAN_VUE.largeur} ${PLAN_VUE.hauteur}`} style={{ width: '100%', display: 'block', overflow: 'visible' }}>
      {seatsOf(detail).map((seat) => {
        if (!detail) return <circle key={seat.n} cx={seat.x} cy={seat.y} r={RAYON_SIEGE * 0.95} fill={NIGHT.ink} />
        const empty = <circle cx={seat.x} cy={seat.y} r={RAYON_SIEGE * 0.7} fill={NIGHT.seatEmpty} />
        if (!seat.vote) return <g key={seat.n}>{empty}</g>
        const pop = spring({ frame: frame - light - seat.delay * stagger, fps, config: { damping: 10, stiffness: 180 } })
        const at = (flip ?? 0) + seat.delay * 0.5 * stagger
        const turn = flip === null ? 0 : interpolate(frame, [at, at + 10], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
        const fill = interpolateColors(turn, [0, 1], [seat.vote.group, POSITION_COLORS[seat.vote.position] ?? NIGHT.notVoting])
        return (
          <g key={seat.n}>
            {empty}
            {/* Liseré clair tant que le siège porte la couleur de son groupe : les teintes sombres restent lisibles sur le fond. */}
            <circle cx={seat.x} cy={seat.y} r={RAYON_SIEGE * 1.22 * pop} fill={fill} stroke="#ffffff" strokeOpacity={0.5 * (1 - turn)} strokeWidth={0.28} />
          </g>
        )
      })}
    </svg>
  )
}

/** Taille de l'adresse du site pour qu'elle tienne sur une ligne (Bricolage 800 : environ 0,56 em par caractère). */
function siteSize(site: string, width: number) {
  return Math.min(64, Math.floor(width / (site.length * 0.56)))
}

/**
 * Carte finale commune aux Reels : ce que le compte publie, l'invitation à
 * s'abonner, l'adresse du site, sur le décor bleu nuit. `detail` allume
 * l'hémicycle aux couleurs des groupes du scrutin montré en ouverture ;
 * `bar` dessine l'en-tête quand le Reel n'en porte pas déjà un.
 */
export function BrandOutro({ site, note, detail = null, bar = true }: { site: string; note: string | null; detail?: ScrutinDetailDto | null; bar?: boolean }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const { hero, width } = useLayout()
  const promise = useEnter(0, 50)
  const button = spring({ frame: frame - 10, fps, config: { damping: 13, stiffness: 170 } })
  const url = useEnter(16, 40)
  const seats = interpolate(frame, [0, 10], [0, 1], { extrapolateLeft: 'clamp', extrapolateRight: 'clamp' })
  return (
    <Night>
      <div style={{ position: 'absolute', left: hero.left, top: hero.top, width: hero.width, opacity: detail ? seats : seats * 0.5 }}>
        <Hemicycle detail={detail} light={-200} flip={null} />
      </div>
      {bar && <TopBar marks={[0]} total={2} labels={[site]} />}
      <Stage>
        <div style={{ ...promise, marginTop: 64, fontFamily: FONT_DISPLAY, fontSize: 96, fontWeight: 800, lineHeight: 0.98, letterSpacing: -2.5 }}>
          Chaque jour de séance, <span style={{ color: NIGHT.accent }}>tous les votes</span> de l’Assemblée.
        </div>
        <div
          style={{
            alignSelf: 'flex-start',
            marginTop: 44,
            transform: `scale(${button})`,
            transformOrigin: 'left center',
            borderRadius: 999,
            background: NIGHT.accent,
            color: NIGHT.backgroundDeep,
            padding: '16px 44px 20px',
            fontFamily: FONT_DISPLAY,
            fontSize: 56,
            fontWeight: 800,
            lineHeight: 1.15,
            letterSpacing: -0.5,
            whiteSpace: 'nowrap',
          }}
        >
          Abonnez-vous
        </div>
        <div style={{ ...url, marginTop: 34, display: 'flex', flexDirection: 'column', gap: 4 }}>
          <span style={{ fontFamily: FONT_DISPLAY, fontSize: siteSize(site, width), fontWeight: 800, lineHeight: 1.1, letterSpacing: -1, whiteSpace: 'nowrap' }}>{site}</span>
          <span style={{ fontSize: 34, fontWeight: 600, lineHeight: 1.3, color: NIGHT.inkSoft }}>Les chiffres et leurs sources · lien dans la bio</span>
          {note && <span style={{ marginTop: 10, fontSize: 28, lineHeight: 1.3, color: NIGHT.inkSoft }}>{note}</span>}
        </div>
      </Stage>
    </Night>
  )
}

/** Accroche des Reels hors votes : intertitre et filet, grand titre, texte facultatif, sur fond plein. */
export function TitleHook({ kicker, title, text, background, size }: { kicker: string; title: ReactNode; text?: ReactNode; background: string; size?: number }) {
  const frame = useCurrentFrame()
  const { fps } = useVideoConfig()
  const head = useEnter(0, 24)
  const heading = useEnter(8, 80)
  const body = useEnter(22, 40)
  const rule = spring({ frame: frame - 4, fps, config: { damping: 200 } })
  const fontSize = size ?? (typeof title === 'string' && title.length > 30 ? 118 : 140)
  return (
    <Safe background={background} color="#ffffff">
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 56 }}>
        <div style={{ ...head, display: 'flex', alignItems: 'center', gap: 24 }}>
          <span style={{ fontSize: 34, fontWeight: 800, letterSpacing: 5, textTransform: 'uppercase' }}>{kicker}</span>
          <span style={{ flex: 1, height: 3, background: '#ffffff', transform: `scaleX(${rule})`, transformOrigin: 'left' }} />
        </div>
        <div style={{ ...heading, fontSize, fontWeight: 900, lineHeight: 0.98, letterSpacing: -4 }}>{title}</div>
        {text && <div style={{ ...body, fontSize: 50, fontWeight: 600, lineHeight: 1.25, opacity: body.opacity * 0.9 }}>{text}</div>}
      </div>
    </Safe>
  )
}
