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
 */
export default function Logo({ className }: LogoProps) {
  return (
    <svg
      viewBox="0 0 200 116"
      role="img"
      aria-label="QuiVoteQuoi"
      className={className}
    >
      <path
        d="M 50.0 107.8 A 50 50 0 0 1 52.3 95.0 L 77.1 102.8 A 24 24 0 0 0 76.0 109.0 Z"
        fill="#16233f"
      />
      <path
        d="M 37.2 84.0 A 68 68 0 0 1 46.1 68.6 L 81.0 95.4 A 24 24 0 0 0 77.8 100.8 Z"
        fill="#16233f"
      />
      <path
        d="M 36.6 51.9 A 86 86 0 0 1 53.8 37.5 L 87.1 89.8 A 24 24 0 0 0 82.3 93.8 Z"
        fill="#16233f"
      />
      <path
        d="M 53.8 21.3 A 100 100 0 0 1 78.4 12.4 L 94.8 86.6 A 24 24 0 0 0 88.9 88.7 Z"
        fill="#ded3bd"
      />
      <path
        d="M 86.9 10.9 A 100 100 0 0 1 113.1 10.9 L 103.1 86.2 A 24 24 0 0 0 96.9 86.2 Z"
        fill="#ded3bd"
      />
      <path
        d="M 121.6 12.4 A 100 100 0 0 1 146.2 21.3 L 111.1 88.7 A 24 24 0 0 0 105.2 86.6 Z"
        fill="#ded3bd"
      />
      <path
        d="M 146.2 37.5 A 86 86 0 0 1 163.4 51.9 L 117.7 93.8 A 24 24 0 0 0 112.9 89.8 Z"
        fill="#9e3038"
      />
      <path
        d="M 153.9 68.6 A 68 68 0 0 1 162.8 84.0 L 122.2 100.8 A 24 24 0 0 0 119.0 95.4 Z"
        fill="#9e3038"
      />
      <path
        d="M 147.7 95.0 A 50 50 0 0 1 150.0 107.8 L 124.0 109.0 A 24 24 0 0 0 122.9 102.8 Z"
        fill="#9e3038"
      />
    </svg>
  )
}
