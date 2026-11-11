import type { CitedAmendmentDto } from '../types/scrutins'

/**
 * Signataire de l'amendement, avec son groupe au dépôt aux couleurs que ce
 * groupe publie (RM-10). Sans couleur publiée, le sigle reste en texte.
 */
export default function CitedAuthor({ cited, compact = false }: { cited: CitedAmendmentDto; compact?: boolean }) {
  const abbrev = cited.author_group_abbrev
  return (
    <span className="inline-flex min-w-0 flex-wrap items-center gap-x-1.5 gap-y-0.5 text-[13px] text-ink-soft">
      {abbrev &&
        (cited.author_group_color ? (
          <span
            className="rounded px-1.5 text-[11px] leading-[18px] font-bold tracking-wide ring-1 ring-inset ring-badge-edge"
            style={{
              backgroundColor: cited.author_group_color,
              color: cited.author_text_tone === 'white' ? '#ffffff' : '#000000',
            }}
          >
            {abbrev}
          </span>
        ) : (
          <span className="rounded bg-surface-soft px-1.5 text-[11px] leading-[18px] font-semibold ring-1 ring-line ring-inset">
            {abbrev}
          </span>
        ))}
      <span className="text-ink">{cited.author_name ?? 'Auteur non identifié'}</span>
      {cited.cosignatory_count > 0 && (
        <span className="text-ink-faint">
          {compact
            ? `+${cited.cosignatory_count}`
            : `et ${cited.cosignatory_count} cosignataire${cited.cosignatory_count > 1 ? 's' : ''}`}
        </span>
      )}
    </span>
  )
}
