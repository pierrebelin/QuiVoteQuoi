import type { IconName } from '../components/icons'
import { ENTRY_PATH } from './entryFlow'

/*
 * Navigation principale, la même à toutes les largeurs : quatre destinations
 * puis « Plus ». Sous 768 px c'est la barre basse (`MobileNav`), au-dessus la
 * barre haute (`App`).
 *
 * Chaque entrée couvre aussi les pages qu'elle ouvre : la fiche d'un scrutin
 * reste sous « Votes », celle d'un groupe sous « Groupes ».
 */
export interface NavEntry {
  to: string
  label: string
  icon: IconName
  /** Préfixes d'adresse sous lesquels l'entrée est active, `to` compris. */
  covers: string[]
}

export const NAV_TABS: NavEntry[] = [
  { to: ENTRY_PATH, label: 'Sujets', icon: 'themes', covers: [ENTRY_PATH, '/themes', '/textes'] },
  { to: '/scrutins', label: 'Votes', icon: 'list', covers: ['/scrutins'] },
  { to: '/votes-par-groupe', label: 'Groupes', icon: 'votes', covers: ['/votes-par-groupe', '/groupes'] },
  { to: '/candidats-2027', label: 'Candidats 2027', icon: 'candidats', covers: ['/candidats-2027'] },
]

export const NAV_MORE: NavEntry[] = [
  { to: '/dossiers', label: 'Dossiers', icon: 'dossiers', covers: ['/dossiers'] },
  { to: '/comprendre', label: 'Comprendre', icon: 'comprendre', covers: ['/comprendre'] },
  { to: '/a-propos', label: 'À propos', icon: 'apropos', covers: ['/a-propos'] },
]

export function isUnder(pathname: string, to: string): boolean {
  return pathname === to || pathname.startsWith(`${to}/`)
}

export function isActive(pathname: string, entry: NavEntry): boolean {
  return entry.covers.some((prefix) => isUnder(pathname, prefix))
}
