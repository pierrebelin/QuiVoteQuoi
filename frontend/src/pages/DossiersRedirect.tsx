import { Navigate, useSearchParams } from 'react-router'

/** Filtres de l'ancienne liste des textes, repris tels quels par la vue Par texte. */
const KEPT = ['search', 'outcome', 'initiative', 'theme']

/**
 * La liste des textes a fusionné dans l'onglet Votes, vue Par texte : une
 * adresse /dossiers partagée avant la fusion y mène, filtres compris.
 */
export default function DossiersRedirect() {
  const [params] = useSearchParams()
  const target = new URLSearchParams({ vue: 'texte' })
  for (const key of KEPT) {
    const value = params.get(key)
    if (value) target.set(key, value)
  }
  return <Navigate to={`/scrutins?${target}`} replace />
}
