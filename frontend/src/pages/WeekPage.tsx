import type { ReactNode } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Link, useParams, useSearchParams } from 'react-router'
import VotesTabs from '../components/VotesTabs'
import VoteCard from '../components/VoteCard'
import AdvancedTexts, { ADVANCED_ANCHOR } from '../components/week/AdvancedTexts'
import DayPills from '../components/week/DayPills'
import WeekFigures from '../components/week/WeekFigures'
import WeekTextCard from '../components/week/WeekTextCard'
import { CoverageNote } from '../components/ScrutinList'
import { InfoTip } from '../components/InfoTip'
import { ErrorPanel, Loading } from '../components/ui'
import { finalVoteCard, WHOLE_TEXT_KICKER } from '../components/voteCardData'
import { fetchJson } from '../lib/fetchJson'
import { votesWord, weekSpan } from '../lib/week'
import type { WeekResponse } from '../types/week'

const METHOD_PATH = '/votes-par-groupe/methode'
/** Méthode publique des Reels de votes (REELS RM-15). */
const REELS_METHOD_PATH = '/reels/methode'

function Chevron({ direction }: { direction: 'left' | 'right' }) {
  return (
    <svg viewBox="0 0 20 20" className="size-5" fill="none" stroke="currentColor" strokeWidth={2} aria-hidden="true">
      <path d={direction === 'left' ? 'M12 5l-5 5 5 5' : 'M8 5l5 5-5 5'} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  )
}

const ROUND = 'flex size-11 shrink-0 items-center justify-center rounded-full bg-surface'

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
    <Link to={to} aria-label={label} className={`${ROUND} text-ink transition-colors hover:bg-surface-soft`}>
      <Chevron direction={direction} />
    </Link>
  )
}

/** Lien de pied de page en carte : une légende, puis la destination. */
function FooterLink({ to, caption, children }: { to: string; caption: string; children: ReactNode }) {
  return (
    <Link
      to={to}
      className="flex min-h-16 items-center justify-between gap-3 border-t border-line py-3.5 hover:text-accent-ink"
    >
      <span>
        <span className="block text-[13px] text-ink-faint">{caption}</span>
        <span className="block text-[15px] leading-snug font-semibold">{children}</span>
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
 * pastilles (RM-11). Les textes qui ont avancé (navette RM-08) viennent
 * avant les jours : la semaine entière, sans filtre de jour.
 */
export default function WeekPage() {
  const { date } = useParams()
  const [searchParams, setSearchParams] = useSearchParams()
  const day = searchParams.get('jour')
  const setDay = (next: string | null) =>
    setSearchParams(
      (params) => {
        if (next) params.set('jour', next)
        else params.delete('jour')
        return params
      },
      { replace: true },
    )
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
    <div className="flex flex-col gap-7 md:gap-9">
      <VotesTabs current="semaine" />

      {/* Seulement quand la page, ouverte sans semaine, se rabat sur la dernière semaine publiée :
          sur une semaine passée choisie, la semaine en cours n'est pas celle affichée. */}
      {!date && data.current_week_empty && (
        <p className="-mb-2 text-[13.5px] text-ink-soft">Aucun vote publié cette semaine pour l’instant.</p>
      )}

      <div className="flex flex-col gap-6 md:flex-row md:items-end md:justify-between md:gap-8">
        <div className="flex flex-col gap-4 md:gap-5">
          <div>
            <p className="mb-0.5 text-[13px] text-ink-faint md:text-sm">Assemblée nationale</p>
            <h1 className="flex items-center gap-1 heading-2 md:gap-1.5 md:heading-1">
              La semaine
              <InfoTip label="À propos de cette page" className="md:hidden">
                <p>Tous les votes publics de la semaine, sans sélection.</p>
                <p>{data.coverage_note}</p>
              </InfoTip>
            </h1>
            {/* À partir de 768 px, le texte de l'infobulle est écrit sous le titre ; la mention de
                couverture reste en bas de page. */}
            <p className="mt-3 hidden max-w-2xl text-lg leading-relaxed text-ink-soft md:block">
              Tous les votes publics de la semaine, sans sélection.
            </p>
          </div>
          <div className="flex items-center gap-2.5 md:gap-3.5">
            <WeekArrow
              to={data.previous ? `/semaine/${data.previous.monday}` : null}
              label={previousLabel ? `Semaine précédente, ${previousLabel.toLowerCase()}` : 'Semaine précédente'}
              direction="left"
            />
            <div className="flex-1 text-center md:flex-none md:text-left">
              <p className="font-heading text-[16.5px] leading-6 font-[560] tracking-[-0.01em] whitespace-nowrap md:text-2xl md:leading-7">
                {weekSpan(data.monday)}
              </p>
              {data.is_current && (
                <span className="inline-flex rounded-full bg-accent-soft px-2 text-xs leading-[18px] font-semibold text-accent-ink md:text-[12.5px]">
                  Semaine en cours
                </span>
              )}
            </div>
            <WeekArrow to={data.next ? `/semaine/${data.next}` : null} label="Semaine suivante" direction="right" />
          </div>
        </div>

        <div className="flex flex-col gap-3 border-t border-line pt-4 md:w-[520px] md:gap-3.5 md:border-t-0 md:pt-0">
          <WeekFigures totals={totals} large />
          {totals.others.length > 0 && (
            <p className="text-[12.5px] text-ink-soft">
              Dont {totals.others.map((other) => `${other.count} ${other.label}`).join(', ')}
            </p>
          )}
          <p className="text-[13px] leading-[18px] text-ink-faint">
            Sur {data.texts_count} texte{data.texts_count > 1 ? 's' : ''}
            {data.last_published &&
              ` · dernier vote publié le ${new Date(data.last_published + 'T00:00:00').toLocaleDateString('fr-FR', { day: 'numeric', month: 'long' })}`}
          </p>
          {data.advanced_count > 0 && (
            <p className="flex items-baseline gap-2 border-y border-line py-3 text-sm md:py-3.5">
              <span className="figure text-[26px] leading-[30px] tracking-normal md:text-[28px] md:leading-8">
                {data.advanced_count.toLocaleString('fr-FR')}
              </span>
              {data.advanced_count > 1 ? 'textes ont avancé' : 'texte a avancé'}
              <a
                href={`#${ADVANCED_ANCHOR}`}
                aria-label="Voir les textes qui ont avancé"
                className="ml-auto text-[13px] font-semibold link"
              >
                Voir ↓
              </a>
            </p>
          )}
        </div>
      </div>

      {data.advanced.length > 0 && <AdvancedTexts advanced={data.advanced} />}

      <DayPills days={data.days} selected={day} onSelect={setDay} />

      {empty ? (
        <p className="rounded bg-surface px-4 py-6 text-sm text-ink-soft">
          Aucun vote public publié cette semaine.
        </p>
      ) : (
        <>
          <section className="flex flex-col gap-4">
            <h2 className="flex items-center gap-1 heading-4 md:heading-3">
              <span className="text-balance">Votes sur l’ensemble d’un texte</span>
              <InfoTip label="Critère des votes sur l’ensemble" className="md:hidden">
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
            <div className="-mt-2 hidden max-w-2xl flex-col gap-1.5 text-[15px] leading-normal text-ink-soft md:flex">
              <p>
                Critère : un vote dont l’objet officiel porte sur l’ensemble d’un texte. Tous les autres votes de la
                semaine sont listés plus bas.
              </p>
              <p>
                <Link to={METHOD_PATH} className="link">
                  Méthode des votes sur l’ensemble
                </Link>
              </p>
            </div>
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

          <section className="flex flex-col gap-4">
            <div>
              <h2 className="heading-4 md:heading-3">Tous les votes, texte par texte</h2>
              <p className="mt-1 text-[13.5px] leading-5 text-ink-soft">
                {votesWord(shownVotes)} sur {shownTextCount} texte{shownTextCount > 1 ? 's' : ''}
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

      <div className="grid md:grid-cols-2 md:gap-x-6 xl:grid-cols-3">
        {data.previous && previousLabel && (
          <FooterLink to={`/semaine/${data.previous.monday}`} caption="Semaine précédente">
            {previousLabel} · {votesWord(data.previous.votes)}
          </FooterLink>
        )}
        <FooterLink to="/scrutins" caption="Sans découpage par semaine">
          Tous les votes de la législature
          {legislature.data && ` · ${votesWord(legislature.data.total)}`}
        </FooterLink>
        <FooterLink to={REELS_METHOD_PATH} caption="En vidéo sur Instagram et TikTok">
          Méthode des Reels · chaque jour à 18 h
        </FooterLink>
      </div>
      <CoverageNote note={data.coverage_note} />
    </div>
  )
}
