import { Img, Sequence, spring, staticFile, useCurrentFrame, useVideoConfig, interpolate, Easing } from 'remotion'
import { Soundtrack } from '../components'
import { BrandOutro, CREAM, NAVY, PAPER, Push, Safe, TitleHook, useEnter, useFonts } from '../motion'
import { COLORS, HEIGHT } from '../theme'
import { TITLE_HOOK, NEWS_PHONE, PUSH, VOTES_OUTRO } from '../timeline'
import type { ReelProps } from '../types'

/** Largeur et hauteur de l'écran du téléphone, en pixels de la vidéo. */
const SCREEN = { width: 640, height: 1180 }
/** Hauteur de l'écran dans le post en 4:5. */
const POST_SCREEN_HEIGHT = 940

/** La page de la nouveauté, capturée en production, défile dans un téléphone (RM-08). */
function Phone({ file, site, path }: { file: string; site: string; path: string }) {
  const frame = useCurrentFrame()
  const { fps, height } = useVideoConfig()
  // Post en 4:5 : écran moins haut, et le haut de la page, image fixe sans défilement.
  const post = height < HEIGHT
  const screenHeight = post ? POST_SCREEN_HEIGHT : SCREEN.height
  const rise = spring({ frame, fps, config: { damping: 18, stiffness: 90 } })
  const scroll = post ? 0 : interpolate(frame, [30, NEWS_PHONE - 20], [0, 1], {
    extrapolateLeft: 'clamp',
    extrapolateRight: 'clamp',
    easing: Easing.inOut(Easing.cubic),
  })
  const label = useEnter(14, 30)
  return (
    <Safe background={PAPER} color={NAVY}>
      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', gap: 44 }}>
        <div
          style={{
            transform: `translateY(${(1 - rise) * 500}px) rotate(${(1 - rise) * 6}deg)`,
            padding: 18,
            borderRadius: 86,
            background: NAVY,
            boxShadow: '0 40px 90px rgba(22, 35, 63, 0.35)',
          }}
        >
          <div style={{ width: SCREEN.width, height: screenHeight, borderRadius: 68, overflow: 'hidden', background: '#ffffff' }}>
            <Img
              src={staticFile(file)}
              style={{ width: '100%', display: 'block', transform: `translateY(calc(${-scroll} * (100% - ${screenHeight}px)))` }}
            />
          </div>
        </div>
        <div style={{ ...label, fontSize: 40, fontWeight: 800, color: COLORS.accent }}>
          {site}<span style={{ color: NAVY }}>{path}</span>
        </div>
      </div>
    </Safe>
  )
}

/**
 * Reel « Nouveau sur le site » (RM-08) : le titre et le texte de la nouveauté,
 * sans aucun chiffre, puis sa page en production qui défile, puis la fin commune.
 */
export function SiteNews(props: ReelProps) {
  useFonts()
  const { reel, site, capture } = props
  const news = reel.site_news
  if (!news) throw new Error('Reel sans nouveauté')

  return (
    <>
      <Soundtrack audio={props.audio} />
      <Sequence durationInFrames={TITLE_HOOK + PUSH}>
        <Push length={TITLE_HOOK} overlap={PUSH} first>
          <TitleHook kicker="Nouveau sur le site" title={news.title} text={news.text} background={COLORS.accent} />
        </Push>
      </Sequence>
      <Sequence from={TITLE_HOOK} durationInFrames={NEWS_PHONE + PUSH}>
        <Push length={NEWS_PHONE} overlap={PUSH}>
          {capture ? <Phone file={capture} site={site} path={news.path} /> : <Safe background={NAVY} color={CREAM}>{null}</Safe>}
        </Push>
      </Sequence>
      <Sequence from={TITLE_HOOK + NEWS_PHONE} durationInFrames={VOTES_OUTRO}>
        <Push length={VOTES_OUTRO} overlap={PUSH} last>
          <BrandOutro site={site} linkPath={reel.link_path} note={null} />
        </Push>
      </Sequence>
    </>
  )
}
