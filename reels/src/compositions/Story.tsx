import type { CSSProperties, ReactNode } from 'react'
import { capitalize, Kicker, Night, useFonts } from '../motion'
import { FONT_DISPLAY, NIGHT } from '../theme'
import { isVotesReel } from '../timeline'
import type { ReelProps, ReelResponse } from '../types'

/** Zones couvertes par l'interface d'une story (barre de progression et compte en haut, réponse en bas). */
const STORY_SAFE = { top: 250, bottom: 280, side: 90 }

const DISPLAY: CSSProperties = { fontFamily: FONT_DISPLAY, fontWeight: 800, color: NIGHT.ink }

const plural = (count: number, one: string, many: string) => (count > 1 ? many : one)

type Teaser = { context: string; title: string; question: ReactNode; call: string }

/**
 * Accroche du post du jour, selon son sujet : le titre seul, sans réponse,
 * pour que la story donne envie d'ouvrir le carrousel sur le profil.
 */
function teaserOf(reel: ReelResponse, details: ReelProps['details']): Teaser {
  if (reel.site_news) {
    return { context: 'Nouveau sur le site', title: reel.site_news.title, question: null, call: 'Le détail sur le profil' }
  }
  if (reel.theme) {
    return { context: 'Un thème, tous ses votes', title: reel.theme.label, question: null, call: 'Le détail sur le profil' }
  }
  if (!isVotesReel(reel)) {
    const count = reel.programmes.length
    return {
      context: 'Programmes 2027',
      title: `${count} ${plural(count, 'candidature a modifié son programme', 'candidatures ont modifié leur programme')}`,
      question: 'Qui a changé quoi ?',
      call: 'La réponse sur le profil',
    }
  }
  const mf = reel.most_followed
  const question = (
    <>
      Qui a voté <span style={{ color: NIGHT.for }}>pour</span>&nbsp;?
      <br />
      Qui a voté <span style={{ color: NIGHT.against }}>contre</span>&nbsp;?
    </>
  )
  if (mf) {
    const title = details[mf.uid]?.dossier_label ?? capitalize(mf.subject)
    return { context: capitalize(reel.period.label), title, question, call: 'La réponse sur le profil' }
  }
  return { context: "À l'Assemblée nationale", title: capitalize(reel.period.label), question, call: 'La réponse sur le profil' }
}

/**
 * Story d'annonce du post du jour, image fixe en 9:16. L'API d'Instagram ne
 * permet ni lien ni partage d'un post en story : elle renvoie donc au profil,
 * où le carrousel est la dernière publication.
 */
export function Story({ reel, details, site }: ReelProps) {
  useFonts()
  const teaser = teaserOf(reel, details)
  return (
    <Night>
      <div
        style={{
          position: 'absolute',
          top: STORY_SAFE.top,
          bottom: STORY_SAFE.bottom,
          left: STORY_SAFE.side,
          right: STORY_SAFE.side,
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
        }}
      >
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 20 }}>
          <Kicker>Nouveau post</Kicker>
          <span style={{ fontSize: 36, fontWeight: 600, color: NIGHT.inkSoft }}>{teaser.context}</span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 48 }}>
          <div style={{ ...DISPLAY, fontSize: teaser.title.length > 60 ? 88 : 112, lineHeight: 0.98, letterSpacing: -2.5 }}>{teaser.title}</div>
          {teaser.question && <div style={{ ...DISPLAY, fontSize: 64, fontWeight: 700, lineHeight: 1.12, letterSpacing: -1 }}>{teaser.question}</div>}
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
          <span style={{ ...DISPLAY, fontSize: 64 }}>{teaser.call}</span>
          <span style={{ fontSize: 34, fontWeight: 600, color: NIGHT.inkSoft }}>Touchez le nom du compte, en haut · {site}</span>
        </div>
      </div>
    </Night>
  )
}
