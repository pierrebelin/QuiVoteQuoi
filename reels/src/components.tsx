import { useEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react'
import {
  AbsoluteFill,
  Audio,
  cancelRender,
  continueRender,
  delayRender,
  interpolate,
  Sequence,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from 'remotion'
import '@fontsource-variable/inter'
import { chargerPolices, dessinerVote } from '../../frontend/src/lib/voteImage'
import { COLORS, FONT, SAFE } from './theme'
import { INTRO, OUTRO, type CardPlan } from './timeline'
import type { ReelProps, ScrutinDetailDto } from './types'

/** Fond et zone de contenu hors des zones couvertes par les réseaux. */
export function Frame({ children }: { children: ReactNode }) {
  return (
    <AbsoluteFill style={{ backgroundColor: COLORS.background, fontFamily: FONT, color: COLORS.ink }}>
      <div
        style={{
          position: 'absolute',
          top: SAFE.top,
          bottom: SAFE.bottom,
          left: SAFE.left,
          right: SAFE.right,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          gap: 40,
        }}
      >
        {children}
      </div>
    </AbsoluteFill>
  )
}

/** Apparition et disparition douces d'une scène de `length` images. */
export function useFade(length: number) {
  const frame = useCurrentFrame()
  const opacity = interpolate(frame, [0, 8, length - 8, length], [0, 1, 1, 0], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
  })
  const shift = interpolate(frame, [0, 10], [24, 0], { extrapolateRight: 'clamp' })
  return { opacity, transform: `translateY(${shift}px)` } satisfies CSSProperties
}

export function Kicker({ children }: { children: ReactNode }) {
  return (
    <p style={{ margin: 0, fontSize: 40, fontWeight: 600, color: COLORS.accent, letterSpacing: '-0.01em' }}>
      {children}
    </p>
  )
}

export function Title({ children, size = 84 }: { children: ReactNode; size?: number }) {
  return (
    <h1 style={{ margin: 0, fontSize: size, lineHeight: 1.08, fontWeight: 700, letterSpacing: '-0.025em' }}>
      {children}
    </h1>
  )
}

export function Note({ children }: { children: ReactNode }) {
  return <p style={{ margin: 0, fontSize: 32, lineHeight: 1.35, color: COLORS.inkSoft }}>{children}</p>
}

/** En-tête de période (première scène). */
export function Intro({ kicker, title }: { kicker: string; title: string }) {
  const style = useFade(INTRO)
  return (
    <Frame>
      <div style={{ ...style, display: 'flex', flexDirection: 'column', gap: 28 }}>
        <Kicker>{kicker}</Kicker>
        <Title size={104}>{title}</Title>
      </div>
    </Frame>
  )
}

/** Nombre publié, qui défile de zéro jusqu'à sa valeur. */
export function CountUp({ value, label, color, length }: { value: number; label: string; color: string; length: number }) {
  const frame = useCurrentFrame()
  const shown = Math.round(
    interpolate(frame, [6, Math.round(length * 0.55)], [0, value], {
      extrapolateLeft: 'clamp',
      extrapolateRight: 'clamp',
    }),
  )
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      <span style={{ fontSize: 168, lineHeight: 1, fontWeight: 700, letterSpacing: '-0.04em', color, fontVariantNumeric: 'tabular-nums' }}>
        {shown.toLocaleString('fr-FR')}
      </span>
      <span style={{ fontSize: 40, color: COLORS.inkSoft }}>{label}</span>
    </div>
  )
}

/** Polices chargées une fois pour tous les canvas de la vidéo. */
let fontsReady: Promise<void> | null = null
function loadFonts() {
  fontsReady ??= chargerPolices()
  return fontsReady
}

/**
 * Image d'un scrutin dessinée par `dessinerVote` au format `reel`, comme le
 * partage du site : groupes rangés par position publiée, décompte, source.
 */
export function VoteImage({ detail, site, scale = 1 }: { detail: ScrutinDetailDto; site: string; scale?: number }) {
  const canvas = useRef<HTMLCanvasElement>(null)
  const [handle] = useState(() => delayRender(`Dessin du scrutin ${detail.number}`))
  useEffect(() => {
    loadFonts()
      .then(() => {
        if (canvas.current) {
          dessinerVote(canvas.current, detail, {
            reseau: 'reel',
            visuel: 'groupes',
            theme: 'clair',
            lien: `${site}/scrutins/${detail.uid}`,
          })
        }
        continueRender(handle)
      })
      .catch((error: unknown) => cancelRender(error))
  }, [detail, handle, site])
  return <canvas ref={canvas} style={{ width: 1080 * scale, height: 1920 * scale, display: 'block' }} />
}

/** Images de scrutins à plein cadre, ou en grille quand une carte en porte plusieurs. */
export function VoteImages({ details, site }: { details: ScrutinDetailDto[]; site: string }) {
  if (details.length === 1) {
    return (
      <AbsoluteFill>
        <VoteImage detail={details[0]} site={site} />
      </AbsoluteFill>
    )
  }
  const columns = details.length <= 2 ? 2 : Math.ceil(Math.sqrt(details.length))
  const rows = Math.ceil(details.length / columns)
  const scale = Math.min(1 / columns, 1 / rows)
  return (
    <AbsoluteFill style={{ backgroundColor: COLORS.backgroundSoft, display: 'flex', flexWrap: 'wrap', alignContent: 'center', justifyContent: 'center' }}>
      {details.map((detail) => (
        <VoteImage key={detail.uid} detail={detail} site={site} scale={scale} />
      ))}
    </AbsoluteFill>
  )
}

/** Une `Sequence` par carte, dans l'ordre de l'API. */
export function Cards({ from, plan, render }: { from: number; plan: CardPlan; render: (indexes: number[]) => ReactNode }) {
  return (
    <>
      {plan.groups.map((indexes, card) => (
        <Sequence key={indexes.join('-')} from={from + card * plan.each} durationInFrames={plan.each}>
          <FadeIn length={plan.each}>{render(indexes)}</FadeIn>
        </Sequence>
      ))}
    </>
  )
}

export function FadeIn({ length, children }: { length: number; children: ReactNode }) {
  const { opacity } = useFade(length)
  return <AbsoluteFill style={{ opacity }}>{children}</AbsoluteFill>
}

/** Carte finale : l'adresse de la page qui porte les chiffres, et les mentions. */
export function Outro({ site, path, notes }: { site: string; path: string | null; notes: (string | null)[] }) {
  const style = useFade(OUTRO)
  return (
    <Frame>
      <div style={{ ...style, display: 'flex', flexDirection: 'column', gap: 36 }}>
        <Kicker>Tous les chiffres et leurs sources</Kicker>
        <Title size={88}>{site}</Title>
        {path && <p style={{ margin: 0, fontSize: 40, color: COLORS.inkSoft, wordBreak: 'break-all' }}>{path}</p>}
        {notes.filter((note): note is string => Boolean(note)).map((note) => (
          <Note key={note}>{note}</Note>
        ))}
      </div>
    </Frame>
  )
}

/** Piste libre de droits, absente : vidéo muette (T8). */
export function Soundtrack({ audio }: Pick<ReelProps, 'audio'>) {
  const { durationInFrames } = useVideoConfig()
  if (!audio) return null
  return (
    <Audio
      src={staticFile('piste.mp3')}
      volume={(frame) =>
        interpolate(frame, [0, 15, durationInFrames - 30, durationInFrames], [0, 0.6, 0.6, 0], {
          extrapolateLeft: 'clamp',
          extrapolateRight: 'clamp',
        })
      }
    />
  )
}
