import { LOGO_TRACES, LOGO_VUE } from './logoTraces'

type LogoProps = {
  className?: string
}

/*
 * L'hémicycle tricolore.
 *
 * Les trois rayons centraux sont en crème (`#ded3bd`) et non dans le blanc
 * cassé du fichier source (`#efeade`) : ce dernier est fait pour un fond
 * sombre et s'efface sur le blanc de l'en-tête. Le crème est celui de la
 * pastille du favicon, donc une couleur déjà tenue par l'identité.
 *
 * Inliné plutôt que servi en fichier : neuf tracés pèsent moins qu'une requête,
 * et l'en-tête ne s'affiche pas un instant sans son logo.
 *
 * Les tracés vivent dans `logoTraces.ts`, partagés avec l'image d'un scrutin.
 */

/** Les rayons bleu et rouge suivent le thème ; la crème tient sur les deux. */
const FILL_THEMABLE: Record<string, string> = {
  '#16233f': 'var(--color-logo-blue)',
  '#9e3038': 'var(--color-logo-red)',
}

export default function Logo({ className }: LogoProps) {
  return (
    <svg
      viewBox={`0 0 ${LOGO_VUE.largeur} ${LOGO_VUE.hauteur}`}
      role="img"
      aria-label="QuiVoteQuoi"
      className={className}
    >
      {LOGO_TRACES.map(([d, fill]) => (
        <path key={d} d={d} fill={FILL_THEMABLE[fill] ?? fill} />
      ))}
    </svg>
  )
}
