import { formatDate } from '../types/scrutins'

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
