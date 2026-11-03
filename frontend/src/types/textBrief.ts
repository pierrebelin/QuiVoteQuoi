/** `GET /api/dossiers/{uid}/en-bref` : « En bref » d'un dossier (CU-03). */

export interface TextVersionDto {
  document_uid: string
  /** Nature et chambre, écrites par le serveur : « Texte adopté par l'Assemblée nationale ». */
  label: string
  /** `an` : l'Assemblée nationale ; `sn` : le Sénat. */
  chamber: 'an' | 'sn'
  deposited_on: string
  /** `null` pour une version que l'Assemblée ne publie pas. */
  official_url: string | null
  /** Le texte de la version est publié par l'Assemblée et lisible. */
  available: boolean
}

/** `ready` : la ligne ; `pending` : à venir ; `unavailable` : refusée ou jamais produite. */
export type ArticleBriefState = 'ready' | 'pending' | 'unavailable'

export interface ArticleBriefDto {
  /** Numéro tel que l'en-tête l'écrit : `1er`, `19 bis`. */
  label: string
  /** Chemin de la division, du plus haut niveau au plus bas, séparés par « > ». */
  division: string | null
  state: ArticleBriefState
  /** Présente seulement quand l'état est `ready`. */
  line: string | null
}

/** Seule une version `read` porte des articles. */
export type TextStatus = 'unread' | 'read' | 'missing' | 'senate_only'

export interface DossierBriefResponse {
  label: 'Synthèse automatique'
  /** De la plus récente à la plus ancienne. */
  versions: TextVersionDto[]
  /** `null` : le dossier n'a aucune version dont l'Assemblée publie le texte. */
  shown: TextVersionDto | null
  text_status: TextStatus | null
  /** La version montrée est la dernière disponible. */
  is_latest: boolean
  /** Version plus récente que celle qui est montrée, dont le texte n'est pas publié. */
  newer_unavailable_version: TextVersionDto | null
  articles: ArticleBriefDto[]
  models: string[]
  generated_at: string | null
}
