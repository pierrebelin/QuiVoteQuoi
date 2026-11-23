import { formatShortDate } from './dossierStatus'
import type { DecisionSense, LawChamber, LawState, LawStateKind } from '../types/dossiers'

/**
 * Où en est la loi (todo/navette/SPEC-navette.md) : même libellé et même
 * couleur sur toutes les pages. Le vert est réservé aux textes qui vont
 * s'appliquer (RM-07).
 */
export const LAW_STATE_LABELS: Record<LawStateKind, string> = {
  in_shuttle: 'En navette',
  definitively_adopted: 'Adopté définitivement',
  became_law: 'Devenu loi',
}

/** Fond et texte de chaque état (RM-07). */
export const LAW_STATE_TONES: Record<LawStateKind, string> = {
  in_shuttle: 'bg-surface-soft text-ink',
  definitively_adopted: 'bg-yes-soft text-yes',
  became_law: 'bg-yes text-surface',
}

/**
 * Pastille de chaque état (refonte A) : un disque plein, le mot qui la suit
 * porte la couleur. Le vert reste aux textes qui vont s'appliquer (RM-07) ;
 * « En navette » est neutre.
 */
export const LAW_STATE_DOTS: Record<LawStateKind, string> = {
  in_shuttle: 'bg-ink text-canvas',
  definitively_adopted: 'bg-yes-bar text-white',
  became_law: 'bg-yes-bar text-white',
}

/** Couleur du mot qui suit la pastille. */
export const LAW_STATE_WORDS: Record<LawStateKind, string> = {
  in_shuttle: 'text-ink',
  definitively_adopted: 'text-yes',
  became_law: 'text-yes',
}

/** Libellés des filtres de la liste par texte, au pluriel. */
export const LAW_STATE_FILTER_LABELS: Record<LawStateKind, string> = {
  in_shuttle: 'En navette',
  definitively_adopted: 'Adoptés définitivement',
  became_law: 'Devenus loi',
}

const CHAMBER_NAMES: Record<LawChamber, string> = {
  an: "l'Assemblée",
  sn: 'le Sénat',
  cmp: 'la CMP',
}

const DECISION_WORDS: Record<DecisionSense, string> = {
  adopted: 'adopté',
  adopted_unchanged: 'adopté sans modification',
  modified: 'modifié',
  rejected: 'rejeté',
  cmp_agreement: 'accord en CMP',
  cmp_disagreement: 'désaccord en CMP',
}

/**
 * Complément de l'étiquette d'une carte (RM-01 à RM-03) : la dernière
 * décision pour un texte en navette (« modifié par le Sénat le 20 nov. »),
 * « promulgation attendue », ou le numéro et la date de la loi.
 */
export function lawStateDetail(state: LawState, today: string): string | null {
  if (state.kind === 'definitively_adopted') return 'promulgation attendue'
  if (state.kind === 'became_law') {
    const date = formatShortDate(state.date, today)
    return state.law_code ? `loi n° ${state.law_code} du ${date}` : `promulguée le ${date}`
  }
  const decision = state.last_decision
  if (!decision) return null
  const date = formatShortDate(decision.date, today)
  const sense = decision.sense
  if (sense === 'cmp_agreement' || sense === 'cmp_disagreement') return `${DECISION_WORDS[sense]} le ${date}`
  const by = decision.chamber && decision.chamber !== 'cmp' ? ` par ${CHAMBER_NAMES[decision.chamber]}` : ''
  return `${DECISION_WORDS[sense]}${by} le ${date}`
}
