import type { BudgetMissionDto } from '../types/budget'

/*
 * Mise en forme des montants du budget de l'État. Les règles sont celles de la
 * maquette : trois chiffres environ, assez pour comparer deux lignes sans
 * laisser croire à une précision au million près sur 600 milliards.
 *
 * - 10 Md€ et plus : une décimale (« 145,6 Md€ ») ;
 * - de 1 à 10 Md€ : deux décimales (« 6,07 Md€ ») ;
 * - sous 1 Md€ : millions entiers (« 866 M€ », « 0 M€ »).
 */

const BILLION = 1_000_000_000
const MILLION = 1_000_000
const NBSP = ' '

function decimals(value: number, digits: number): string {
  return value.toLocaleString('fr-FR', {
    minimumFractionDigits: digits,
    maximumFractionDigits: digits,
  })
}

/** Montant découpé en valeur et unité, pour le grand chiffre clé. */
export function amountParts(euros: number): { value: string; unit: 'Md€' | 'M€' } {
  const millions = Math.round(euros / MILLION)
  if (millions < 1000) return { value: decimals(millions, 0), unit: 'M€' }
  const billions = euros / BILLION
  return { value: decimals(billions, billions >= 10 ? 1 : 2), unit: 'Md€' }
}

export function formatAmount(euros: number): string {
  const { value, unit } = amountParts(euros)
  return `${value}${NBSP}${unit}`
}

/** Part d'une mission dans le budget général : une décimale, deux sous 1 %. */
export function formatMissionShare(euros: number, total: number): string {
  const share = total > 0 ? (euros / total) * 100 : 0
  return `${decimals(share, share >= 1 ? 1 : 2)}${NBSP}%`
}

/** Part d'un programme dans sa mission : une décimale. */
export function formatProgrammeShare(euros: number, total: number): string {
  const share = total > 0 ? (euros / total) * 100 : 0
  return `${decimals(share, 1)}${NBSP}%`
}

/** Largeur d'une barre, relative à la plus grosse ligne de la liste. */
export function barWidth(euros: number, largest: number): string {
  return `${largest > 0 ? ((euros / largest) * 100).toFixed(2) : '0'}%`
}

/** `2026-02-19` → « 19 février 2026 », « 1er » pour le premier du mois. */
export function formatLawDate(iso: string): string {
  const [year, month, day] = iso.split('-').map(Number)
  const date = new Date(year, month - 1, day)
  const monthYear = date.toLocaleDateString('fr-FR', { month: 'long', year: 'numeric' })
  return `${day === 1 ? '1er' : day} ${monthYear}`
}

export function accountLabel(mission: BudgetMissionDto): string {
  return mission.account_kind === 'budget_general'
    ? 'Mission du budget général'
    : 'Compte de concours financiers'
}

/** Ligne sous le titre du détail : nature du compte, montant, part du total. */
export function missionSubtitle(mission: BudgetMissionDto, total: number, year: number): string {
  const parts = [accountLabel(mission), `${formatAmount(mission.cp_euros)} en ${year}`]
  if (mission.account_kind === 'budget_general') {
    parts.push(`${formatMissionShare(mission.cp_euros, total)} du total`)
  }
  return parts.join(' · ')
}
