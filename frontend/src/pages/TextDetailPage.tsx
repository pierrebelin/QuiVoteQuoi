import { useQuery } from '@tanstack/react-query'
import { Link, useParams } from 'react-router'
import { BackLink, Card, Loading } from '../components/ui'
import { FamilyBadges, UnassignedNote } from '../components/ThemeBadges'
import VoteCard from '../components/VoteCard'
import { isWholeTextVote, scrutinDetailCard, textScrutinCard } from '../components/voteCardData'
import { fetchJson } from '../lib/fetchJson'
import type { ScrutinDetailDto } from '../types/scrutins'
import { formatDate } from '../types/scrutins'
import type { TextDetailResponse } from '../types/themes'

export default function TextDetailPage() {
  const { key } = useParams<{ key: string }>()

  const text = useQuery({
    queryKey: ['texts', key],
    queryFn: (): Promise<TextDetailResponse> =>
      fetch(`/api/texts/${encodeURIComponent(key!)}`).then((res) => {
        if (res.status === 404) throw new Error('Texte inconnu')
        if (!res.ok) throw new Error('Chargement impossible')
        return res.json()
      }),
  })

  // Le dernier vote sur l'ensemble du texte, s'il y en a un, porte la
  // position de chaque groupe : il est lu dans le détail du scrutin.
  const whole = text.data?.scrutins.find((scrutin) => isWholeTextVote(scrutin.subject))
  const wholeDetail = useQuery({
    queryKey: ['scrutin', whole?.uid],
    enabled: Boolean(whole),
    queryFn: () => fetchJson<ScrutinDetailDto>(`/api/scrutins/${whole!.uid}`),
    retry: false,
  })

  if (text.isLoading)
    return <p className="animate-pulse text-ink-faint">Chargement…</p>
  if (text.error)
    return <p className="text-no">{(text.error as Error).message}</p>

  const data = text.data!
  const others = data.scrutins.filter((scrutin) => scrutin.uid !== whole?.uid)
  const motives = data.families.filter((family) => family.motive)

  return (
    <div className="mx-auto max-w-3xl space-y-5">
      <div className="space-y-2">
        <BackLink to="/scrutins">Votes</BackLink>
        <p className="text-xs font-semibold tracking-[0.06em] text-ink-faint uppercase">Texte</p>
        <h2 className="heading-2">{data.label}</h2>
        {data.families.length > 0 ? (
          <FamilyBadges families={data.families} />
        ) : (
          <UnassignedNote outcome={data.last_attempt_outcome} />
        )}
        {motives.length > 0 && (
          <details className="text-xs text-ink-soft">
            <summary className="cursor-pointer text-ink-faint">Pourquoi ces sujets</summary>
            <ul className="mt-1.5 space-y-0.5">
              {motives.map((family) => (
                <li key={family.code}>
                  <span className="font-medium text-ink">{family.label}</span> — {family.motive}
                </li>
              ))}
            </ul>
          </details>
        )}
        {data.dossier_uid && (
          <p>
            <Link
              to={`/dossiers/${data.dossier_uid}`}
              className="inline-flex min-h-11 items-center text-sm font-medium link"
            >
              Le texte et tout son parcours →
            </Link>
          </p>
        )}
      </div>

      <section className="space-y-3" aria-labelledby="votes-du-texte">
        <h3 id="votes-du-texte" className="text-sm font-semibold text-ink-soft">
          {data.scrutin_count} vote{data.scrutin_count > 1 ? 's' : ''}
          {data.first_vote && data.last_vote && (
            <>
              {' · '}
              {data.first_vote === data.last_vote
                ? formatDate(data.first_vote)
                : `du ${formatDate(data.first_vote)} au ${formatDate(data.last_vote)}`}
            </>
          )}
        </h3>

        {data.scrutins.length === 0 && <p className="text-sm text-ink-faint">Aucun vote listé.</p>}

        {whole &&
          (wholeDetail.data ? (
            <VoteCard size="full" {...scrutinDetailCard(wholeDetail.data)} />
          ) : wholeDetail.isError ? (
            <VoteCard size="compact" {...textScrutinCard(whole)} />
          ) : (
            <Card className="px-4 py-6">
              <Loading>Chargement du vote sur l’ensemble…</Loading>
            </Card>
          ))}

        {others.map((scrutin) => (
          <VoteCard key={scrutin.uid} size="compact" {...textScrutinCard(scrutin)} />
        ))}

        {data.scrutin_count > data.scrutins.length && (
          <p className="text-xs text-ink-faint">
            {data.scrutins.length} votes affichés sur {data.scrutin_count}.
          </p>
        )}
      </section>
    </div>
  )
}
