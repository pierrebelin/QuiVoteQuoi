import { useQuery } from '@tanstack/react-query'
import type { ReactNode } from 'react'
import { Link } from 'react-router'
import { QUESTIONS, REPERES } from '../../content/comprendre'
import { fetchJson } from '../../lib/fetchJson'
import type { FinalVoteDto, FinalVoteListResponse, GroupDto } from '../../types/finalVotes'
import LatestVoteHemicycle from '../LatestVoteHemicycle'
import { AssemblyFooter, GroupsByPosition, VoteCardHeader } from '../VoteCard'
import { finalVoteCard } from '../voteCardData'

/*
 * « Lire un vote » : la carte du dernier vote sur l'ensemble d'un texte,
 * découpée en morceaux, chacun suivi des repères qui l'expliquent.
 *
 * La carte est servie en direct, comme sur l'accueil (même requête, même
 * cache) : la page n'affiche ainsi aucun chiffre figé, et les repères sont
 * écrits pour rester vrais quel que soit le vote affiché.
 */

/** Numéro d'un repère : posé sur le bord gauche du morceau qu'il désigne. */
function Numero({ n, flottant = false }: { n: number; flottant?: boolean }) {
  return (
    <span
      aria-hidden
      className={`${
        flottant ? 'absolute top-0 -left-[29px] z-[2]' : ''
      } inline-flex size-[22px] shrink-0 items-center justify-center rounded-full bg-accent text-xs leading-[22px] font-bold text-surface ring-2 ring-surface`}
    >
      {n}
    </span>
  )
}

function Repere({ n, children }: { n: number; children: ReactNode }) {
  return (
    <div className="relative">
      <Numero n={n} flottant />
      {children}
    </div>
  )
}

/** Un morceau de carte, dans le même cadre que la carte entière. */
function Morceau({ children }: { children: ReactNode }) {
  return (
    <div className="pl-3.5">
      <div className="flex flex-col gap-2.5 rounded border border-line bg-surface p-4 shadow-card sm:px-5">
        {children}
      </div>
    </div>
  )
}

const QUESTION_PAR_ID = new Map(QUESTIONS.map((q) => [q.id, q.question]))

function Explications({ numeros }: { numeros: number[] }) {
  return (
    <ol className="lg:divide-y lg:divide-line">
      {numeros.map((n) => {
        const repere = REPERES[n - 1]
        return (
          <li key={n} value={n} className="mt-2.5 flex gap-2.5 lg:mt-0 lg:gap-3 lg:py-3">
            <Numero n={n} />
            <div className="flex-1">
              <p className="text-sm leading-5 font-semibold text-ink lg:text-[15px] lg:leading-[22px]">
                {repere.libelle}
              </p>
              <p className="mt-0.5 text-sm leading-[21px] text-ink-soft">{repere.texte}</p>
              <p className="mt-1 text-[13px] leading-5">
                <Link to={{ hash: repere.question }} className="link">
                  {QUESTION_PAR_ID.get(repere.question)}
                </Link>
              </p>
            </div>
          </li>
        )
      })}
    </ol>
  )
}

function Etape({ numeros, children }: { numeros: number[]; children: ReactNode }) {
  return (
    <div className="mb-[22px] lg:mb-0 lg:grid lg:grid-cols-[minmax(0,360px)_minmax(0,560px)] lg:items-start lg:gap-x-12 lg:border-t lg:border-line lg:py-5">
      <Morceau>{children}</Morceau>
      <Explications numeros={numeros} />
    </div>
  )
}

export default function LireUnVote() {
  const latest = useQuery({
    queryKey: ['votes-finaux', 'dernier'],
    queryFn: () =>
      fetchJson<FinalVoteListResponse>('/api/votes-finaux?tous_groupes=true&limit=1&offset=0'),
    retry: false,
  })
  const vote = latest.data?.items[0]

  return (
    <section aria-labelledby="lire-un-vote" className="lg:mb-4">
      <h3 id="lire-un-vote" className="heading-4">
        Lire un vote
      </h3>
      <p className="mt-1 mb-3.5 text-[13px] leading-[19px] text-ink-faint">
        La carte du dernier vote sur l’ensemble d’un texte, découpée et expliquée morceau par morceau.
      </p>

      {vote ? (
        <Carte vote={vote} groups={latest.data?.groups ?? []} />
      ) : (
        <p className="rounded border border-line bg-surface px-4 py-6 text-sm text-ink-soft">
          {latest.isError
            ? 'Le dernier vote n’est pas disponible pour le moment.'
            : 'Chargement du dernier vote…'}
        </p>
      )}
    </section>
  )
}

function Carte({ vote, groups }: { vote: FinalVoteDto; groups: GroupDto[] }) {
  const card = finalVoteCard(vote, groups)
  return (
    <>
      <Etape numeros={[1, 2, 3]}>
        <div className="space-y-1.5">
          <Repere n={1}>
            <VoteCardHeader
              adopted={card.adopted}
              outcome={card.outcome}
              outcomeTitle={card.outcomeTitle}
              reading={card.reading}
              date={card.date}
            />
          </Repere>
          <Repere n={2}>
            <p className="text-[12.5px] font-medium text-ink-soft">{card.kicker}</p>
          </Repere>
          <Repere n={3}>
            <h4 className="heading-5 text-ink">
              <Link to={card.to} className="rounded transition-colors hover:text-accent">
                {card.title}
              </Link>
            </h4>
          </Repere>
        </div>
      </Etape>
      <Etape numeros={[4]}>
        <Repere n={4}>
          <LatestVoteHemicycle scrutinUid={vote.scrutin_uid} tally={vote.synthesis} labelledBy="lire-un-vote" />
        </Repere>
      </Etape>
      {card.groups && (
        <Etape numeros={[5]}>
          <Repere n={5}>
            <GroupsByPosition {...card.groups} />
          </Repere>
        </Etape>
      )}
      <Etape numeros={[6]}>
        <Repere n={6}>
          <AssemblyFooter tally={card.tally} />
        </Repere>
      </Etape>
    </>
  )
}
