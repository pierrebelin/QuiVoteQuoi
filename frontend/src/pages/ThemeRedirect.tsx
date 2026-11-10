import { Navigate, useParams } from 'react-router'
import { ENTRY_PATH, themeLink } from '../lib/entryFlow'

/**
 * Les fiches de thème ont fusionné dans le parcours par sujet : une adresse
 * partagée avant la fusion ouvre les votes de ce sujet, et la liste de ses
 * textes s'y déplie.
 */
export default function ThemeRedirect() {
  const { code } = useParams<{ code: string }>()
  return <Navigate to={code ? themeLink(code) : ENTRY_PATH} replace />
}
