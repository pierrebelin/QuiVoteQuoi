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
 * il vaut au plus jusqu'à minuit UTC, et l'écran n'est joignable que par le
 * tunnel SSH. Passé cette échéance, l'API répond 401 et il faut le recoller.
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

  if (response.status === 401) {
    throw new ApiError(
      401,
      "Jeton absent, faux ou expiré. Le jeton change chaque jour : en récupérer un neuf avec `deploy/bin/admin-token.sh`, puis le recoller.",
    )
  }
  if (response.status === 403) {
    throw new ApiError(
      403,
      "Écriture fermée côté serveur : ADMIN_TOKEN_SECRET n'est pas configuré.",
    )
  }
  if (!response.ok) {
    throw new ApiError(response.status, `HTTP ${response.status}`)
  }

  return response
}
