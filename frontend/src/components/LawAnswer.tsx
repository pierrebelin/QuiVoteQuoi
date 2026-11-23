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
 * `compact` : la fiche de vote, où l'encadré est une carte blanche dans une bande grise.
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
  // Fiche de vote (refonte A) : une carte blanche dans la bande grise, la
  // pastille de l'état, le mot en titre, la phrase à l'encre.
  if (compact) {
    return (
      <div className={`rounded-[16px] bg-canvas px-4 py-3.5 md:p-5 ${className}`}>
        <p className="text-[13px] font-semibold text-ink-soft">Ce texte est-il devenu loi ?</p>
        <p
          className={`my-1.5 flex items-center gap-2.5 font-heading text-2xl leading-7 font-[560] tracking-[-0.01em] ${
            kind === 'in_shuttle' ? 'text-ink' : 'text-yes'
          }`}
        >
          <span className={`flex size-7 shrink-0 items-center justify-center rounded-full ${LAW_STATE_TONES[kind]}`}>
            <LawStateIcon kind={kind} className="size-4" />
          </span>
          {ANSWER_WORDS[answer.kind]}
        </p>
        <p className="text-sm leading-[21px] text-ink md:leading-relaxed">
          {answer.sentence}
          {isLaw && lawState.legifrance_url && (
            <>
              {' '}
              <a href={lawState.legifrance_url} target="_blank" rel="noreferrer" className="font-semibold link">
                Légifrance ↗
              </a>
            </>
          )}
        </p>
      </div>
    )
  }
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
        className="my-1 flex items-center gap-2 font-heading text-[26px] leading-[30px] font-[560] tracking-[-0.01em] sm:text-[28px] sm:leading-8"
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
