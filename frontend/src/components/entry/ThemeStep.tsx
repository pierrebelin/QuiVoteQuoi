import { Link } from 'react-router'
import { FamilyIllustration } from '../FamilyIllustration'
import { Icon } from '../icons'
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
    <div className="space-y-6 md:space-y-7">
      <div>
        <Link
          to="/"
          className={`inline-flex min-h-11 items-center rounded text-sm font-semibold text-ink-soft transition-colors hover:text-ink ${FOCUS_RING}`}
        >
          ← Accueil
        </Link>
        <StepIntro
          step={1}
          title="Quels sujets vous intéressent&nbsp;?"
          lead="Choisissez un ou plusieurs sujets, puis les groupes à suivre."
        />
      </div>

      {/* Deux tuiles par ligne dès le téléphone : seize sujets sur une colonne
          faisaient défiler quatre écrans avant le bouton « Continuer ». */}
      <ul className="grid grid-cols-2 gap-2 sm:gap-3 lg:grid-cols-4">
        {families.map((family) => {
          const on = selected.includes(family.code)
          return (
            <li key={family.code}>
              <button
                type="button"
                aria-pressed={on}
                onClick={() => onToggle(family.code)}
                className={`${choiceTileClass(on, 'flex-col items-start justify-between gap-2 sm:flex-row sm:items-center sm:justify-start sm:gap-3')} relative h-full min-h-[104px] p-3 sm:min-h-20 sm:px-3.5 sm:py-3.5`}
              >
                {/* L'icône de famille en couleur, sur une pastille blanche. */}
                <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-canvas">
                  <FamilyIllustration
                    bare
                    code={family.code}
                    className="shrink-0"
                    glyphClassName="size-[22px]"
                  />
                </span>
                <span className="min-w-0 text-[14.5px] leading-[1.3] font-[620] tracking-[-0.005em] text-balance sm:flex-1 sm:text-[15.5px]">
                  {typographic(family.label)}
                </span>
                <span className="absolute top-3 right-3 sm:static">
                  <CheckDot on={on} />
                </span>
              </button>
            </li>
          )
        })}
      </ul>

      {/* Deux lignes à filet sur téléphone, flèche à droite ; deux liens soulignés au-delà. */}
      <ul className="text-[15px] md:-mt-2 md:flex md:flex-wrap md:gap-x-8 md:gap-y-1 md:text-sm">
        {[
          ['/themes/methode', 'Comment les textes sont rattachés aux sujets'],
          ['/themes/non-rattaches', 'Textes sans sujet'],
        ].map(([to, label]) => (
          <li key={to}>
            <Link
              to={to}
              className={`flex min-h-[52px] items-center justify-between gap-3 border-t border-line font-semibold first:border-t-0 md:min-h-11 md:border-0 md:font-medium md:link ${FOCUS_RING}`}
            >
              {label}
              <Icon name="chevron" className="size-[18px] text-accent md:hidden" strokeWidth={2} />
            </Link>
          </li>
        ))}
      </ul>

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
