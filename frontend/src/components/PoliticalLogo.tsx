import type { Logo } from '../content/logos'

/**
 * Logo officiel d'un groupe ou d'un parti, posé sur une plaque blanche : les
 * fichiers sont dessinés pour un fond clair, et la plaque le leur garde en mode
 * sombre. Sans logo référencé, le sigle tient lieu de marque, à la couleur
 * fournie — même emplacement, même taille, aucun groupe sans identité visuelle.
 */
export function PoliticalLogo({
  logo,
  fallback,
  color,
  className = 'h-12 w-20',
}: {
  logo: Logo | undefined
  fallback: string
  color: string | null
  className?: string
}) {
  return (
    <span
      aria-hidden
      className={`flex shrink-0 items-center justify-center overflow-hidden rounded-lg bg-white p-1 ring-1 ring-inset ring-black/10 ${className}`}
    >
      {logo ? (
        <img src={logo.src} alt="" loading="lazy" className="max-h-full max-w-full object-contain" />
      ) : (
        <span className="truncate text-[11px] font-bold tracking-tight" style={{ color: color ?? '#52525b' }}>
          {fallback}
        </span>
      )}
    </span>
  )
}
