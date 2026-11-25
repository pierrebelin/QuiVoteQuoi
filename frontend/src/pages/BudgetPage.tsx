import { useState } from 'react'
import { Link, useParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchJson } from '../lib/fetchJson'
import { useMediaQuery } from '../lib/useMediaQuery'
import { formatLawDate, missionSubtitle, spendingNote } from '../lib/budgetFormat'
import type { BudgetExerciseDto, BudgetMissionDto } from '../types/budget'
import { BottomSheet } from '../components/BottomSheet'
import { ErrorPanel, Loading } from '../components/ui'
import { Icon } from '../components/icons'
import { InfoTip } from '../components/InfoTip'
import { MissionDetail, MissionRow, OffBudgetRow } from '../components/budget/BudgetMission'
import { BudgetBreakdown } from '../components/budget/BudgetBreakdown'

/*
 * Budget de l'État voté en loi de finances initiale : le budget général par
 * mission, trié par montant, et les comptes spéciaux retenus à part.
 *
 * Rien n'est propre à une année : titre, loi, article, procédure et avertissement
 * viennent de l'exercice importé (`data/budget/lfi-<année>.json`). `/budget`
 * montre le plus récent, `/budget/<année>` un exercice précis.
 *
 * Toutes les missions sont listées, aucune n'est mise en avant (README §2) : à
 * 1280 px, le panneau de détail s'ouvre sur la première de la liste, la plus
 * grosse, par simple ordre de tri.
 */
export default function BudgetPage() {
  const { year } = useParams<{ year?: string }>()
  const query = useQuery<BudgetExerciseDto>({
    queryKey: ['budget', year ?? 'latest'],
    queryFn: () => fetchJson<BudgetExerciseDto>(year ? `/api/budget/${year}` : '/api/budget'),
  })

  if (query.isPending) return <Loading>Chargement du budget…</Loading>
  if (query.isError) {
    const missing = query.error instanceof Error && query.error.message === 'HTTP 404'
    return missing ? (
      <p className="py-10 text-sm text-ink-soft">
        Aucun budget de l’État importé{year ? ` pour ${year}` : ''}.
      </p>
    ) : (
      <ErrorPanel error={query.error} />
    )
  }
  return <BudgetView exercise={query.data} />
}

function BudgetView({ exercise }: { exercise: BudgetExerciseDto }) {
  const wide = useMediaQuery('(min-width: 1280px)')
  const [selectedName, setSelectedName] = useState<string | null>(null)
  const [sheetOpen, setSheetOpen] = useState(false)

  const { general_budget: general, off_budget: offBudget, year } = exercise
  const missions = general.missions
  const all = [...missions, ...offBudget]
  const largest = missions[0]?.cp_euros ?? 0

  const chosen = all.find((mission) => mission.name === selectedName)
  const panelMission: BudgetMissionDto | undefined = chosen ?? missions[0]

  const select = (mission: BudgetMissionDto) => {
    setSelectedName(mission.name)
    if (!wide) setSheetOpen(true)
  }
  const isSelected = (mission: BudgetMissionDto) => wide && panelMission?.name === mission.name

  const lawDate = formatLawDate(exercise.law.date)

  return (
    <div className="flex flex-col gap-10 xl:gap-12">
      <header className="flex max-w-[760px] flex-col gap-2.5">
        <p className="text-[13px] leading-[18px] text-ink-faint md:text-sm">Finances publiques</p>
        <h1 className="heading-3 text-ink md:heading-1">
          Budget de l’État {year}
          <InfoTip label={`À propos de cette page : Budget de l’État ${year}`} className="md:hidden">
            {spendingNote(year, new Date().getFullYear())}
          </InfoTip>
        </h1>
        <p className="text-base leading-normal text-ink-soft md:max-w-2xl md:text-lg md:leading-relaxed">
          Crédits votés en loi de finances initiale {year}
          {exercise.law.adoption_procedure === 'article_49_3'
            ? ', adoptée après recours au 49.3.'
            : '.'}
        </p>
        {/* Sous 768 px, la note est derrière l'infobulle du titre. */}
        <div
          role="note"
          className="mt-1 hidden items-center gap-2.5 rounded-[14px] bg-surface px-4 py-3 text-ink-soft md:flex md:max-w-2xl"
        >
          <Icon name="info" className="size-[18px] shrink-0 text-ink-faint" />
          <p className="text-sm leading-[1.45]">{spendingNote(year, new Date().getFullYear())}</p>
        </div>
        {exercise.available_years.length > 1 && (
          <nav
            aria-label="Exercice"
            className="mt-1 grid w-[180px] grid-flow-col auto-cols-fr gap-1 rounded-xl bg-surface p-[3px] md:mt-3"
          >
            {exercise.available_years.map((other) => (
              <Link
                key={other}
                to={`/budget/${other}`}
                aria-current={other === year ? 'page' : undefined}
                className={`inline-flex min-h-11 items-center justify-center rounded-[10px] px-2 text-sm font-semibold transition-colors ${
                  other === year ? 'bg-canvas text-ink shadow-seg' : 'text-ink-soft hover:text-ink'
                }`}
              >
                {other}
              </Link>
            ))}
          </nav>
        )}
      </header>

      <div className="flex flex-col gap-10 xl:grid xl:grid-cols-[minmax(0,1fr)_400px] xl:grid-rows-[auto_auto_auto_1fr] xl:items-start xl:gap-x-6 xl:gap-y-8">
        {/* Répartition : avant la liste, sur toute la largeur à 1280 px pour laisser
            le panneau de détail en haut de la colonne de droite. */}
        <div className="xl:col-span-2 xl:row-start-1 xl:pb-4">
          <BudgetBreakdown exercise={exercise} onSelectMission={select} />
        </div>

        <section
          aria-labelledby="budget-missions"
          className="flex flex-col gap-3 xl:col-start-1 xl:row-span-3 xl:row-start-2 xl:gap-4"
        >
          <div className="flex flex-col gap-1">
            <h2 id="budget-missions" className="heading-4 text-ink md:heading-3">
              Par mission
            </h2>
            <p className="text-[13px] leading-[18px] text-ink-faint md:text-sm md:leading-5">
              Triées par montant. {wide ? 'Cliquer' : 'Toucher'} une mission pour voir ses
              programmes.
            </p>
          </div>
          <div
            aria-hidden="true"
            className="hidden gap-4 px-3 text-[13px] font-semibold text-ink-faint xl:grid xl:grid-cols-[minmax(0,300px)_minmax(0,1fr)_88px_56px]"
          >
            <span>Mission</span>
            <span />
            <span className="text-right">Montant</span>
            <span className="text-right">Part</span>
          </div>
          <ul className="flex flex-col">
            {missions.map((mission) => (
              <MissionRow
                key={mission.name}
                mission={mission}
                total={general.cp_euros}
                largest={largest}
                wide={wide}
                selected={isSelected(mission)}
                onSelect={() => select(mission)}
              />
            ))}
          </ul>
        </section>

        {offBudget.length > 0 && (
          <section
            aria-labelledby="budget-hors-budget"
            className="flex flex-col gap-3 max-xl:band max-xl:py-8 xl:col-start-2 xl:row-start-2"
          >
            <div className="flex flex-col gap-1">
              <h2 id="budget-hors-budget" className="heading-4 text-ink xl:text-[22px] xl:leading-[26px]">
                Hors budget général
              </h2>
              <p className="text-[13px] leading-[18px] text-ink-faint md:text-sm md:leading-5">
                {offBudget.length > 1 ? 'Comptes spéciaux' : 'Compte spécial'}, non compris dans
                le total ci-dessus.
              </p>
            </div>
            <div className="flex flex-col xl:border-b xl:border-line">
              {offBudget.map((mission) => (
                <OffBudgetRow
                  key={mission.name}
                  mission={mission}
                  wide={wide}
                  selected={isSelected(mission)}
                  onSelect={() => select(mission)}
                />
              ))}
            </div>
          </section>
        )}

        {wide && panelMission && (
          <section
            aria-labelledby="budget-detail"
            className="sticky top-20 col-start-2 row-start-3 flex flex-col gap-5 rounded bg-surface p-6"
          >
            <div className="flex flex-col gap-1">
              <p className="text-[13px] leading-[17px] font-semibold text-ink-faint">Détail</p>
              <h2 id="budget-detail" className="heading-3 text-ink">
                {panelMission.name}
              </h2>
              <p className="text-sm leading-5 text-ink-faint">
                {missionSubtitle(panelMission, general.cp_euros, year)}
              </p>
            </div>
            <MissionDetail mission={panelMission} year={year} onCard />
          </section>
        )}
      </div>

      <p className="text-[13px] leading-[1.6] text-ink-soft xl:max-w-[900px] xl:border-t xl:border-line xl:pt-5 xl:text-sm">
        Source :{' '}
        <a href={exercise.law.legifrance_url} target="_blank" rel="noreferrer" className="link">
          loi n° {exercise.law.number} du {lawDate} de finances pour {year}
        </a>{' '}
        (article {exercise.law.credits_article}) ; fichier «{' '}
        <a href={exercise.source.url} target="_blank" rel="noreferrer" className="link">
          {exercise.source.title}
        </a>
        {' '}» publié le {formatLawDate(exercise.source.published_on)} sur budget.gouv.fr.
      </p>

      {!wide && (
        <BottomSheet
          open={sheetOpen && chosen !== undefined}
          onClose={() => setSheetOpen(false)}
          title={chosen?.name ?? ''}
        >
          {chosen && (
            <div className="min-h-0 overflow-y-auto px-5 pb-6">
              <p className="mb-4 text-sm leading-[21px] text-ink-soft">
                {missionSubtitle(chosen, general.cp_euros, year)}
              </p>
              <MissionDetail mission={chosen} year={year} />
            </div>
          )}
        </BottomSheet>
      )}
    </div>
  )
}
