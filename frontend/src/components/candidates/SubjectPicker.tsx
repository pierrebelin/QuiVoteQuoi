import { useState } from 'react'
import type { CandidateDto } from '../../types/candidates'
import type { FamilyDto } from '../../types/themes'
import { BottomSheet } from '../BottomSheet'
import { FamilyIllustration } from '../FamilyIllustration'
import { Icon } from '../icons'
import { SectionTitle } from '../ui'
import { CandidatePortrait } from './CandidateIdentity'

const FOCUS =
  'focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-accent'

/** Téléphone : le sujet en cours sur un bouton, les seize dans un tiroir. */
export function SubjectButton({
  families,
  current,
  onPick,
}: {
  families: FamilyDto[]
  current: FamilyDto
  onPick: (code: string) => void
}) {
  const [open, setOpen] = useState(false)
  return (
    <>
      <button
        type="button"
        aria-haspopup="dialog"
        onClick={() => setOpen(true)}
        className={`mb-3 flex w-full items-center gap-2.5 rounded border border-line bg-surface py-2 pr-3.5 pl-2 text-left shadow-card transition-colors hover:bg-surface-soft ${FOCUS}`}
      >
        <FamilyIllustration code={current.code} className="size-[38px] rounded" glyphClassName="size-[22px]" />
        <span className="min-w-0 flex-1">
          <span className="block text-xs leading-[15px] text-ink-soft">Sujet</span>
          <span className="block text-[15px] leading-[19px] font-semibold text-ink">{current.label}</span>
        </span>
        <span className="text-[13px] font-semibold text-accent">Changer</span>
      </button>
      <BottomSheet open={open} onClose={() => setOpen(false)} title="Choisir un sujet">
        <ul className="max-h-[70dvh] overflow-y-auto px-2 pb-3">
          {families.map((family) => {
            const on = family.code === current.code
            return (
              <li key={family.code}>
                <button
                  type="button"
                  aria-pressed={on}
                  onClick={() => {
                    onPick(family.code)
                    setOpen(false)
                  }}
                  className={`grid min-h-[52px] w-full grid-cols-[34px_minmax(0,1fr)_18px] items-center gap-2.5 rounded px-2 py-1.5 text-left text-[14.5px] leading-tight transition-colors hover:bg-surface-soft ${FOCUS} ${
                    on ? 'bg-accent-soft font-semibold text-accent' : 'font-medium text-ink'
                  }`}
                >
                  <FamilyIllustration code={family.code} className="size-[34px] rounded" glyphClassName="size-5" />
                  <span>{family.label}</span>
                  {on && <Icon name="check" className="size-[18px]" strokeWidth={2.5} />}
                </button>
              </li>
            )
          })}
        </ul>
      </BottomSheet>
    </>
  )
}

/** Ordinateur : les seize sujets en colonne, toujours visibles. */
export function SubjectList({
  families,
  current,
  onPick,
}: {
  families: FamilyDto[]
  current: string
  onPick: (code: string) => void
}) {
  return (
    <div>
      <SectionTitle>Sujet</SectionTitle>
      <nav aria-label="Sujets" className="rounded border border-line bg-surface p-1.5 shadow-card">
        <ul className="flex flex-col gap-0.5">
          {families.map((family) => {
            const on = family.code === current
            return (
              <li key={family.code}>
                <button
                  type="button"
                  aria-pressed={on}
                  onClick={() => onPick(family.code)}
                  className={`flex min-h-[38px] w-full items-center justify-between gap-2 rounded px-3 py-1.5 text-left text-sm leading-tight transition-colors ${FOCUS} ${
                    on ? 'bg-accent-soft font-semibold text-accent' : 'text-ink hover:bg-surface-soft'
                  }`}
                >
                  {family.label}
                  {on && <Icon name="chevron" className="size-4 shrink-0" />}
                </button>
              </li>
            )
          })}
        </ul>
      </nav>
    </div>
  )
}

/** Ordinateur : l'encart du face-à-face sous les sujets, vers l'onglet 2. */
export function FaceToFaceBox({
  selected,
  max,
  onRemove,
  onCompare,
}: {
  selected: CandidateDto[]
  max: number
  onRemove: (candidateId: string) => void
  onCompare: () => void
}) {
  return (
    <section className="mt-5">
      <SectionTitle>{`Face-à-face · ${selected.length} sur ${max}`}</SectionTitle>
      <div className="flex flex-col gap-3 rounded border border-line bg-surface p-3 shadow-card">
        {selected.length === 0 ? (
          <p className="text-[13px] leading-snug text-ink-soft">
            Dépliez une candidature et ajoutez-la pour lire ses extraits à côté d’autres.
          </p>
        ) : (
          <ul className="flex flex-col gap-2">
            {selected.map((candidate) => (
              <li key={candidate.id} className="flex items-center gap-2.5">
                <CandidatePortrait candidate={candidate} className="size-7" />
                <span className="min-w-0 flex-1 truncate text-sm font-medium text-ink">{candidate.display_name}</span>
                <button
                  type="button"
                  onClick={() => onRemove(candidate.id)}
                  aria-label={`Retirer du face-à-face : ${candidate.display_name}`}
                  title="Retirer du face-à-face"
                  className={`flex size-8 shrink-0 items-center justify-center rounded text-ink-soft transition-colors hover:bg-surface-soft hover:text-ink ${FOCUS}`}
                >
                  <Icon name="close" className="size-4" />
                </button>
              </li>
            ))}
          </ul>
        )}
        <button
          type="button"
          onClick={onCompare}
          className={`flex h-10 items-center justify-center rounded bg-accent text-[13.5px] font-semibold text-surface transition-colors hover:bg-accent-strong ${FOCUS}`}
        >
          Comparer
        </button>
      </div>
    </section>
  )
}
