import { Sequence } from 'remotion'
import { Cards, CountUp, Frame, Intro, Kicker, Outro, Soundtrack, Title, useFade } from '../components'
import { COLORS } from '../theme'
import { INTRO, NUMBERS, scenesOf } from '../timeline'
import type { ReelProgrammeDto, ReelProps } from '../types'

function extracts(count: number, verb: string) {
  return `${count} extrait${count > 1 ? 's' : ''} ${verb}${count > 1 ? 's' : ''}`
}

function Candidature({ programme, themeLabels }: { programme: ReelProgrammeDto; themeLabels: ReelProps['themeLabels'] }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
      <Title size={72}>{programme.name}</Title>
      <p style={{ margin: 0, fontSize: 44, fontWeight: 600 }}>
        {extracts(programme.added, 'ajouté')} · {extracts(programme.removed, 'retiré')}
      </p>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 14 }}>
        {programme.themes.map((code) => (
          <span key={code} style={{ fontSize: 32, padding: '8px 20px', borderRadius: 999, backgroundColor: COLORS.backgroundSoft, color: COLORS.inkSoft }}>
            {themeLabels[code] ?? code}
          </span>
        ))}
      </div>
    </div>
  )
}

function Count({ count }: { count: number }) {
  const style = useFade(NUMBERS)
  return (
    <Frame>
      <div style={style}>
        <CountUp
          value={count}
          label={count > 1 ? 'candidatures ont modifié leur programme' : 'candidature a modifié son programme'}
          color={COLORS.ink}
          length={NUMBERS}
        />
      </div>
    </Frame>
  )
}

/**
 * Reel « Programmes » (RM-06) : chaque candidature changée dans la période,
 * dans l'ordre de l'API (alphabétique), sans comparaison, score ni résumé.
 */
export function Programmes(props: ReelProps) {
  const { reel, site, themeLabels } = props
  const scenes = scenesOf(props)
  const cardsFrom = INTRO + scenes.numbers
  const outroFrom = cardsFrom + scenes.cards.groups.length * scenes.cards.each

  return (
    <>
      <Soundtrack audio={props.audio} />
      <Sequence durationInFrames={INTRO}>
        <Intro kicker="Programmes 2027" title={reel.period.label} />
      </Sequence>
      <Sequence from={INTRO} durationInFrames={NUMBERS}>
        <Count count={reel.programmes.length} />
      </Sequence>
      <Cards
        from={cardsFrom}
        plan={scenes.cards}
        render={(indexes) => (
          <Frame>
            <Kicker>Changements publiés sur le site</Kicker>
            {indexes.map((i) => (
              <Candidature key={reel.programmes[i].candidate_id} programme={reel.programmes[i]} themeLabels={themeLabels} />
            ))}
          </Frame>
        )}
      />
      <Sequence from={outroFrom}>
        <Outro site={site} path={reel.link_path} notes={[]} />
      </Sequence>
    </>
  )
}
