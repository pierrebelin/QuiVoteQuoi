import { Link } from 'react-router'
import { FamilyIllustration } from '../FamilyIllustration'
import type { FamilyDto } from '../../types/themes'
import { typographic } from '../../lib/entryFlow'
import { ActionBar, CheckDot, PrimaryButton, StepIntro } from './parts'
import { choiceTileClass } from './styles'

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
      <StepIntro
        eyebrow="Étape 1 sur 2"
        title="Quels sujets vous intéressent&nbsp;?"
        hint="Choisissez-en un ou plusieurs. Vous verrez ensuite ce qu’ont voté les députés sur ces sujets."
      />

      <ul className="grid grid-cols-1 gap-3 min-[480px]:grid-cols-2 lg:grid-cols-4">
        {families.map((family) => {
          const on = selected.includes(family.code)
          return (
            <li key={family.code}>
              <button
                type="button"
                aria-pressed={on}
                onClick={() => onToggle(family.code)}
                className={`${choiceTileClass(on)} min-h-[72px] px-3.5 py-3`}
              >
                <FamilyIllustration
                  code={family.code}
                  className="size-10 shrink-0 rounded-[10px]"
                  glyphClassName="size-6"
                />
                <span className="min-w-0 flex-1 text-[15px] leading-snug font-semibold">
                  {typographic(family.label)}
                </span>
                <CheckDot on={on} />
              </button>
            </li>
          )
        })}
      </ul>

      <p className="text-sm text-ink-faint">
        <Link to="/themes/methode" className="text-accent underline">
          Comment les textes sont rattachés aux sujets
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
