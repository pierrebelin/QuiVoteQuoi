import { readingLabel } from './dossierStatus'
import { formatDate } from '../types/scrutins'
import type { DecisionSense, LawChamber } from '../types/dossiers'
import type { AdvancedText, WeekEvent } from '../types/week'

export function addDays(iso: string, days: number): string {
  const date = new Date(iso + 'T00:00:00Z')
  date.setUTCDate(date.getUTCDate() + days)
  return date.toISOString().slice(0, 10)
}

/** « Du 9 au 15 novembre 2026 », le mois et l'année du lundi omis quand ce sont les mêmes. */
export function weekSpan(monday: string): string {
  const sunday = addDays(monday, 6)
  const sameMonth = monday.slice(0, 7) === sunday.slice(0, 7)
  const sameYear = monday.slice(0, 4) === sunday.slice(0, 4)
  const first = new Date(monday + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    ...(sameMonth ? {} : { month: 'long' }),
    ...(sameYear ? {} : { year: 'numeric' }),
  })
  return `Du ${first} au ${formatDate(sunday)}`
}

export const votesWord = (count: number) => `${count.toLocaleString('fr-FR')} vote${count > 1 ? 's' : ''}`

/** « Jeu. 19 » : jour d'une semaine déjà située par son titre. */
export function weekDayLabel(iso: string): string {
  const label = new Date(iso + 'T00:00:00')
    .toLocaleDateString('fr-FR', { weekday: 'short', day: 'numeric' })
    .replace(/^(\p{L}+)\.? /u, '$1. ')
  return label.charAt(0).toUpperCase() + label.slice(1)
}

const ADVANCE_SENSES: Record<DecisionSense, string> = {
  adopted: 'adopté',
  adopted_unchanged: 'adopté sans modification',
  modified: 'modifié',
  rejected: 'rejeté',
  cmp_agreement: 'accord',
  cmp_disagreement: 'désaccord',
}

const CHAMBER_SUBJECTS: Record<LawChamber, string> = {
  an: 'Assemblée',
  sn: 'Sénat',
  cmp: 'Commission mixte paritaire',
}

/** Où l'étape a eu lieu : « Assemblée, 2e lecture », « Sénat, texte de la CMP ». */
function advancePlace(event: WeekEvent): string {
  if (event.reading === 'CMP' && event.chamber && event.chamber !== 'cmp') {
    return `${CHAMBER_SUBJECTS[event.chamber]}, texte de la CMP`
  }
  const reading = readingLabel(event.reading)
  if (reading) return reading.replace(' · ', ', ')
  return event.chamber ? CHAMBER_SUBJECTS[event.chamber] : event.reading
}

/** « 18 nov. » */
const dayMonth = (iso: string) =>
  new Date(iso + 'T00:00:00').toLocaleDateString('fr-FR', { day: 'numeric', month: 'short' })

/**
 * Étape de la semaine d'un texte qui a avancé (RM-08), en phrase à trous
 * remplie avec la source seule (README §8) : « Jeu. 19 · Assemblée, 1re
 * lecture : adopté sans modification, sans scrutin public. ». « Sans scrutin
 * public » ne vaut que pour l'Assemblée : les scrutins du Sénat ne sont pas
 * dans le jeu de données.
 */
export function advanceSentence(text: AdvancedText): { lead: string; sense: string; tail: string } {
  const { event } = text
  const day = weekDayLabel(event.date)
  if (event.reading === 'PROM') {
    const code = text.law_state.law_code
    return { lead: `${day} · Promulgation : `, sense: code ? `loi n° ${code}` : 'promulguée', tail: '.' }
  }
  const sense = event.sense ? ADVANCE_SENSES[event.sense] : 'décision publiée'
  const agreement =
    event.joint_committee_agreement_date && event.sense !== 'cmp_agreement' && event.sense !== 'cmp_disagreement'
      ? ` (accord en CMP le ${dayMonth(event.joint_committee_agreement_date)})`
      : ''
  const silent = event.chamber === 'an' && !event.has_public_vote ? ', sans scrutin public' : ''
  return { lead: `${day} · ${advancePlace(event)} : `, sense, tail: `${agreement}${silent}.` }
}
