import { useEffect, useState } from 'react'

/** Durée du défilement, alignée sur le remplissage de l'hémicycle. */
const DUREE_MS = 1400

const animationsReduites = () =>
  typeof window !== 'undefined' &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches

/** Départ rapide, arrivée douce : le nombre se pose sur sa valeur. */
const ralenti = (t: number) => 1 - (1 - t) ** 4

/**
 * Nombre qui défile de zéro à sa valeur (identité C1, chantier 4).
 *
 * Le défilement n'est qu'un dessin : la valeur publiée est la seule lue par
 * les lecteurs d'écran, et la seule affichée quand l'utilisateur a demandé
 * moins d'animations. Aucun chiffre intermédiaire n'est donc jamais donné
 * pour un résultat.
 */
export default function CountUp({ value }: { value: number }) {
  const [affiche, setAffiche] = useState(() => (animationsReduites() ? value : 0))

  useEffect(() => {
    if (animationsReduites()) {
      setAffiche(value)
      return
    }
    let image = 0
    const debut = performance.now()
    const avancer = (maintenant: number) => {
      const t = Math.min(1, (maintenant - debut) / DUREE_MS)
      setAffiche(Math.round(value * ralenti(t)))
      if (t < 1) image = requestAnimationFrame(avancer)
    }
    image = requestAnimationFrame(avancer)
    // Sans image à dessiner (onglet en arrière-plan, rendu suspendu), le
    // défilement s'arrête : la valeur publiée s'affiche quand même à l'heure.
    const garde = setTimeout(() => setAffiche(value), DUREE_MS + 100)
    return () => {
      cancelAnimationFrame(image)
      clearTimeout(garde)
    }
  }, [value])

  return (
    <>
      <span aria-hidden className="tabular-nums">
        {affiche}
      </span>
      <span className="sr-only">{value}</span>
    </>
  )
}
