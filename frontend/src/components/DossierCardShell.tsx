import type { ReactNode } from 'react'
import { Link } from 'react-router'
import DossierStatusBanner from './DossierStatusBanner'
import { FamilyIllustration } from './FamilyIllustration'
import { themeLink, typographic } from '../lib/entryFlow'
import type { DossierStatusLike } from '../lib/dossierStatus'
import type { AssignedFamilyDto } from '../types/themes'

function plural(count: number, word: string) {
  return `${count.toLocaleString('fr-FR')} ${word}${count > 1 ? 's' : ''}`
}

/**
 * Répartition des scrutins adoptés et rejetés, en nombres (README.md §6). La
 * portée dit ce qui est compté : tout le dossier ou les scrutins du jour. Un
 * décompte nul n'a pas de pastille.
 */
export function OutcomeSplit({ adopted, rejected, scope }: { adopted: number; rejected: number; scope: string }) {
  return (
    <p className="mt-2.5 flex flex-wrap items-center gap-1.5 text-[12.5px] font-semibold tabular-nums">
      {adopted > 0 && <span className="rounded-full bg-yes-soft px-2.5 py-0.5 text-yes">{plural(adopted, 'adopté')}</span>}
      {rejected > 0 && <span className="rounded-full bg-no-soft px-2.5 py-0.5 text-no">{plural(rejected, 'rejeté')}</span>}
      <span className="font-medium text-ink-faint">sur {scope}</span>
    </p>
  )
}

/**
 * Puces de thème. Sous 768 px, chacune a une zone tactile de 44 px sans
 * grossir : un pseudo-élément déborde de 9 px au-dessus et au-dessous de ses
 * 26 px. Les rangées se touchent exactement quand elles se replient (26 + 18
 * d'écart = 44) ; plus serrées, deux zones se recouvriraient.
 */
export function FamilyChips({
  families,
  onFamily,
}: {
  families: AssignedFamilyDto[]
  onFamily?: (code: string) => void
}) {
  if (families.length === 0) {
    return <span className="text-xs text-ink-faint italic">Aucun sujet attribué pour l'instant.</span>
  }
  const chip =
    'relative z-10 inline-flex items-center gap-1.5 rounded bg-surface-soft py-0.5 pr-2.5 pl-0.5 text-[13px] font-medium text-ink ring-1 ring-transparent transition before:absolute before:inset-x-0 before:-inset-y-[9px] md:before:hidden hover:ring-line-strong'
  return (
    <div className="flex flex-wrap gap-1.5 max-md:gap-y-[18px]">
      {families.map((family) => {
        const content = (
          <>
            <FamilyIllustration code={family.code} className="size-[22px] rounded" glyphClassName="size-3.5" />
            {typographic(family.label)}
          </>
        )
        return onFamily ? (
          <button
            key={family.code}
            type="button"
            onClick={() => onFamily(family.code)}
            title={`Voir les textes du sujet « ${typographic(family.label)} »`}
            className={chip}
          >
            {content}
          </button>
        ) : (
          <Link key={family.code} to={themeLink(family.code)} className={chip}>
            {content}
          </Link>
        )
      })}
    </div>
  )
}

/**
 * Carte d'un dossier, commune à la liste des dossiers et aux deux vues des
 * scrutins : bandeau d'état, titre, thèmes, répartition, puis ce que la vue
 * ajoute (`below` dans l'en-tête, `children` après lui).
 */
export default function DossierCardShell({
  uid,
  title,
  status,
  today,
  families,
  onFamily,
  themesAside,
  split,
  below,
  stretched = false,
  children,
}: {
  uid: string
  title: string
  status: DossierStatusLike | null
  /** Date du jour (AAAA-MM-JJ) : une étape datée après est annoncée à venir. */
  today: string
  families: AssignedFamilyDto[]
  /** Sans lui, une puce mène à la page du thème ; avec, elle filtre la liste. */
  onFamily?: (code: string) => void
  /** À droite des thèmes sur grand écran, dessous sur mobile. */
  themesAside?: ReactNode
  split?: ReactNode
  below?: ReactNode
  /** Titre étiré sur toute la carte : seulement sans autre lien que les thèmes. */
  stretched?: boolean
  children?: ReactNode
}) {
  const themes = <FamilyChips families={families} onFamily={onFamily} />

  return (
    <article
      className={`group relative overflow-hidden rounded border border-line bg-surface shadow-card ${stretched ? 'transition duration-200 hover:scale-[1.01] hover:border-line-strong hover:shadow-card-hover motion-reduce:transform-none motion-reduce:transition-none' : ''}`}
    >
      {status && <DossierStatusBanner dossier={status} today={today} />}

      <header className="px-[18px] pt-3 pb-3.5">
        <h2
          className={`text-[16.5px] leading-snug font-semibold text-ink transition-colors ${stretched ? 'group-hover:text-accent' : 'hover:text-accent'}`}
        >
          <Link
            to={`/dossiers/${uid}`}
            className={stretched ? 'after:absolute after:inset-0 focus:outline-none' : undefined}
          >
            {title}
          </Link>
        </h2>

        {themesAside ? (
          <div className="mt-2.5 flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
            {themes}
            {themesAside}
          </div>
        ) : (
          <div className="mt-2.5">{themes}</div>
        )}
        {split}
        {below}
      </header>

      {children}
    </article>
  )
}
