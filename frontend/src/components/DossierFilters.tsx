import type { ReactNode } from 'react'
import { FamilyIllustration } from './FamilyIllustration'
import { Icon } from './icons'
import { CheckRow, FOCUS, SheetSection } from './SheetControls'
import type { FamilyDto } from '../types/themes'

export interface FilterOption {
  value: string
  label: string
  /** Précision affichée sous l'option, quand son libellé ne suffit pas. */
  help?: string
}

export interface FilterShortcut {
  label: string
  /** Dossiers que le raccourci affiche ; `undefined` tant qu'il n'est pas connu. */
  count: number | undefined
  active: boolean
  onToggle: () => void
}

/**
 * Bouton qui pose d'un coup plusieurs filtres, et les retire au second appui.
 * Plein ET coché quand il est actif : pas la seule couleur.
 */
export function ShortcutChip({ shortcut }: { shortcut: FilterShortcut }) {
  const { label, count, active, onToggle } = shortcut
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onToggle}
      className={`flex min-h-11 w-full items-center gap-2 rounded-xl px-3 text-left text-sm font-semibold transition-colors ${FOCUS} ${
        active
          ? 'bg-ink text-surface'
          : 'bg-surface text-ink ring-1 ring-line-strong ring-inset hover:bg-surface-soft'
      }`}
    >
      <Icon
        name={active ? 'check' : 'dossiers'}
        className={`size-[18px] ${active ? '' : 'text-ink-soft'}`}
        strokeWidth={active ? 3 : undefined}
      />
      {label}
      {count !== undefined && (
        <span className={`ml-auto text-[13px] font-medium ${active ? '' : 'text-ink-soft'}`}>
          {count.toLocaleString('fr-FR')}
        </span>
      )}
    </button>
  )
}

/**
 * Champs de filtre de la liste des dossiers : raccourci, sort, initiative, thème.
 *
 * Le même contenu sert à deux endroits : dans le tiroir « Filtres » sous
 * 1024 px, et dans la colonne de gauche au-delà. Il n'a donc aucun état propre
 * (les filtres vivent dans l'URL, la page les lui passe). La colonne reste
 * montée mais masquée (`display: none`) sous 1024 px : elle n'est ni visible
 * ni atteignable au clavier tant que le tiroir est le seul à servir.
 */
export default function DossierFilters({
  layout,
  shortcut,
  outcomes,
  selectedOutcomes,
  onToggleOutcome,
  initiatives,
  initiative,
  onInitiative,
  families,
  theme,
  onTheme,
}: {
  /** `sheet` : tiroir étroit, initiative en trois boutons. `aside` : colonne, en rangées. */
  layout: 'sheet' | 'aside'
  shortcut: FilterShortcut
  outcomes: FilterOption[]
  selectedOutcomes: string[]
  onToggleOutcome: (value: string) => void
  initiatives: FilterOption[]
  initiative: string
  onInitiative: (value: string) => void
  /** `undefined` tant que les thèmes se chargent : la section n'est pas rendue. */
  families: FamilyDto[] | undefined
  theme: string
  /** Code du thème à filtrer, ou `''` pour tous. */
  onTheme: (code: string) => void
}) {
  return (
    <>
      <SheetSection title="Raccourci">
        <ShortcutChip shortcut={shortcut} />
      </SheetSection>

      <SheetSection title="Sort">
        <div role="group" aria-label="Sort">
          {outcomes.map((option) => (
            <div key={option.value}>
              <CheckRow
                checked={selectedOutcomes.includes(option.value)}
                onToggle={() => onToggleOutcome(option.value)}
              >
                {option.label}
              </CheckRow>
              {option.help && (
                <p className="mx-0.5 mt-1.5 text-[12.5px] leading-snug text-ink-soft">
                  {option.help}
                </p>
              )}
            </div>
          ))}
        </div>
      </SheetSection>

      <SheetSection title="Initiative">
        {layout === 'sheet' ? (
          <div
            role="group"
            aria-label="Initiative"
            className="grid grid-cols-3 gap-1 rounded-xl bg-surface-soft p-[3px]"
          >
            {initiatives.map((option) => {
              const on = initiative === option.value
              return (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={on}
                  onClick={() => onInitiative(option.value)}
                  className={`flex min-h-11 items-center justify-center gap-1 rounded-[9px] px-1 text-[13px] whitespace-nowrap transition-colors ${FOCUS} ${
                    on
                      ? 'bg-ink font-semibold text-surface'
                      : 'font-medium text-ink-soft hover:text-ink'
                  }`}
                >
                  {on && <Icon name="check" className="size-3.5" strokeWidth={3} />}
                  {option.label}
                </button>
              )
            })}
          </div>
        ) : (
          <div role="group" aria-label="Initiative">
            {initiatives.map((option) => {
              const on = initiative === option.value
              return (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={on}
                  onClick={() => onInitiative(option.value)}
                  className={`flex min-h-12 w-full items-center gap-3 border-b border-line px-0.5 text-left text-[15px] text-ink transition-colors hover:bg-surface-soft ${FOCUS} ${
                    on ? 'font-semibold' : 'font-medium'
                  }`}
                >
                  <span
                    aria-hidden
                    className={`flex size-6 shrink-0 items-center justify-center rounded-full ${
                      on ? 'bg-ink text-surface' : 'ring-[1.5px] ring-line-strong ring-inset'
                    }`}
                  >
                    {on && <Icon name="check" className="size-[15px]" strokeWidth={3.2} />}
                  </span>
                  {option.label}
                </button>
              )
            })}
          </div>
        )}
      </SheetSection>

      {/* Les thèmes en tuiles plutôt qu'en liste déroulante : on reconnaît un
          sujet à son pictogramme avant d'en lire le libellé. Le thème choisi
          est plein et coché : il ne se distingue pas par la seule couleur. */}
      {families && (
        <SheetSection title="Thème">
          <div role="group" aria-label="Thème" className="grid grid-cols-2 gap-1.5">
            <ThemeTile active={!theme} onClick={() => onTheme('')}>
              <span className="min-w-0">Tous les thèmes</span>
            </ThemeTile>
            {families.map((family) => {
              const on = theme === family.code
              return (
                <ThemeTile
                  key={family.code}
                  active={on}
                  title={family.scope}
                  onClick={() => onTheme(on ? '' : family.code)}
                  icon={
                    on ? undefined : (
                      <FamilyIllustration
                        code={family.code}
                        className="size-8 rounded-lg"
                        glyphClassName="size-5"
                      />
                    )
                  }
                >
                  <span className="min-w-0">{family.label}</span>
                </ThemeTile>
              )
            })}
          </div>
        </SheetSection>
      )}
    </>
  )
}

/** Tuile de thème de 48 px : pictogramme (ou coche si choisie) puis libellé. */
function ThemeTile({
  active,
  onClick,
  title,
  icon,
  children,
}: {
  active: boolean
  onClick: () => void
  title?: string
  icon?: ReactNode
  children: ReactNode
}) {
  return (
    <button
      type="button"
      aria-pressed={active}
      onClick={onClick}
      title={title}
      className={`flex min-h-12 min-w-0 items-center gap-2 rounded-xl py-1 pr-2 ${
        active || icon ? 'pl-1.5' : 'pl-3'
      } text-left text-[12.5px] leading-tight transition-colors ${FOCUS} ${
        active
          ? 'bg-ink font-semibold text-surface'
          : 'font-medium text-ink-soft ring-1 ring-line ring-inset hover:text-ink hover:ring-line-strong'
      }`}
    >
      {active ? (
        <span className="flex size-8 shrink-0 items-center justify-center">
          <Icon name="check" className="size-4" strokeWidth={3} />
        </span>
      ) : (
        icon
      )}
      {children}
    </button>
  )
}
