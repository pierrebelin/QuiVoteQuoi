import type { ComponentType } from 'react'
import { Composition, type CalculateMetadataFunction } from 'remotion'
import nouveaute from '../samples/nouveaute.json'
import programmes from '../samples/programmes.json'
import theme from '../samples/theme.json'
import votes from '../samples/votes.json'
import { Programmes } from './compositions/Programmes'
import { SiteNews } from './compositions/SiteNews'
import { Theme } from './compositions/Theme'
import { Votes } from './compositions/Votes'
import { FPS, HEIGHT, WIDTH } from './theme'
import { scenesOf } from './timeline'
import type { ReelProps } from './types'

/** La durée suit le nombre de cartes : 90 s au plus pour l'ensemble des cartes (T7). */
const calculateMetadata: CalculateMetadataFunction<ReelProps> = ({ props }) => ({
  durationInFrames: scenesOf(props).total,
})

/**
 * Les exemples de `samples/` servent au studio et à la vérification 1 :
 * `votes.json` reprend la semaine publique du 16 novembre 2026, les trois
 * autres sont illustratifs. En production, `scripts/prepare.mjs` passe les
 * propriétés lues sur l'API (`--props`).
 */
const COMPOSITIONS: { id: string; component: ComponentType<ReelProps>; sample: unknown }[] = [
  { id: 'Votes', component: Votes, sample: votes },
  { id: 'Programmes', component: Programmes, sample: programmes },
  { id: 'SiteNews', component: SiteNews, sample: nouveaute },
  { id: 'Theme', component: Theme, sample: theme },
]

export function Root() {
  return (
    <>
      {COMPOSITIONS.map(({ id, component, sample }) => (
        <Composition
          key={id}
          id={id}
          component={component}
          width={WIDTH}
          height={HEIGHT}
          fps={FPS}
          durationInFrames={1}
          defaultProps={sample as ReelProps}
          calculateMetadata={calculateMetadata}
        />
      ))}
    </>
  )
}
