import { useSyncExternalStore } from 'react'

/** Seuil `lg` de Tailwind. */
const LARGE = '(min-width: 64rem)'

function subscribe(onChange: () => void) {
  const query = window.matchMedia(LARGE)
  query.addEventListener('change', onChange)
  return () => query.removeEventListener('change', onChange)
}

/** Vrai à partir de 1024 px, et suit le redimensionnement de la fenêtre. */
export function useLargeScreen(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => window.matchMedia(LARGE).matches,
    () => false,
  )
}
