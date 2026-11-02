import type { AmendmentDto } from '../types/amendments'

/**
 * Un signataire nommé, avec le groupe qu'il avait au dépôt, aux couleurs
 * publiées de ce groupe (RM-10). Sans couleur publiée, le groupe reste en texte.
 */
export default function AmendmentAuthor({ amendment }: { amendment: AmendmentDto }) {
  const name = amendment.author_name ?? amendment.author_actor_uid ?? 'Auteur non identifié'

  return (
    <span className="text-ink-soft">
      {amendment.author_official_url ? (
        <a
          href={amendment.author_official_url}
          target="_blank"
          rel="noreferrer"
          className="font-medium text-ink hover:underline"
        >
          {name}
        </a>
      ) : (
        <span className="font-medium text-ink">{name}</span>
      )}

      {amendment.author_group_abbrev && (
        <span
          className={
            amendment.author_group_color
              ? 'ml-1.5 rounded px-1.5 py-0.5 text-xs font-semibold'
              : 'ml-1.5'
          }
          style={
            amendment.author_group_color
              ? {
                  backgroundColor: amendment.author_group_color,
                  color: amendment.author_text_tone === 'white' ? '#ffffff' : '#000000',
                }
              : undefined
          }
          title={
            amendment.author_group_origin === 'resolved_at_deposit'
              ? `${amendment.author_group_label} — groupe du signataire à la date de dépôt, reconstitué depuis les appartenances datées`
              : `${amendment.author_group_label} — groupe publié par la source sur cet amendement`
          }
        >
          {amendment.author_group_color
            ? amendment.author_group_abbrev
            : `(${amendment.author_group_abbrev})`}
        </span>
      )}

      {amendment.author_group_ambiguous && (
        <span
          className="ml-1.5 text-ink-faint"
          title="Deux groupes concurrents revendiquent ce signataire à la date de dépôt : aucun n'est affiché."
        >
          (groupe indéterminé)
        </span>
      )}

      {amendment.cosignatory_count > 0 && (
        <span className="text-ink-faint">
          {' '}
          et {amendment.cosignatory_count} cosignataire
          {amendment.cosignatory_count > 1 ? 's' : ''}
        </span>
      )}
    </span>
  )
}
