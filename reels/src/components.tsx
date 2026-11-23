import { Audio, interpolate, staticFile, useVideoConfig } from 'remotion'
import type { ReelProps } from './types'

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
