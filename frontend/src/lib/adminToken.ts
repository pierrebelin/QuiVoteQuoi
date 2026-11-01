/**
 * Jeton d'administration, côté navigateur.
 *
 * Le serveur n'accepte pas un jeton fixe : il dérive celui du jour à partir
 * d'un secret qui, lui, ne quitte jamais le VPS. L'opérateur récupère le jeton
 * du jour par SSH puis le colle ici :
 *
 *   ssh hemicycle@<IP_DU_VPS> '~/app/deploy/bin/admin-token.sh'
 *
 * Il est conservé en `localStorage` — le stocker n'est pas un risque nouveau :
 * il vaut au plus jusqu'à minuit UTC, et sur le site public il ne suffit pas
 * seul : nginx exige d'abord l'identifiant HTTP Basic de l'opérateur. Passé
 * minuit, l'API répond 401 et il faut le recoller.
 */

const TOKEN_KEY = 'quivotequoi.adminToken'

export function getAdminToken(): string {
  return localStorage.getItem(TOKEN_KEY) ?? ''
}

export function setAdminToken(token: string): void {
  const trimmed = token.trim()
  if (trimmed === '') {
    localStorage.removeItem(TOKEN_KEY)
    return
  }
  localStorage.setItem(TOKEN_KEY, trimmed)
}

/** Erreur portant le code HTTP, pour distinguer un refus d'un incident. */
export class ApiError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.name = 'ApiError'
    this.status = status
  }
}

/**
 * `fetch` des routes d'écriture : ajoute le jeton du jour et traduit les refus
 * en message lisible, plutôt qu'en « HTTP 401 » opaque.
 */
export async function adminFetch(
  input: string,
  init: RequestInit = {},
): Promise<Response> {
  const headers = new Headers(init.headers)
  headers.set('x-admin-token', getAdminToken())

  const response = await fetch(input, { ...init, headers })

  // Deux couches peuvent refuser : nginx (identifiant HTTP Basic, ou filtre
  // d'écriture d'une ancienne configuration) et le backend (jeton du jour).
  // Seul nginx annonce `WWW-Authenticate: Basic`, et seul le backend cite
  // ADMIN_TOKEN_SECRET dans son 403 : imputer le refus à la mauvaise couche
  // envoie l'opérateur chercher sur le serveur une panne qui n'y est pas.
  if (response.status === 401) {
    if (response.headers.get('www-authenticate')?.startsWith('Basic')) {
      throw new ApiError(
        401,
        "Identifiant d'administration refusé ou saisie annulée. Recharger la page puis relancer l'action pour qu'il soit redemandé.",
      )
    }
    throw new ApiError(
      401,
      "Jeton absent, faux ou expiré. Le jeton change chaque jour : en récupérer un neuf avec `deploy/bin/admin-token.sh`, puis le recoller.",
    )
  }
  if (response.status === 403) {
    const body = await response.text()
    throw new ApiError(
      403,
      body.includes('ADMIN_TOKEN_SECRET')
        ? "Écriture fermée côté serveur : ADMIN_TOKEN_SECRET n'est pas configuré."
        : "Écriture refusée par le serveur web avant l'application. Passer par le tunnel SSH d'administration.",
    )
  }
  if (!response.ok) {
    throw new ApiError(response.status, `HTTP ${response.status}`)
  }

  return response
}
