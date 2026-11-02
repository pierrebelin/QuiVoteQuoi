import { useLayoutEffect, useState } from 'react'

/**
 * Hauteur de l'en-tête collant du site. Sur petit écran, titre et navigation
 * s'empilent sur un nombre de lignes qui dépend de la largeur : un décalage en
 * dur masquerait la barre des votes sous l'en-tête, ou la laisserait flotter
 * loin de lui.
 */
export function useHeaderHeight(): number {
  const [height, setHeight] = useState(0)

  useLayoutEffect(() => {
    const header = document.querySelector('header')
    if (!header) return
    const measure = () => setHeight(Math.round(header.getBoundingClientRect().height))
    measure()
    const observer = new ResizeObserver(measure)
    observer.observe(header)
    return () => observer.disconnect()
  }, [])

  return height
}
