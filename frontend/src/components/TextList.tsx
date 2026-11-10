import { Link } from 'react-router'
import type { TextSummaryDto } from '../types/themes'
import { FamilyBadges, UnassignedNote } from './ThemeBadges'

function period(text: TextSummaryDto) {
  if (!text.first_vote || !text.last_vote) return null
  const from = new Date(text.first_vote).toLocaleDateString('fr-FR')
  const to = new Date(text.last_vote).toLocaleDateString('fr-FR')
  return from === to ? from : `${from} → ${to}`
}

export default function TextList({
  texts,
  showFamilies = true,
}: {
  texts: TextSummaryDto[]
  showFamilies?: boolean
}) {
  if (texts.length === 0) {
    return <p className="text-sm text-ink-faint">Aucun texte dans cette liste.</p>
  }

  return (
    <ul className="divide-y divide-line overflow-hidden rounded-xl border border-line bg-surface shadow-card">
      {texts.map((text) => (
        <li
          key={text.key}
          className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1.5 px-4 py-3"
        >
          {/* La largeur minimale renvoie les pastilles à la ligne : sans elle,
              le titre, de base nulle, se laisse écraser à quelques pixels. */}
          <div className="min-w-[min(100%,16rem)] flex-1">
            <Link
              to={`/textes/${encodeURIComponent(text.key)}`}
              className="text-[15px] font-semibold leading-snug transition-colors hover:text-accent"
            >
              {text.label}
            </Link>
            <p className="mt-0.5 text-xs text-ink-faint">
              {text.scrutin_count} scrutin{text.scrutin_count > 1 ? 's' : ''}
              {period(text) ? ` · ${period(text)}` : ''}
              {text.dossier_uid ? ' · dossier rattaché' : ''}
            </p>
          </div>
          <div className="max-w-full shrink-0">
            {showFamilies && text.families.length > 0 ? (
              <FamilyBadges families={text.families} />
            ) : (
              <UnassignedNote outcome={text.last_attempt_outcome} />
            )}
          </div>
        </li>
      ))}
    </ul>
  )
}
