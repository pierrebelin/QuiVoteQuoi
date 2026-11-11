import { useId, useState, type KeyboardEvent, type ReactNode } from 'react'
import { PoliticalLogo } from '../PoliticalLogo'
import { GROUP_LOGOS } from '../../content/logos'
import type { GroupDto } from '../../types/finalVotes'
import type { FamilyDto } from '../../types/themes'
import { groupShortName, groupsSummary, themesSummary, typographic } from '../../lib/entryFlow'
import { ToggleChip } from './parts'
import { FOCUS_RING } from './styles'
import { useHeaderHeight } from './useHeaderHeight'

type Panel = 'themes' | 'groups'

function PanelButton({
  name,
  summary,
  open,
  panelId,
  onClick,
}: {
  name: string
  summary: string
  open: boolean
  panelId: string
  onClick: () => void
}) {
  return (
    <button
      type="button"
      aria-expanded={open}
      aria-controls={panelId}
      onClick={onClick}
      className={`inline-flex min-h-11 w-full min-w-0 items-center gap-2 rounded border px-3 text-left text-sm font-semibold transition-colors md:w-auto ${FOCUS_RING} ${
        open
          ? 'border-accent bg-accent-soft text-ink'
          : 'border-line-strong bg-surface text-ink hover:bg-surface-soft'
      }`}
    >
      <span className="font-medium text-ink-soft">{name}&nbsp;:</span>
      <span className="min-w-0 flex-1 truncate md:flex-none">{summary}</span>
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={2.4}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden
        className={`size-3.5 shrink-0 transition-transform ${open ? 'rotate-180' : ''}`}
      >
        <path d="M6 9l6 6 6-6" />
      </svg>
    </button>
  )
}

function PanelBody({
  id,
  onClose,
  children,
}: {
  id: string
  onClose: () => void
  children: ReactNode
}) {
  return (
    <div id={id} className="flex flex-col gap-3 pt-3 pb-1">
      <div className="flex max-h-[50vh] flex-wrap gap-2 overflow-y-auto">{children}</div>
      <button
        type="button"
        onClick={onClose}
        className={`min-h-11 self-start rounded-full bg-ink px-5 text-sm font-semibold text-surface ${FOCUS_RING}`}
      >
        Valider
      </button>
    </div>
  )
}

/**
 * Barre collée sous l'en-tête de l'écran des votes : deux menus (sujets,
 * groupes) qui s'ouvrent en panneaux de pastilles. Les positions se lisent sur
 * chaque carte, rangée par rangée : plus de légende à décoder.
 * Les choix s'appliquent à mesure ; « Valider » referme le panneau.
 */
export default function VotesToolbar({
  families,
  themes,
  groups,
  followed,
  onToggleTheme,
  onToggleGroup,
  onAllGroups,
}: {
  families: FamilyDto[]
  /** Codes des sujets choisis. */
  themes: string[]
  groups: GroupDto[]
  /** Sigles suivis ; vide = tous. */
  followed: string[]
  onToggleTheme: (code: string) => void
  onToggleGroup: (abbrev: string) => void
  onAllGroups: () => void
}) {
  const [open, setOpen] = useState<Panel | null>(null)
  const headerHeight = useHeaderHeight()
  const themesPanelId = useId()
  const groupsPanelId = useId()

  const themeLabels = families.filter((f) => themes.includes(f.code)).map((f) => f.label)
  const followedNames = groups
    .filter((group) => followed.includes(group.abbrev))
    .map((group) => groupShortName(group))

  const toggle = (panel: Panel) => setOpen((current) => (current === panel ? null : panel))
  const close = () => setOpen(null)
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') close()
  }

  return (
    <>
      <div
        onKeyDown={onKeyDown}
        className="sticky z-10 -mx-6 -mt-8 border-b border-line bg-surface/95 px-6 py-2.5 shadow-card backdrop-blur"
        style={{ top: headerHeight }}
      >
        <div className="grid grid-cols-2 items-center gap-2 md:flex md:flex-wrap">
          <PanelButton
            name="Sujets"
            summary={themesSummary(themeLabels)}
            open={open === 'themes'}
            panelId={themesPanelId}
            onClick={() => toggle('themes')}
          />
          <PanelButton
            name="Groupes"
            summary={groupsSummary(followedNames, groups.length)}
            open={open === 'groups'}
            panelId={groupsPanelId}
            onClick={() => toggle('groups')}
          />
        </div>

        {open === 'themes' && (
          <PanelBody id={themesPanelId} onClose={close}>
            {families.map((family) => {
              const on = themes.includes(family.code)
              // Au moins un sujet reste choisi : sans sujet, l'écran des votes n'a plus d'objet.
              const last = on && themes.length === 1
              return (
                <ToggleChip
                  key={family.code}
                  pressed={on}
                  disabled={last}
                  title={last ? 'Gardez au moins un sujet' : undefined}
                  onClick={() => onToggleTheme(family.code)}
                >
                  {typographic(family.label)}
                </ToggleChip>
              )
            })}
          </PanelBody>
        )}

        {open === 'groups' && (
          <PanelBody id={groupsPanelId} onClose={close}>
            <ToggleChip pressed={followed.length === 0} onClick={onAllGroups}>
              Tous les groupes
            </ToggleChip>
            {groups.map((group) => (
              <ToggleChip
                key={group.uid}
                pressed={followed.includes(group.abbrev)}
                title={group.label}
                onClick={() => onToggleGroup(group.abbrev)}
              >
                <PoliticalLogo
                  logo={GROUP_LOGOS[group.abbrev]}
                  fallback={group.abbrev}
                  color={null}
                  className="-ml-1.5 h-[26px] w-10"
                />
                {groupShortName(group)}
              </ToggleChip>
            ))}
          </PanelBody>
        )}
      </div>
    </>
  )
}
