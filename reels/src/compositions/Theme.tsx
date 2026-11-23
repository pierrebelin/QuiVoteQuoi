import { Sequence } from 'remotion'
import { Soundtrack } from '../components'
import { BigCount, BrandOutro, NAVY, PAPER, Push, Safe, TitleHook, useEnter, useFonts } from '../motion'
import { COLORS } from '../theme'
import { NUMBERS, PUSH, TITLE_HOOK, VOTES_OUTRO } from '../timeline'
import type { ReelProps, ReelThemeDto } from '../types'

function Numbers({ theme }: { theme: ReelThemeDto }) {
  const texts = useEnter(4, 80)
  const scrutins = useEnter(16, 80)
  const items = [
    { style: texts, value: theme.text_count, label: theme.text_count > 1 ? 'textes' : 'texte', color: NAVY, start: 6 },
    { style: scrutins, value: theme.scrutin_count, label: theme.scrutin_count > 1 ? 'scrutins classés' : 'scrutin classé', color: COLORS.accent, start: 18 },
  ]
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', justifyContent: 'center', gap: 90 }}>
        {items.map((item) => (
          <div key={item.label} style={{ ...item.style, display: 'flex', flexDirection: 'column', gap: 8 }}>
            <span style={{ fontSize: 260, fontWeight: 900, lineHeight: 0.85, letterSpacing: -10, color: item.color, fontVariantNumeric: 'tabular-nums' }}>
              <BigCount value={item.value} start={item.start} />
            </span>
            <span style={{ fontSize: 64, fontWeight: 800 }}>{item.label}</span>
          </div>
        ))}
      </div>
    </Safe>
  )
}

/** Reel « Thème » (RM-07) : le thème, les nombres publiés sur sa page, puis l'adresse du site. */
export function Theme(props: ReelProps) {
  useFonts()
  const { reel, site } = props
  const theme = reel.theme
  if (!theme) throw new Error('Reel sans thème')

  return (
    <>
      <Soundtrack audio={props.audio} />
      <Sequence durationInFrames={TITLE_HOOK + PUSH}>
        <Push length={TITLE_HOOK} overlap={PUSH} first>
          <TitleHook kicker="Un thème, tous ses votes" title={theme.label} background={NAVY} />
        </Push>
      </Sequence>
      <Sequence from={TITLE_HOOK} durationInFrames={NUMBERS + PUSH}>
        <Push length={NUMBERS} overlap={PUSH}>
          <Numbers theme={theme} />
        </Push>
      </Sequence>
      <Sequence from={TITLE_HOOK + NUMBERS} durationInFrames={VOTES_OUTRO}>
        <Push length={VOTES_OUTRO} overlap={PUSH} last>
          <BrandOutro site={site} linkPath={reel.link_path} note={null} />
        </Push>
      </Sequence>
    </>
  )
}
