import { useState } from 'react'
import { normalizeText, organizationLabel } from '../../lib/candidates'
import type { CandidateDto } from '../../types/candidates'
import { BottomSheet } from '../BottomSheet'
import { Icon } from '../icons'
import { CandidatePortrait } from './CandidateIdentity'

const FOCUS =
  'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

/**
 * Cases de la comparaison : deux sous 768 px, quatre au-delà (le parent fixe le
 * nombre). Toucher une case ouvre le tiroir de choix ; choisir remplace la
 * candidature de la case. Une case vide invite à en choisir une.
 *
 * Le parent possède la sélection (l'URL) : ce composant ne garde que l'état du
 * tiroir, donc rien ici ne déclenche de chargement.
 */
export function CandidateSlots({
  candidates,
  slots,
  onPick,
  onClear,
}: {
  /** Toutes les candidatures référencées, triées par nom de famille. */
  candidates: CandidateDto[]
  /** Une entrée par case : la candidature choisie, ou `null`. */
  slots: (CandidateDto | null)[]
  onPick: (slot: number, candidateId: string) => void
  onClear: (slot: number) => void
}) {
  const [openSlot, setOpenSlot] = useState<number | null>(null)
  const current = openSlot === null ? null : (slots[openSlot] ?? null)
  const taken = new Set(
    slots.flatMap((candidate, index) => (candidate && index !== openSlot ? [candidate.id] : [])),
  )

  return (
    <>
      <ul className="grid grid-cols-2 gap-2 md:grid-cols-4" aria-label="Candidatures comparées">
        {slots.map((candidate, index) => (
          <li key={index} className="min-w-0">
            {candidate ? (
              <div className="flex items-stretch rounded-xl bg-surface shadow-card ring-[1.5px] ring-ink ring-inset">
                <button
                  type="button"
                  aria-haspopup="dialog"
                  aria-label={`Candidature ${index + 1} : ${candidate.display_name}. Changer`}
                  onClick={() => setOpenSlot(index)}
                  className={`grid min-h-[52px] min-w-0 flex-1 grid-cols-[34px_minmax(0,1fr)_16px] items-center gap-2 rounded-xl py-1.5 pr-2 pl-2 text-left transition-colors hover:bg-surface-soft ${FOCUS}`}
                >
                  <CandidatePortrait candidate={candidate} className="size-[34px]" />
                  <span className="text-[13.5px] leading-[1.15] font-semibold [overflow-wrap:anywhere] text-ink">
                    {candidate.display_name}
                  </span>
                  <Icon name="down" className="size-4 text-ink-soft" />
                </button>
                <button
                  type="button"
                  onClick={() => onClear(index)}
                  aria-label={`Vider la case : ${candidate.display_name}`}
                  title="Vider la case"
                  className={`hidden size-11 shrink-0 items-center justify-center self-center rounded-xl text-ink-soft transition-colors hover:bg-surface-soft hover:text-ink md:flex ${FOCUS}`}
                >
                  <Icon name="close" className="size-4" />
                </button>
              </div>
            ) : (
              <button
                type="button"
                aria-haspopup="dialog"
                aria-label={`Candidature ${index + 1} : choisir une candidature`}
                onClick={() => setOpenSlot(index)}
                className={`flex min-h-[52px] w-full items-center gap-2 rounded-xl border-[1.5px] border-dashed border-line-strong bg-surface px-3 py-1.5 text-left text-[13.5px] leading-[1.15] font-semibold text-accent transition-colors hover:bg-surface-soft ${FOCUS}`}
              >
                <Icon name="add" className="size-4" />
                Choisir une candidature
              </button>
            )}
          </li>
        ))}
      </ul>

      <BottomSheet
        open={openSlot !== null}
        onClose={() => setOpenSlot(null)}
        title="Choisir une candidature"
      >
        <CandidatePicker
          candidates={candidates}
          current={current}
          taken={taken}
          onPick={(candidateId) => {
            if (openSlot !== null) onPick(openSlot, candidateId)
            setOpenSlot(null)
          }}
          onClear={() => {
            if (openSlot !== null) onClear(openSlot)
            setOpenSlot(null)
          }}
        />
      </BottomSheet>
    </>
  )
}

/**
 * Contenu du tiroir : recherche insensible aux accents, puis toutes les
 * candidatures. Celles qui occupent déjà une autre case sont désactivées, et le
 * disent. Monté à l'ouverture seulement : la recherche repart vide à chaque fois.
 */
function CandidatePicker({
  candidates,
  current,
  taken,
  onPick,
  onClear,
}: {
  candidates: CandidateDto[]
  current: CandidateDto | null
  taken: Set<string>
  onPick: (candidateId: string) => void
  onClear: () => void
}) {
  const [search, setSearch] = useState('')
  const query = normalizeText(search.trim())
  const shown = query
    ? candidates.filter(
        (candidate) =>
          normalizeText(candidate.display_name).includes(query) ||
          normalizeText(organizationLabel(candidate)).includes(query),
      )
    : candidates

  return (
    <>
      <label className="mx-3.5 mb-2 flex h-11 shrink-0 items-center gap-2 rounded-xl bg-surface-soft px-3 text-ink-faint">
        <Icon name="search" />
        <input
          type="search"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          placeholder="Rechercher un nom"
          aria-label="Rechercher une candidature"
          autoComplete="off"
          className="min-w-0 flex-1 bg-transparent text-base text-ink outline-none placeholder:text-ink-faint"
        />
      </label>

      <ul className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-1.5 pb-2">
        {shown.map((candidate) => {
          const isCurrent = candidate.id === current?.id
          const isTaken = taken.has(candidate.id)
          return (
            <li key={candidate.id}>
              <button
                type="button"
                disabled={isTaken}
                aria-pressed={isCurrent}
                onClick={() => onPick(candidate.id)}
                className={`grid min-h-[54px] w-full grid-cols-[40px_minmax(0,1fr)_26px] items-center gap-2.5 rounded-xl px-2 py-1.5 text-left transition-colors hover:bg-surface-soft disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:bg-transparent ${FOCUS} ${
                  isCurrent ? 'bg-surface-soft' : ''
                }`}
              >
                <CandidatePortrait candidate={candidate} className="size-10" />
                <span className="min-w-0">
                  <span className="block text-[14.5px] leading-tight font-semibold text-ink">
                    {candidate.display_name}
                  </span>
                  <span className="block truncate text-[12.5px] leading-tight text-ink-soft">
                    {isTaken ? 'Déjà choisie dans une autre case' : organizationLabel(candidate)}
                  </span>
                </span>
                <span
                  aria-hidden
                  className={`flex size-[26px] items-center justify-center rounded-full ${
                    isCurrent ? 'bg-ink text-surface' : 'ring-[1.5px] ring-line-strong ring-inset'
                  }`}
                >
                  {isCurrent && <Icon name="check" className="size-[15px]" strokeWidth={3} />}
                </span>
              </button>
            </li>
          )
        })}
        {shown.length === 0 && (
          <li className="px-3 py-6 text-center text-sm text-ink-soft">
            Aucune candidature ne correspond à cette recherche.
          </li>
        )}
      </ul>

      {current && (
        <div className="shrink-0 border-t border-line px-3.5 py-2.5">
          <button
            type="button"
            onClick={onClear}
            className={`flex min-h-11 w-full items-center justify-center rounded-xl border border-line-strong bg-surface px-4 text-sm font-semibold text-ink transition-colors hover:bg-surface-soft ${FOCUS}`}
          >
            Vider cette case
          </button>
        </div>
      )}
    </>
  )
}
