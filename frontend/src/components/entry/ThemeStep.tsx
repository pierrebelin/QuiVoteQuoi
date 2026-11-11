import { Link } from 'react-router'
import { FamilyIllustration } from '../FamilyIllustration'
import type { FamilyDto } from '../../types/themes'
import { typographic } from '../../lib/entryFlow'
import { ActionBar, CheckDot, PrimaryButton, StepIntro } from './parts'
import { FOCUS_RING, choiceTileClass } from './styles'

/** Écran 1 : choisir un ou plusieurs sujets. */
export default function ThemeStep({
  families,
  selected,
  onToggle,
  onNext,
}: {
  families: FamilyDto[]
  selected: string[]
  onToggle: (code: string) => void
  onNext: () => void
}) {
  const count = selected.length

  return (
    <div className="space-y-7">
      <div>
        <Link
          to="/"
          className={`inline-flex min-h-11 items-center rounded text-sm font-medium link ${FOCUS_RING}`}
        >
          ← Accueil
        </Link>
        <StepIntro
          eyebrow="Étape 1 sur 2"
          title="Quels sujets vous intéressent&nbsp;?"
          lead="Choisissez un ou plusieurs sujets, puis les groupes à suivre."
        />
      </div>

      {/* Deux tuiles par ligne dès le téléphone : seize sujets sur une colonne
          faisaient défiler quatre écrans avant le bouton « Continuer ». */}
      <ul className="grid grid-cols-2 gap-2.5 sm:gap-3 lg:grid-cols-4">
        {families.map((family) => {
          const on = selected.includes(family.code)
          return (
            <li key={family.code}>
              <button
                type="button"
                aria-pressed={on}
                onClick={() => onToggle(family.code)}
                className={`${choiceTileClass(on, 'flex-col items-start justify-between gap-2 sm:flex-row sm:items-center sm:justify-start sm:gap-3')} relative h-full min-h-24 p-3 sm:min-h-[72px] sm:px-3.5 sm:py-3`}
              >
                <FamilyIllustration
                  code={family.code}
                  className="size-9 shrink-0 rounded sm:size-10"
                  glyphClassName="size-5 sm:size-6"
                />
                <span className="min-w-0 text-[13.5px] leading-snug font-semibold sm:flex-1 sm:text-[15px]">
                  {typographic(family.label)}
                </span>
                <span className="absolute top-2 right-2 sm:static">
                  <CheckDot on={on} />
                </span>
              </button>
            </li>
          )
        })}
      </ul>

      <p className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-ink-faint">
        <Link to="/themes/methode" className="link">
          Comment les textes sont rattachés aux sujets
        </Link>
        <Link to="/themes/non-rattaches" className="link">
          Textes sans sujet
        </Link>
      </p>

      <ActionBar
        summary={
          count === 0
            ? 'Aucun sujet choisi'
            : `${count} sujet${count > 1 ? 's' : ''} choisi${count > 1 ? 's' : ''}`
        }
      >
        <PrimaryButton onClick={onNext} disabled={count === 0}>
          Continuer →
        </PrimaryButton>
      </ActionBar>
    </div>
  )
}
