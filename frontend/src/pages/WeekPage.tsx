import { useState, type ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import VotesTabs from '../components/VotesTabs'
import VoteCard from '../components/VoteCard'
import DayPills from '../components/week/DayPills'
import WeekTextCard from '../components/week/WeekTextCard'
import { CoverageNote } from '../components/ScrutinList'
import { InfoTip } from '../components/InfoTip'
import { ErrorPanel, Loading } from '../components/ui'
import { finalVoteCard, WHOLE_TEXT_KICKER } from '../components/voteCardData'
import { fetchJson } from '../lib/fetchJson'
import { formatDate } from '../types/scrutins'
import type { WeekResponse } from '../types/week'

const METHOD_PATH = '/votes-par-groupe/methode'

const votes = (count: number) => `${count.toLocaleString('fr-FR')} vote${count > 1 ? 's' : ''}`

function addDays(iso: string, days: number): string {
  const date = new Date(iso + 'T00:00:00Z')
  date.setUTCDate(date.getUTCDate() + days)
  return date.toISOString().slice(0, 10)
}

/** « Du 9 au 15 novembre 2026 », le mois et l'année du lundi omis quand ce sont les mêmes. */
function weekSpan(monday: string): string {
  const sunday = addDays(monday, 6)
  const sameMonth = monday.slice(0, 7) === sunday.slice(0, 7)
  const sameYear = monday.slice(0, 4) === sunday.slice(0, 4)
  const first = new Date(monday + 'T00:00:00').toLocaleDateString('fr-FR', {
    day: 'numeric',
    ...(sameMonth ? {} : { month: 'long' }),
    ...(sameYear ? {} : { year: 'numeric' }),
  })
  return `Du ${first} au ${formatDate(sunday)}`
}

function Chevron({ direction }: { direction: 'left' | 'right' }) {
  return (
    <svg viewBox="0 0 20 20" className="size-5" fill="none" stroke="currentColor" strokeWidth={2} aria-hidden="true">
      <path d={direction === 'left' ? 'M12 5l-5 5 5 5' : 'M8 5l5 5-5 5'} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

const ROUND = 'flex size-11 shrink-0 items-center justify-center rounded-full border border-line bg-surface'

/** Flèche vers une semaine voisine, inactive à la borne (RM-12). */
function WeekArrow({ to, label, direction }: { to: string | null; label: string; direction: 'left' | 'right' }) {
  if (!to) {
    return (
      <span role="link" aria-disabled="true" aria-label={label} className={`${ROUND} text-ink-faint/60`}>
        <Chevron direction={direction} />
      </span>
    )
  }
  return (
    <Link to={to} aria-label={label} className={`${ROUND} text-ink hover:border-line-strong`}>
      <Chevron direction={direction} />
    </Link>
  )
}

/** Lien de pied de page en carte : une légende, puis la destination. */
function FooterLink({ to, caption, children }: { to: string; caption: string; children: ReactNode }) {
  return (
    <Link
      to={to}
      className="flex min-h-14 items-center justify-between gap-3 rounded-[18px] border border-line bg-surface px-4 py-3 hover:border-line-strong"
    >
      <span>
        <span className="block text-[12.5px] text-ink-faint">{caption}</span>
        <span className="block text-[14.5px] font-semibold">{children}</span>
      </span>
      <Chevron direction="right" />
    </Link>
  )
}

/**
 * La semaine de votes (CU-01) : le sélecteur de semaine, les nombres de toute
 * l'Assemblée (RM-05), les jours (RM-06), les votes sur l'ensemble d'un texte
 * avec la position de chaque groupe (RM-07), puis tous les votes texte par
 * texte (RM-08 à RM-10). Le filtre de jour ne touche ni les nombres ni les
 * pastilles (RM-11).
 */
export default function WeekPage() {
  const { date } = useParams()
  const [day, setDay] = useState<string | null>(null)
  const { data, error, isPending } = useQuery({
    queryKey: ['week', date ?? 'current'],
    queryFn: () => fetchJson<WeekResponse>(date ? `/api/semaine/${date}` : '/api/semaine'),
  })
  const legislature = useQuery({
    queryKey: ['scrutins', 'total'],
    queryFn: () => fetchJson<{ total: number }>('/api/scrutins?limit=1&offset=0'),
  })

  if (isPending) return <Loading>Chargement de la semaine…</Loading>
  if (error) return <ErrorPanel error={error} />

  const today = new Date().toISOString().slice(0, 10)
  const { totals } = data
  const finalVotes = day ? data.final_votes.filter((vote) => vote.date === day) : data.final_votes
  const finalLabels = new Map(data.final_votes.map((vote) => [vote.scrutin_uid, vote.text_label]))
  const shownTexts = data.texts.filter((text) => !day || text.scrutins.some((scrutin) => scrutin.date === day))
  const shownVotes = day ? (data.days.find((d) => d.date === day)?.votes ?? 0) : totals.votes
  const shownTextCount = shownTexts.filter((text) => text.dossier_uid).length
  const empty = totals.votes === 0
  const previousLabel = data.previous ? weekSpan(data.previous.monday) : null

  return (
    <div className="flex flex-col gap-[22px] md:gap-7">
      <VotesTabs current="semaine" />

      {data.current_week_empty && (
        <p className="rounded-[14px] bg-surface-soft px-4 py-2.5 text-[13.5px] text-ink-soft">
          Aucun vote publié cette semaine pour l’instant.
        </p>
      )}

      <div className="flex flex-col gap-[22px] md:flex-row md:items-end md:justify-between md:gap-8">
        <div className="flex flex-col gap-3.5 md:gap-4">
          <div>
            <p className="mb-0.5 text-[13px] text-ink-faint md:text-sm">Assemblée nationale</p>
            <h1 className="flex items-center gap-1 font-heading text-[30px] leading-[34px] font-[460] tracking-[-0.01em] md:gap-1.5 md:text-[40px] md:leading-[44px]">
              La semaine
              <InfoTip label="À propos de cette page">
                <p>Tous les votes publics de la semaine, sans sélection.</p>
                <p>{data.coverage_note}</p>
              </InfoTip>
            </h1>
          </div>
          <div className="flex items-center gap-2.5 md:gap-3.5">
            <WeekArrow
              to={data.previous ? `/semaine/${data.previous.monday}` : null}
              label={previousLabel ? `Semaine précédente, ${previousLabel.toLowerCase()}` : 'Semaine précédente'}
              direction="left"
            />
            <div className="flex-1 text-center md:flex-none md:text-left">
              <p className="font-heading text-[16.5px] leading-[22px] font-[460] tracking-[-0.01em] whitespace-nowrap md:text-[22px] md:leading-7">
                {weekSpan(data.monday)}
              </p>
              {data.is_current && (
                <span className="inline-flex rounded-full bg-info-soft px-2 text-xs leading-[18px] font-semibold text-info md:text-[12.5px]">
                  Semaine en cours
                </span>
              )}
            </div>
            <WeekArrow to={data.next ? `/semaine/${data.next}` : null} label="Semaine suivante" direction="right" />
          </div>
        </div>

        <div className="flex flex-col gap-2.5 rounded-[18px] border border-line bg-surface p-4 md:w-[520px] md:gap-3 md:px-6 md:py-5">
          <div className="grid grid-cols-3 gap-2 md:gap-4">
            {[
              { value: totals.votes, label: `vote${totals.votes > 1 ? 's' : ''} public${totals.votes > 1 ? 's' : ''}`, tone: '' },
              { value: totals.adopted, label: `adopté${totals.adopted > 1 ? 's' : ''}`, tone: 'text-yes' },
              { value: totals.rejected, label: `rejeté${totals.rejected > 1 ? 's' : ''}`, tone: 'text-no' },
            ].map((stat) => (
              <div key={stat.label}>
                <p className={`figure text-[30px] leading-[34px] tracking-normal md:text-[40px] md:leading-[44px] ${stat.tone}`}>
                  {stat.value.toLocaleString('fr-FR')}
                </p>
                <p className="text-[12.5px] leading-[17px] text-ink-soft md:text-[13.5px]">{stat.label}</p>
              </div>
            ))}
          </div>
          {totals.others.length > 0 && (
            <p className="text-[12.5px] text-ink-soft">
              Dont {totals.others.map((other) => `${other.count} ${other.label}`).join(', ')}
            </p>
          )}
          <p className="border-t border-line pt-2.5 text-[12.5px] leading-[18px] text-ink-faint md:pt-3 md:text-[13px]">
            Sur {data.texts_count} texte{data.texts_count > 1 ? 's' : ''}
            {data.last_published &&
              ` · dernier vote publié le ${new Date(data.last_published + 'T00:00:00').toLocaleDateString('fr-FR', { day: 'numeric', month: 'long' })}`}
          </p>
        </div>
      </div>

      <DayPills days={data.days} selected={day} onSelect={setDay} />

      {empty ? (
        <p className="rounded-[18px] border border-line bg-surface px-4 py-6 text-sm text-ink-soft">
          Aucun vote public publié cette semaine.
        </p>
      ) : (
        <>
          <section className="flex flex-col gap-3">
            <h2 className="font-heading text-xl leading-[26px] font-[460] tracking-[-0.01em]">
              Votes sur l’ensemble d’un texte{' '}
              <InfoTip label="Critère des votes sur l’ensemble" className="align-middle">
                <p>
                  Critère : un vote dont l’objet officiel porte sur l’ensemble d’un texte. Tous les autres votes de
                  la semaine sont listés plus bas.
                </p>
                <p>
                  <Link to={METHOD_PATH} className="link">
                    Méthode des votes sur l’ensemble
                  </Link>
                </p>
              </InfoTip>
            </h2>
            {finalVotes.length > 0 ? (
              <div className="grid items-start gap-4 lg:grid-cols-2">
                {finalVotes.map((vote) => (
                  <VoteCard
                    key={vote.scrutin_uid}
                    size="full"
                    {...finalVoteCard(vote, data.groups)}
                    kicker={
                      /solennel/i.test(vote.ballot_type_label)
                        ? `${WHOLE_TEXT_KICKER} · scrutin solennel`
                        : WHOLE_TEXT_KICKER
                    }
                  />
                ))}
              </div>
            ) : (
              <p className="text-[13.5px] text-ink-soft">
                Aucun vote sur l’ensemble d’un texte {day ? 'ce jour' : 'cette semaine'}.
              </p>
            )}
          </section>

          <section className="flex flex-col gap-3">
            <div>
              <h2 className="font-heading text-xl leading-[26px] font-[460] tracking-[-0.01em]">
                Tous les votes, texte par texte
              </h2>
              <p className="mt-0.5 text-[13.5px] leading-5 text-ink-soft">
                {votes(shownVotes)} sur {shownTextCount} texte{shownTextCount > 1 ? 's' : ''}
                {day ? ' ce jour' : ''}, dans l’ordre du premier vote de la semaine.
              </p>
            </div>
            <div className="grid items-start gap-4 lg:grid-cols-3">
              {shownTexts.map((text) => (
                <WeekTextCard
                  key={`${text.dossier_uid ?? 'sans-dossier'}-${day ?? 'semaine'}`}
                  text={text}
                  day={day}
                  today={today}
                  finalLabels={finalLabels}
                />
              ))}
            </div>
          </section>
        </>
      )}

      <div className="grid gap-2.5 md:grid-cols-2 md:gap-4">
        {data.previous && previousLabel && (
          <FooterLink to={`/semaine/${data.previous.monday}`} caption="Semaine précédente">
            {previousLabel} · {votes(data.previous.votes)}
          </FooterLink>
        )}
        <FooterLink to="/scrutins" caption="Sans découpage par semaine">
          Tous les votes de la législature
          {legislature.data && ` · ${votes(legislature.data.total)}`}
        </FooterLink>
      </div>
      <CoverageNote note={data.coverage_note} />
    </div>
  )
}
