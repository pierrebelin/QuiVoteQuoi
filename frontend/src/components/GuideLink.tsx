import { Link } from 'react-router'

/**
 * Renvoi vers le guide de lecture. La prose vit dans `/comprendre` et nulle
 * part ailleurs : une explication recopiée sur une page de vote finirait par
 * contredire la page qui fait référence.
 *
 * Une ancre vers une question de détail déplie le bloc « Pour aller plus
 * loin » qui la contient : le lien reste valable quel que soit son niveau.
 */
export default function GuideLink({
  ancre,
  children,
}: {
  ancre: string
  children: React.ReactNode
}) {
  return (
    <Link
      to={`/comprendre#${ancre}`}
      className="text-xs text-ink-soft underline decoration-line-strong underline-offset-2 hover:text-accent hover:decoration-accent"
    >
      {children}
    </Link>
  )
}
