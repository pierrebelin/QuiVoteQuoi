import { Link } from 'react-router'
import { Icon } from '../icons'
import { PoliticalLogo } from '../PoliticalLogo'
import { GROUP_LOGOS } from '../../content/logos'
import type { GroupDto } from '../../types/finalVotes'
import { groupShortName, groupsSummary } from '../../lib/entryFlow'
import { ActionBar, CheckDot, PrimaryButton, StepIntro } from './parts'
import { FOCUS_RING, choiceTileClass, groupRowClass } from './styles'

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
      <div>
        <Link
          to={backTo}
          className={`inline-flex min-h-11 items-center rounded text-sm font-semibold text-ink-soft transition-colors hover:text-ink ${FOCUS_RING}`}
        >
          <span>
            ← Sujets&nbsp;: <span className="ml-0.5 font-[650] text-ink">{themesLabel}</span>
          </span>
        </Link>

        <StepIntro
          step={2}
          title={
            <>
              Quels groupes <span className="whitespace-nowrap">voulez-vous</span> suivre&nbsp;?
            </>
          }
          hint="Un groupe réunit des députés à l’Assemblée. Ce n’est pas toujours un parti."
        />
      </div>

      <button
        type="button"
        aria-pressed={all}
        onClick={onAll}
        className={`${choiceTileClass(all)} min-h-16 py-2.5 pr-4 pl-3 text-base font-[620] md:min-h-[68px] md:py-3 md:pl-3.5 md:text-[17px]`}
      >
        <span className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-canvas">
          <Icon name="candidats" className="size-[22px]" />
        </span>
        <span className="min-w-0 flex-1">
          Tous les groupes <span className="font-normal text-ink-soft">· les {groups.length}</span>
        </span>
        <CheckDot on={all} />
      </button>

      <div className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <p className="text-[13px] font-semibold text-ink-faint md:text-sm">Ou choisissez-en quelques-uns&nbsp;:</p>
        <Link to="/groupes" className="text-sm link">
          Composition des groupes
        </Link>
      </div>

      <ul className="-mt-2 grid grid-cols-1 min-[480px]:mt-0 min-[480px]:grid-cols-2 min-[480px]:gap-2.5 md:gap-3 lg:grid-cols-4">
        {groups.map((group) => {
          const on = followed.includes(group.abbrev)
          return (
            <li key={group.uid}>
              <button
                type="button"
                aria-pressed={on}
                onClick={() => onToggle(group.abbrev)}
                title={group.label}
                className={groupRowClass(on)}
              >
                <PoliticalLogo
                  logo={GROUP_LOGOS[group.abbrev]}
                  fallback={group.abbrev}
                  color={null}
                  className="h-[30px] w-11 min-[480px]:h-9 min-[480px]:w-14"
                />
                <span className="min-w-0 flex-1 text-[15px] leading-snug font-semibold text-pretty">
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
