import { useState } from 'react'
import { Link, useParams } from 'react-router'
import { useQuery } from '@tanstack/react-query'
import { fetchJson } from '../lib/fetchJson'
import { useMediaQuery } from '../lib/useMediaQuery'
import { amountParts, formatAmount, formatLawDate, missionSubtitle } from '../lib/budgetFormat'
import type { BudgetExerciseDto, BudgetMissionDto } from '../types/budget'
import { BottomSheet } from '../components/BottomSheet'
import { ErrorPanel, Loading } from '../components/ui'
import { Icon } from '../components/icons'
import {
  MissionDetail,
  MissionRow,
  OffBudgetRow,
} from '../components/budget/BudgetMission'

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
  const programmeCount = missions.reduce((sum, mission) => sum + mission.programmes.length, 0)

  const chosen = all.find((mission) => mission.name === selectedName)
  const panelMission: BudgetMissionDto | undefined = chosen ?? missions[0]

  const select = (mission: BudgetMissionDto) => {
    setSelectedName(mission.name)
    if (!wide) setSheetOpen(true)
  }
  const isSelected = (mission: BudgetMissionDto) => wide && panelMission?.name === mission.name

  const total = amountParts(general.cp_euros)
  const refunds = general.refunds_and_rebates_cp_euros
  const lawDate = formatLawDate(exercise.law.date)

  return (
    <div className="flex flex-col gap-6 xl:gap-7">
      <header className="flex max-w-[760px] flex-col gap-2.5">
        <p className="text-[13px] leading-[18px] text-ink-faint">Finances publiques</p>
        <h1 className="heading-3 text-ink xl:text-[40px] xl:leading-[44px]">
          Budget de l’État {year}
        </h1>
        <p className="text-[15px] leading-[22px] text-ink-soft">
          Crédits votés en loi de finances initiale {year}
          {exercise.law.adoption_procedure === 'article_49_3'
            ? ', adoptée après recours au 49.3.'
            : '.'}
        </p>
        <div
          role="note"
          className="mt-1 flex items-start gap-2.5 rounded-[14px] bg-surface-soft px-3.5 py-3 text-ink-soft"
        >
          <Icon name="info" className="mt-px size-[18px] shrink-0" />
          <p className="text-[13.5px] leading-5">
            Montants votés, pas encore dépensés. Les dépenses réelles de {year} seront connues
            mi-{year + 1}.
          </p>
        </div>
        {exercise.available_years.length > 1 && (
          <nav aria-label="Exercice" className="mt-1 flex flex-wrap gap-2 text-sm">
            {exercise.available_years.map((other) => (
              <Link
                key={other}
                to={`/budget/${other}`}
                aria-current={other === year ? 'page' : undefined}
                className={`rounded-full border px-3 py-1 ${
                  other === year
                    ? 'border-ink bg-ink text-canvas'
                    : 'border-line bg-surface text-ink-soft hover:border-line-strong'
                }`}
              >
                {other}
              </Link>
            ))}
          </nav>
        )}
      </header>

      <div className="flex flex-col gap-6 xl:grid xl:grid-cols-[minmax(0,1fr)_400px] xl:grid-rows-[auto_auto_1fr] xl:items-start">
        {/* Chiffre clé : avant la liste au téléphone, en tête de colonne à 1280 px. */}
        <div className="flex flex-col gap-1.5 rounded border border-line bg-surface p-[18px] xl:col-start-2 xl:row-start-1">
          <p className="text-[13.5px] leading-[19px] text-ink-soft">
            Crédits de paiement du budget général
          </p>
          <p className="figure text-[44px] text-ink xl:text-[52px]">
            {total.value}
            <span className="ml-1.5 text-[0.5em] font-medium">{total.unit}</span>
          </p>
          {refunds !== null && (
            <p className="text-[12.5px] leading-[18px] text-ink-faint">
              Dont {formatAmount(refunds)} de remboursements et dégrèvements d’impôts ;{' '}
              {formatAmount(general.cp_euros - refunds)} hors ceux-ci.
            </p>
          )}
          <p className="mt-1.5 border-t border-line pt-2.5 text-[12.5px] leading-[18px] text-ink-soft">
            {missions.length} missions · {programmeCount} programmes · article{' '}
            {exercise.law.credits_article} de la loi n° {exercise.law.number} du {lawDate}
          </p>
        </div>

        <section
          aria-labelledby="budget-missions"
          className="flex flex-col gap-3 xl:col-start-1 xl:row-span-3 xl:row-start-1 xl:gap-3.5 xl:rounded xl:border xl:border-line xl:bg-surface xl:px-3.5 xl:pt-5 xl:pb-3.5"
        >
          <div className="flex flex-col gap-0.5 xl:px-1.5">
            <h2 id="budget-missions" className="heading-4 text-ink">
              Par mission
            </h2>
            <p className="text-[13px] leading-[18px] text-ink-soft">
              Triées par montant. {wide ? 'Cliquer' : 'Toucher'} une mission pour voir ses
              programmes.
            </p>
          </div>
          <div
            aria-hidden="true"
            className="hidden gap-4 px-[15px] text-xs text-ink-faint xl:grid xl:grid-cols-[minmax(0,300px)_minmax(0,1fr)_88px_56px]"
          >
            <span>Mission</span>
            <span />
            <span className="text-right">Montant</span>
            <span className="text-right">Part</span>
          </div>
          <ul className="flex flex-col gap-2 xl:gap-0.5">
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
            className="flex flex-col gap-3 xl:col-start-2 xl:row-start-2"
          >
            <div className="flex flex-col gap-0.5">
              <h2 id="budget-hors-budget" className="heading-4 text-ink">
                Hors budget général
              </h2>
              <p className="text-[13px] leading-[18px] text-ink-soft">
                {offBudget.length > 1 ? 'Comptes spéciaux' : 'Compte spécial'}, non compris dans
                le total ci-dessus.
              </p>
            </div>
            {offBudget.map((mission) => (
              <OffBudgetRow
                key={mission.name}
                mission={mission}
                wide={wide}
                selected={isSelected(mission)}
                onSelect={() => select(mission)}
              />
            ))}
          </section>
        )}

        {wide && panelMission && (
          <section
            aria-labelledby="budget-detail"
            className="sticky top-20 col-start-2 row-start-3 flex flex-col gap-3.5 rounded border border-accent bg-surface p-[18px]"
          >
            <div>
              <p className="text-[12.5px] leading-[17px] text-ink-faint">Détail</p>
              <h2 id="budget-detail" className="heading-4 text-ink">
                {panelMission.name}
              </h2>
              <p className="text-[13px] leading-[19px] text-ink-soft">
                {missionSubtitle(panelMission, general.cp_euros, year)}
              </p>
            </div>
            <MissionDetail mission={panelMission} />
          </section>
        )}
      </div>

      <p className="text-[12.5px] leading-[18px] text-ink-faint">
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
              <p className="mb-3.5 text-[13px] leading-[19px] text-ink-soft">
                {missionSubtitle(chosen, general.cp_euros, year)}
              </p>
              <MissionDetail mission={chosen} />
            </div>
          )}
        </BottomSheet>
      )}
    </div>
  )
}
