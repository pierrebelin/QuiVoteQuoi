import { AbsoluteFill, Img, Sequence, staticFile } from 'remotion'
import { Frame, Intro, Note, Outro, Soundtrack, Title, useFade } from '../components'
import { INTRO, SITE_NEWS, scenesOf } from '../timeline'
import type { ReelProps } from '../types'

const TEXT = Math.round(SITE_NEWS * 0.4)
const CAPTURE = SITE_NEWS - TEXT

function Text({ title, text }: { title: string; text: string }) {
  const style = useFade(TEXT)
  return (
    <Frame>
      <div style={{ ...style, display: 'flex', flexDirection: 'column', gap: 32 }}>
        <Title size={72}>{title}</Title>
        <Note>{text}</Note>
      </div>
    </Frame>
  )
}

function Capture({ file }: { file: string }) {
  const style = useFade(CAPTURE)
  return (
    <AbsoluteFill style={{ opacity: style.opacity }}>
      <Img src={staticFile(file)} style={{ width: 1080, height: 1920, objectFit: 'cover', objectPosition: 'top' }} />
    </AbsoluteFill>
  )
}

/**
 * Reel « Nouveau sur le site » (RM-08) : le texte de la nouveauté, sans aucun
 * chiffre, puis la capture de sa page en production.
 */
export function SiteNews(props: ReelProps) {
  const { reel, site, capture } = props
  const scenes = scenesOf(props)
  const news = reel.site_news
  if (!news) throw new Error('Reel sans nouveauté')

  return (
    <>
      <Soundtrack audio={props.audio} />
      <Sequence durationInFrames={INTRO}>
        <Intro kicker="Nouveau sur le site" title={news.title} />
      </Sequence>
      <Sequence from={INTRO} durationInFrames={TEXT}>
        <Text title={news.title} text={news.text} />
      </Sequence>
      {capture && (
        <Sequence from={INTRO + TEXT} durationInFrames={CAPTURE}>
          <Capture file={capture} />
        </Sequence>
      )}
      <Sequence from={INTRO + scenes.siteNews}>
        <Outro site={site} path={reel.link_path} notes={[]} />
      </Sequence>
    </>
  )
}
