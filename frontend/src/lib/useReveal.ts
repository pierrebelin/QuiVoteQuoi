import { useEffect, useRef } from 'react'

/**
 * Apparition au défilement : l'élément, d'abord estompé, monte en place quand
 * il entre dans l'écran (styles `[data-reveal]`, index.css).
 *
 * Le contenu est visible par défaut : il n'est estompé que s'il est encore
 * sous le bas de l'écran au montage, que le navigateur sait l'observer, et
 * que le visiteur n'a pas demandé moins d'animations. Un élément déjà à
 * l'écran ne clignote donc jamais, et rien ne reste caché si le script échoue.
 */
export function useReveal<T extends HTMLElement>() {
  const ref = useRef<T>(null)

  useEffect(() => {
    const element = ref.current
    if (!element || typeof IntersectionObserver === 'undefined') return
    if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return
    if (element.getBoundingClientRect().top < window.innerHeight) return

    element.dataset.reveal = 'hidden'
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return
        element.dataset.reveal = 'shown'
        observer.disconnect()
      },
      { rootMargin: '0px 0px -8% 0px' },
    )
    observer.observe(element)
    return () => {
      observer.disconnect()
      delete element.dataset.reveal
    }
  }, [])

  return ref
}
