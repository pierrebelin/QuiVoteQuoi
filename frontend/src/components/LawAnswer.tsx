import { LAW_STATE_TONES } from '../lib/lawState'
import type { LawAnswer as LawAnswerDto, LawState } from '../types/dossiers'
import { LawStateIcon } from './LawStateBadge'

const ANSWER_WORDS: Record<LawAnswerDto['kind'], string> = {
  not_yet: 'Pas encore.',
  soon: 'Bientôt.',
  yes: 'Oui.',
}

/**
 * Encadré « Ce texte est-il devenu loi ? » (RM-06) : la réponse, puis la
 * phrase à trous écrite par l'API. Seul le lien Légifrance s'ajoute ici.
 * `compact` : taille de la fiche de vote, où l'encadré précède le résultat.
 */
export default function LawAnswer({
  answer,
  lawState,
  compact = false,
  className = '',
}: {
  answer: LawAnswerDto
  lawState: LawState
  compact?: boolean
  className?: string
}) {
  const kind = lawState.kind
  const isLaw = kind === 'became_law'
  return (
    <div
      className={`flex flex-wrap items-baseline gap-x-[18px] rounded-[14px] px-4 py-3.5 sm:px-[18px] sm:py-4 ${LAW_STATE_TONES[kind]} ${className}`}
    >
      <p
        className={`w-full text-xs font-semibold tracking-[0.05em] uppercase ${
          kind === 'in_shuttle' ? 'text-ink-soft' : isLaw ? 'opacity-90' : ''
        }`}
      >
        Ce texte est-il devenu loi ?
      </p>
      <p
        className={`my-1 flex items-center gap-2 font-heading font-[560] tracking-[-0.01em] ${
          compact ? 'text-[22px] leading-[26px] sm:text-2xl' : 'text-[26px] leading-[30px] sm:text-[28px] sm:leading-8'
        }`}
      >
        <LawStateIcon kind={kind} className="size-5" />
        {ANSWER_WORDS[answer.kind]}
      </p>
      <p
        className={`text-sm leading-[21px] sm:min-w-[300px] sm:flex-1 sm:text-[15px] sm:leading-[22px] ${isLaw ? '' : 'text-ink'}`}
      >
        {answer.sentence}
        {isLaw && lawState.legifrance_url && (
          <>
            {' '}
            <a
              href={lawState.legifrance_url}
              target="_blank"
              rel="noreferrer"
              className="font-semibold underline decoration-2 underline-offset-[3px]"
            >
              Légifrance ↗
            </a>
          </>
        )}
      </p>
    </div>
  )
}
