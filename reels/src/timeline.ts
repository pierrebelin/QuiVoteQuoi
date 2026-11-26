import { FPS, POST_MAX_IMAGES } from './theme'
import type { ReelProps } from './types'

/**
 * Découpage d'un Reel en scènes, partagé par `calculateMetadata` (durée totale)
 * et par les compositions (début de chaque scène). Durées en images.
 */
export const NUMBERS = 3 * FPS

/**
 * Reel de votes : le scrutin le plus suivi en trois scènes (accroche, hémicycle
 * et décompte, groupes par position), puis la période, les votes sur l'ensemble
 * et la fin. Les scènes se succèdent sur un décor qui ne bouge pas.
 */
export const HOOK = Math.round(2.5 * FPS)
export const COUNT = 4 * FPS
export const GROUPS = 5 * FPS
export const PERIOD = Math.round(2.5 * FPS)
/** Votes sur l'ensemble montrés ensemble sur une carte du Reel de votes. */
export const VOTES_PER_CARD = 2
export const VOTES_OUTRO = Math.round(3.5 * FPS)
/** Reels hors votes : chaque scène déborde de `PUSH` images sur la suivante, qui la pousse hors de l'écran. */
export const PUSH = 14

/** Reels hors votes : accroche, puis nombres du thème, cartes des programmes ou page de la nouveauté, fin commune. */
export const TITLE_HOOK = Math.round(3.5 * FPS)
export const NEWS_PHONE = 7 * FPS

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

export function planCards(count: number, maxCards = Math.floor(CARDS_BUDGET_SECONDS / CARD_MIN_SECONDS), perCardAtLeast = 1): CardPlan {
  if (count === 0) return { groups: [], each: 0 }
  const perCard = Math.max(perCardAtLeast, Math.ceil(count / maxCards))
  const cards = Math.ceil(count / perCard)
  const seconds = Math.min(CARD_MAX_SECONDS, Math.max(CARD_MIN_SECONDS, CARDS_BUDGET_SECONDS / cards))
  const groups = Array.from({ length: cards }, (_, card) =>
    Array.from({ length: Math.min(perCard, count - card * perCard) }, (_, i) => card * perCard + i),
  )
  return { groups, each: Math.round(seconds * FPS) }
}

export type Scenes = {
  /** Durée de chaque scène, dans l'ordre : une image du carrousel par scène. */
  lengths: number[]
  numbers: number
  cards: CardPlan
  mostFollowed: number
  siteNews: number
  total: number
}

/** Scènes d'un Reel selon son sujet : votes, programmes, nouveauté ou thème. */
export function scenesOf({ reel, post }: ReelProps): Scenes {
  if (isVotesReel(reel)) return votesScenesOf(reel, post === true)
  // Même priorité que `compositionOf` : thème, puis nouveauté, puis programmes.
  const numbers = reel.theme ? NUMBERS : 0
  const siteNews = !reel.theme && reel.site_news ? NEWS_PHONE : 0
  const fixed = [TITLE_HOOK, numbers, siteNews, VOTES_OUTRO].filter((length) => length > 0)
  const cards = planCards(reel.theme || reel.site_news ? 0 : reel.programmes.length, maxCardsOf(post === true, fixed.length))
  const lengths = [TITLE_HOOK, numbers, ...cards.groups.map(() => cards.each), siteNews, VOTES_OUTRO].filter((length) => length > 0)
  return { lengths, numbers, cards, mostFollowed: 0, siteNews, total: sum(lengths) }
}

/**
 * Images du carrousel : la dernière image de chaque scène, juste avant que la
 * suivante ne la pousse, quand ses animations sont terminées.
 */
export function stillFrames(props: ReelProps): number[] {
  let end = 0
  return scenesOf(props).lengths.map((length) => {
    end += length
    return end - 1
  })
}

const sum = (values: number[]) => values.reduce((total, value) => total + value, 0)

/** Carrousel : les cartes regroupent les éléments pour tenir en `POST_MAX_IMAGES` images, sans en retirer (README §2). */
function maxCardsOf(post: boolean, fixed: number) {
  return post ? POST_MAX_IMAGES - fixed : undefined
}

/** Même choix que `compositionOf` de `scripts/prepare.mjs` : ni thème, ni nouveauté, ni programmes. */
export function isVotesReel(reel: ReelProps['reel']) {
  return reel.theme === null && reel.site_news === null && reel.programmes.length === 0
}

/**
 * Reel de votes : le scrutin le plus suivi ouvre (accroche, hémicycle et
 * décompte, groupes), puis les nombres de la période, chaque vote sur
 * l'ensemble, la fin (RM-03).
 */
function votesScenesOf(reel: ReelProps['reel'], post: boolean): Scenes {
  const opening = reel.most_followed ? [HOOK, COUNT, GROUPS] : []
  const cards = planCards(reel.final_votes.length, maxCardsOf(post, opening.length + 2), VOTES_PER_CARD)
  const lengths = [...opening, PERIOD, ...cards.groups.map(() => cards.each), VOTES_OUTRO]
  return { lengths, numbers: PERIOD, cards, mostFollowed: sum(opening), siteNews: 0, total: sum(lengths) }
}
