import { Sequence } from 'remotion'
import { CountUp, Frame, Intro, Outro, Soundtrack, useFade } from '../components'
import { COLORS } from '../theme'
import { INTRO, NUMBERS, scenesOf } from '../timeline'
import type { ReelProps, ReelThemeDto } from '../types'

function Numbers({ theme }: { theme: ReelThemeDto }) {
  const style = useFade(NUMBERS)
  return (
    <Frame>
      <div style={{ ...style, display: 'flex', flexDirection: 'column', gap: 56 }}>
        <CountUp value={theme.text_count} label={theme.text_count > 1 ? 'textes' : 'texte'} color={COLORS.ink} length={NUMBERS} />
        <CountUp
          value={theme.scrutin_count}
          label={theme.scrutin_count > 1 ? 'scrutins classés' : 'scrutin classé'}
          color={COLORS.accent}
          length={NUMBERS}
        />
      </div>
    </Frame>
  )
}

/** Reel « Thème » (RM-07) : les nombres publiés sur la page du thème, puis son adresse. */
export function Theme(props: ReelProps) {
  const { reel, site } = props
  const scenes = scenesOf(props)
  const theme = reel.theme
  if (!theme) throw new Error('Reel sans thème')

  return (
    <>
      <Soundtrack audio={props.audio} />
      <Sequence durationInFrames={INTRO}>
        <Intro kicker="Un thème, tous ses votes" title={theme.label} />
      </Sequence>
      <Sequence from={INTRO} durationInFrames={NUMBERS}>
        <Numbers theme={theme} />
      </Sequence>
      <Sequence from={INTRO + scenes.numbers}>
        <Outro site={site} path={reel.link_path} notes={[]} />
      </Sequence>
    </>
  )
}
