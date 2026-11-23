import { Sequence } from 'remotion'
import { Soundtrack } from '../components'
import { BigCount, BrandOutro, NAVY, PAPER, Push, Safe, SEAT_EMPTY, TitleHook, useEnter, useFonts } from '../motion'
import { COLORS } from '../theme'
import { PUSH, scenesOf, TITLE_HOOK, VOTES_OUTRO } from '../timeline'
import type { ReelProgrammeDto, ReelProps } from '../types'

const plural = (count: number, one: string, many: string) => (count > 1 ? many : one)

/** Une candidature : son nom, les extraits ajoutés et retirés, ses thèmes. Couleurs neutres : ni gain ni perte (RM-11). */
function Candidature({ programme, themeLabels, compact }: { programme: ReelProgrammeDto; themeLabels: ReelProps['themeLabels']; compact: boolean }) {
  const name = useEnter(0, 60)
  const counts = useEnter(10, 60)
  const themes = useEnter(20, 40)
  const unit = compact ? 0.62 : 1
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 44 * unit }}>
      <div style={{ ...name, fontSize: 104 * unit, fontWeight: 900, lineHeight: 1, letterSpacing: -3 }}>{programme.name}</div>
      <div style={{ ...counts, display: 'flex', gap: 80 * unit }}>
        {[
          { value: programme.added, label: plural(programme.added, 'extrait ajouté', 'extraits ajoutés'), color: COLORS.accent },
          { value: programme.removed, label: plural(programme.removed, 'extrait retiré', 'extraits retirés'), color: NAVY },
        ].map((item) => (
          <div key={item.label} style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <span style={{ fontSize: 200 * unit, fontWeight: 900, lineHeight: 0.85, letterSpacing: -6, color: item.color, fontVariantNumeric: 'tabular-nums' }}>
              <BigCount value={item.value} start={12} duration={20} />
            </span>
            <span style={{ fontSize: 46 * unit, fontWeight: 800, color: item.color }}>{item.label}</span>
          </div>
        ))}
      </div>
      {programme.themes.length > 0 && (
        <div style={{ ...themes, display: 'flex', flexWrap: 'wrap', gap: 14 }}>
          {programme.themes.map((code) => (
            <span key={code} style={{ fontSize: 34 * Math.max(unit, 0.8), fontWeight: 700, padding: '10px 24px', borderRadius: 999, background: SEAT_EMPTY }}>
              {themeLabels[code] ?? code}
            </span>
          ))}
        </div>
      )}
    </div>
  )
}

function Card({ programmes, themeLabels }: { programmes: ReelProgrammeDto[]; themeLabels: ReelProps['themeLabels'] }) {
  const head = useEnter(0, 24)
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ ...head, display: 'flex', alignItems: 'center', gap: 24 }}>
        <span style={{ fontSize: 34, fontWeight: 800, letterSpacing: 5, textTransform: 'uppercase' }}>Programme modifié</span>
        <span style={{ flex: 1, height: 3, background: NAVY }} />
      </div>
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'space-evenly' }}>
        {programmes.map((programme) => (
          <Candidature key={programme.candidate_id} programme={programme} themeLabels={themeLabels} compact={programmes.length > 1} />
        ))}
      </div>
    </Safe>
  )
}

/**
 * Reel « Programmes » (RM-06) : le nombre de candidatures changées dans la
 * période, puis chacune dans l'ordre de l'API (alphabétique), sans
 * comparaison, score ni résumé, puis l'adresse du site.
 */
export function Programmes(props: ReelProps) {
  useFonts()
  const { reel, site, themeLabels } = props
  const scenes = scenesOf(props)
  const count = reel.programmes.length
  const outroFrom = TITLE_HOOK + scenes.cards.groups.length * scenes.cards.each

  return (
    <>
      <Soundtrack audio={props.audio} />
      <Sequence durationInFrames={TITLE_HOOK + PUSH}>
        <Push length={TITLE_HOOK} overlap={PUSH} first>
          <TitleHook
            kicker="Programmes 2027"
            background={COLORS.accent}
            size={96}
            title={
              <>
                <span style={{ display: 'block', fontSize: 300, letterSpacing: -12, lineHeight: 0.85 }}>
                  <BigCount value={count} start={10} />
                </span>
                {plural(count, 'candidature a modifié son programme', 'candidatures ont modifié leur programme')}
              </>
            }
            text={reel.period.label}
          />
        </Push>
      </Sequence>
      {scenes.cards.groups.map((indexes, card) => (
        <Sequence key={indexes.join('-')} from={TITLE_HOOK + card * scenes.cards.each} durationInFrames={scenes.cards.each + PUSH}>
          <Push length={scenes.cards.each} overlap={PUSH} axis={card === 0 ? 'y' : 'x'}>
            <Card programmes={indexes.map((i) => reel.programmes[i])} themeLabels={themeLabels} />
          </Push>
        </Sequence>
      ))}
      <Sequence from={outroFrom} durationInFrames={VOTES_OUTRO}>
        <Push length={VOTES_OUTRO} overlap={PUSH} last>
          <BrandOutro site={site} linkPath={reel.link_path} note={null} />
        </Push>
      </Sequence>
    </>
  )
}
