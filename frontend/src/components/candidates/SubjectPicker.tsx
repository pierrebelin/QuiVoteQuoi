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
        className={`mb-3 flex min-h-16 w-full items-center gap-3 rounded-2xl bg-surface px-4 py-2.5 text-left transition-colors hover:bg-surface-soft ${FOCUS}`}
      >
        <FamilyIllustration bare code={current.code} className="size-10 shrink-0 rounded-xl bg-canvas" glyphClassName="size-[22px]" />
        <span className="min-w-0 flex-1">
          <span className="block text-[13px] leading-4 text-ink-faint">Sujet</span>
          <span className="block text-base leading-5 font-[620] text-ink">{current.label}</span>
        </span>
        <span className="flex items-center gap-1 text-sm font-semibold text-accent">
          Changer
          <Icon name="down" className="size-4" strokeWidth={2} />
        </span>
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
                  className={`grid min-h-[52px] w-full grid-cols-[28px_minmax(0,1fr)_18px] items-center gap-3 rounded-xl px-2.5 py-1.5 text-left text-[15px] leading-tight transition-colors ${FOCUS} ${
                    on ? 'bg-accent-soft font-semibold text-accent-ink' : 'font-medium text-ink hover:bg-surface'
                  }`}
                >
                  <FamilyIllustration bare code={family.code} className="size-7" glyphClassName="size-[22px]" />
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
      <div className="px-2.5">
        <SectionTitle>Sujet</SectionTitle>
      </div>
      <nav aria-label="Sujets">
        <ul className="flex flex-col gap-0.5">
          {families.map((family) => {
            const on = family.code === current
            return (
              <li key={family.code}>
                <button
                  type="button"
                  aria-pressed={on}
                  onClick={() => onPick(family.code)}
                  className={`flex min-h-11 w-full items-center gap-2.5 rounded-xl px-2.5 py-1.5 text-left text-sm leading-tight transition-colors ${FOCUS} ${
                    on ? 'bg-surface font-[620] text-ink' : 'font-medium text-ink-soft hover:bg-surface hover:text-ink'
                  }`}
                >
                  <FamilyIllustration bare code={family.code} className="size-5 shrink-0" glyphClassName="size-5" />
                  <span className="min-w-0 flex-1">{family.label}</span>
                  {on && <Icon name="chevron" className="size-4 shrink-0" strokeWidth={2} />}
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
    <section className="mt-8 px-2.5">
      <SectionTitle>{`Face-à-face · ${selected.length} sur ${max}`}</SectionTitle>
      <div className="flex flex-col gap-3">
        {selected.length === 0 ? (
          <p className="text-[13px] leading-snug text-ink-soft">
            Dépliez une candidature et ajoutez-la pour lire ses extraits à côté d’autres.
          </p>
        ) : (
          <ul>
            {selected.map((candidate) => (
              <li key={candidate.id} className="flex min-h-12 items-center gap-2.5 border-t border-line">
                <CandidatePortrait candidate={candidate} className="size-7" />
                <span className="min-w-0 flex-1 truncate text-sm font-semibold text-ink">{candidate.display_name}</span>
                <button
                  type="button"
                  onClick={() => onRemove(candidate.id)}
                  aria-label={`Retirer du face-à-face : ${candidate.display_name}`}
                  title="Retirer du face-à-face"
                  className={`-mr-2.5 flex size-11 shrink-0 items-center justify-center rounded-xl text-ink-faint transition-colors hover:text-ink ${FOCUS}`}
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
          className={`flex h-12 items-center justify-center rounded-[14px] bg-ink text-[15px] font-semibold text-surface transition-colors hover:bg-ink/90 ${FOCUS}`}
        >
          Comparer
        </button>
      </div>
    </section>
  )
}
