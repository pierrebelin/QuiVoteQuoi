import { Link } from 'react-router'
import { FamilyIllustration } from './FamilyIllustration'
import { themeLink } from '../lib/entryFlow'
import { Note } from './ui'
import type { AssignedFamilyDto } from '../types/themes'

/**
 * Le rattachement s'affiche pour ce qu'il rattache, sans dire qui l'a ouvert :
 * savoir qu'un texte est « logement selon un modèle » plutôt que « logement »
 * déplace l'attention du lecteur sans rien lui apprendre sur le vote.
 *
 * Le pictogramme de la famille, le même que sur les cartes de dossiers et la
 * page des thèmes, fait reconnaître le thème avant d'en lire le libellé.
 */
export function FamilyBadges({
  families,
  linkToFamily = true,
  onCard = false,
}: {
  families: AssignedFamilyDto[]
  linkToFamily?: boolean
  /** Posées sur une carte grise : l'étiquette passe au blanc pour s'en détacher. */
  onCard?: boolean
}) {
  if (families.length === 0) return null

  return (
    <div className="flex flex-wrap gap-1.5">
      {families.map((family) => {
        const className = `inline-flex h-[26px] items-center gap-1.5 rounded-full pr-2.5 pl-2 text-[13px] font-medium text-ink-soft ${
          onCard ? 'bg-canvas' : 'bg-surface'
        }`
        const content = (
          <>
            <FamilyIllustration code={family.code} bare className="shrink-0" glyphClassName="size-[15px]" />
            {family.label}
          </>
        )
        return linkToFamily ? (
          <Link
            key={family.code}
            to={themeLink(family.code)}
            className={`${className} transition-colors hover:bg-surface-soft hover:text-ink`}
          >
            {content}
          </Link>
        ) : (
          <span key={family.code} className={className}>
            {content}
          </span>
        )
      })}
    </div>
  )
}

/** Lacune affichée, jamais silencieuse (RM-01). */
export function UnassignedNote({ outcome }: { outcome: string | null }) {
  const reason =
    outcome === 'no_family'
      ? "Aucune famille retenue : le modèle a répondu sans rattacher ce texte."
      : outcome === 'failed'
        ? "Aucune famille : le modèle n'a pas répondu. Une nouvelle passe reprendra ce texte."
        : "Aucune famille : ce texte n'a pas encore été soumis."
  return <p className="text-xs text-ink-faint">{reason}</p>
}

export function MethodText({ note }: { note: string }) {
  return (
    <>
      {note}{' '}
      <Link to="/themes/methode" className="link">
        Méthode complète
      </Link>
    </>
  )
}

/** À partir de 768 px ; sur téléphone, la page place `MethodText` dans une infobulle. */
export function MethodNote({ note }: { note: string }) {
  return (
    <Note desktopOnly>
      <MethodText note={note} />
    </Note>
  )
}
