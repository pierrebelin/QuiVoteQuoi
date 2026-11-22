import { FPS } from './theme'
import type { ReelProps } from './types'

/**
 * Découpage d'un Reel en scènes, partagé par `calculateMetadata` (durée totale)
 * et par les compositions (début de chaque scène). Durées en images.
 */
export const INTRO = Math.round(2.5 * FPS)
export const NUMBERS = 3 * FPS
export const MOST_FOLLOWED_TITLE = 2 * FPS
export const MOST_FOLLOWED_IMAGE = 4 * FPS
export const SITE_NEWS = 7 * FPS
export const OUTRO = 4 * FPS

/** Temps donné à l'ensemble des cartes (T7). */
const CARDS_BUDGET_SECONDS = 90
/** Une carte reste au plus 3 s, au moins 2 s ; en deçà, plusieurs éléments par carte. */
const CARD_MAX_SECONDS = 3
const CARD_MIN_SECONDS = 2

export type CardPlan = {
  /** Éléments par carte, dans l'ordre de l'API : aucun n'est retiré (README §2). */
  groups: number[][]
  /** Durée d'une carte, en images. */
  each: number
}

export function planCards(count: number): CardPlan {
  if (count === 0) return { groups: [], each: 0 }
  const perCard = Math.ceil(count / Math.floor(CARDS_BUDGET_SECONDS / CARD_MIN_SECONDS))
  const cards = Math.ceil(count / perCard)
  const seconds = Math.min(CARD_MAX_SECONDS, Math.max(CARD_MIN_SECONDS, CARDS_BUDGET_SECONDS / cards))
  const groups = Array.from({ length: cards }, (_, card) =>
    Array.from({ length: Math.min(perCard, count - card * perCard) }, (_, i) => card * perCard + i),
  )
  return { groups, each: Math.round(seconds * FPS) }
}

export type Scenes = {
  numbers: number
  cards: CardPlan
  mostFollowed: number
  siteNews: number
  total: number
}

/** Scènes d'un Reel selon son sujet : votes, programmes, nouveauté ou thème. */
export function scenesOf({ reel }: ReelProps): Scenes {
  const votes = reel.totals !== null
  const cards = planCards(votes ? reel.final_votes.length : reel.programmes.length)
  const numbers = votes || reel.theme !== null || reel.programmes.length > 0 ? NUMBERS : 0
  const mostFollowed = reel.most_followed ? MOST_FOLLOWED_TITLE + MOST_FOLLOWED_IMAGE : 0
  const siteNews = reel.site_news ? SITE_NEWS : 0
  const cardsLength = cards.groups.length * cards.each
  return {
    numbers,
    cards,
    mostFollowed,
    siteNews,
    total: INTRO + numbers + cardsLength + mostFollowed + siteNews + OUTRO,
  }
}
