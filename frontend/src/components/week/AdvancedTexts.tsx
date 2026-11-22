import { Link } from 'react-router'
import { LawStateIcon } from '../LawStateBadge'
import { LAW_STATE_FILTER_LABELS, LAW_STATE_TONES } from '../../lib/lawState'
import { advanceSentence } from '../../lib/week'
import type { LawStateKind } from '../../types/dossiers'
import type { AdvancedText } from '../../types/week'

/** Ancre du lien « Voir » de la carte des chiffres. */
export const ADVANCED_ANCHOR = 'textes-avances'

/** Ordre des groupes (RM-08) : ce qui va s'appliquer d'abord. */
const GROUP_ORDER: LawStateKind[] = ['became_law', 'definitively_adopted', 'in_shuttle']

const texts = (count: number) => `${count} texte${count > 1 ? 's' : ''}`

/** Titre d'un groupe : vert pour les textes qui vont s'appliquer (RM-07). */
function groupHeading(kind: LawStateKind, count: number): string {
  if (kind === 'definitively_adopted') return `${LAW_STATE_FILTER_LABELS[kind]} · promulgation attendue`
  return `${LAW_STATE_FILTER_LABELS[kind]} · ${texts(count)}`
}

function AdvancedRow({ text }: { text: AdvancedText }) {
  const { lead, sense, tail } = advanceSentence(text)
  return (
    <Link
      to={`/dossiers/${text.dossier_uid}`}
      className="group flex items-start gap-3 border-t border-line py-[11px] text-ink hover:text-ink"
    >
      <span
        className={`flex size-7 shrink-0 items-center justify-center rounded-full ${LAW_STATE_TONES[text.group]}`}
      >
        <LawStateIcon kind={text.group} className="size-3.5" />
      </span>
      <span className="min-w-0">
        <span className="block text-[14.5px] leading-5 font-semibold group-hover:underline">{text.title}</span>
        <span className="mt-0.5 block text-[13px] leading-[19px] text-ink-soft">
          {lead}
          <span className="font-semibold whitespace-nowrap text-ink">{sense}</span>
          {tail}
        </span>
      </span>
    </Link>
  )
}

/**
 * Textes qui ont avancé dans la semaine (RM-08) : une décision d'une chambre
 * ou une promulgation, avec ou sans scrutin public, rangés par état au soir
 * du dimanche. Le groupe Devenus loi n'existe que s'il y a eu une
 * promulgation dans la semaine (Q4). À partir de 768 px, chaque groupe se lit
 * en deux colonnes.
 */
export default function AdvancedTexts({ advanced }: { advanced: AdvancedText[] }) {
  const groups = GROUP_ORDER.map((kind) => ({ kind, items: advanced.filter((text) => text.group === kind) })).filter(
    (group) => group.items.length > 0,
  )
  return (
    <section
      id={ADVANCED_ANCHOR}
      aria-labelledby={`${ADVANCED_ANCHOR}-titre`}
      className="scroll-mt-20 rounded-[18px] border border-line bg-surface p-4 md:p-6"
    >
      <h2
        id={`${ADVANCED_ANCHOR}-titre`}
        className="mb-1 font-heading text-[22px] leading-[26px] font-[460] tracking-[-0.01em] md:text-[26px] md:leading-[30px]"
      >
        Textes qui ont avancé
      </h2>
      <p className="mb-3.5 text-[13px] leading-[19px] text-ink-soft">
        Étapes de la semaine à l’Assemblée et au Sénat, avec ou sans scrutin public.
      </p>
      {groups.map(({ kind, items }, index) => {
        const half = Math.ceil(items.length / 2)
        return (
          <div key={kind}>
            <h3
              className={`mb-1 flex items-center gap-1.5 text-xs font-semibold tracking-[0.06em] uppercase ${
                index > 0 ? 'mt-4' : ''
              } ${kind === 'in_shuttle' ? 'text-ink-soft' : 'text-yes'}`}
            >
              <LawStateIcon kind={kind} />
              {groupHeading(kind, items.length)}
            </h3>
            <div className="md:grid md:grid-cols-2 md:gap-x-10">
              {[items.slice(0, half), items.slice(half)].map((column, columnIndex) => (
                <div key={columnIndex}>
                  {column.map((text) => (
                    <AdvancedRow key={text.dossier_uid} text={text} />
                  ))}
                </div>
              ))}
            </div>
          </div>
        )
      })}
    </section>
  )
}
