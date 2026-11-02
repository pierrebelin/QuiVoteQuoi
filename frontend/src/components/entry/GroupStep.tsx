import { Link } from 'react-router'
import { PoliticalLogo } from '../PoliticalLogo'
import { GROUP_LOGOS } from '../../content/logos'
import type { GroupDto } from '../../types/finalVotes'
import { groupShortName, groupsSummary } from '../../lib/entryFlow'
import { ActionBar, CheckDot, PrimaryButton, StepIntro } from './parts'
import { FOCUS_RING, choiceTileClass } from './styles'

/**
 * Écran 2 : choisir les groupes à suivre. Aucun groupe n'est présélectionné :
 * « Tous les groupes » est l'état de départ, il ne désigne personne
 * (README.md §6).
 */
export default function GroupStep({
  groups,
  followed,
  themesLabel,
  backTo,
  onToggle,
  onAll,
  onNext,
}: {
  /** Groupes dans l'ordre de l'hémicycle. */
  groups: GroupDto[]
  /** Sigles suivis ; vide = tous. */
  followed: string[]
  themesLabel: string
  backTo: string
  onToggle: (abbrev: string) => void
  onAll: () => void
  onNext: () => void
}) {
  const all = followed.length === 0
  const names = groups
    .filter((group) => followed.includes(group.abbrev))
    .map((group) => groupShortName(group))

  return (
    <div className="space-y-6">
      <Link
        to={backTo}
        className={`inline-flex min-h-11 items-center rounded-md text-sm font-medium text-accent hover:underline ${FOCUS_RING}`}
      >
        ← Sujets&nbsp;: {themesLabel}
      </Link>

      <StepIntro eyebrow="Étape 2 sur 2" title="Quels groupes voulez-vous suivre&nbsp;?">
        Un groupe réunit des députés à l’Assemblée. Ce n’est pas toujours un parti.
      </StepIntro>

      <button
        type="button"
        aria-pressed={all}
        onClick={onAll}
        className={`${choiceTileClass(all)} min-h-[60px] px-4 py-3 text-base font-semibold`}
      >
        <span className="min-w-0 flex-1">
          Tous les groupes <span className="font-normal text-ink-soft">· les {groups.length}</span>
        </span>
        <CheckDot on={all} />
      </button>

      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <p className="text-sm text-ink-soft">Ou choisissez-en quelques-uns&nbsp;:</p>
        <Link to="/groupes" className="text-sm text-accent underline">
          Composition des groupes
        </Link>
      </div>

      <ul className="grid grid-cols-1 gap-2.5 min-[480px]:grid-cols-2 lg:grid-cols-4">
        {groups.map((group) => {
          const on = followed.includes(group.abbrev)
          return (
            <li key={group.uid}>
              <button
                type="button"
                aria-pressed={on}
                onClick={() => onToggle(group.abbrev)}
                title={group.label}
                className={`${choiceTileClass(on)} min-h-16 px-3 py-2.5`}
              >
                <PoliticalLogo
                  logo={GROUP_LOGOS[group.abbrev]}
                  fallback={group.abbrev}
                  color={null}
                  className="h-9 w-14 rounded-lg"
                />
                <span className="min-w-0 flex-1 text-sm leading-snug font-semibold">
                  {groupShortName(group)}
                </span>
                <CheckDot on={on} />
              </button>
            </li>
          )
        })}
      </ul>

      <ActionBar summary={groupsSummary(names, groups.length)}>
        <PrimaryButton onClick={onNext}>Voir les votes →</PrimaryButton>
      </ActionBar>
    </div>
  )
}
