import { useEffect, useRef, useState } from 'react'
import {
  RESEAUX,
  aUnExpose,
  chargerPolices,
  dessinerExpose,
  dessinerVote,
  dimensions,
  estDiaporama,
  texteDePartage,
  type ReseauImage,
  type ThemeImage,
  type VisuelImage,
} from '../lib/voteImage'
import type { ScrutinDetailDto } from '../types/scrutins'
import { Button, Card, SegmentedControl } from './ui'

function enBlob(canvas: HTMLCanvasElement) {
  return new Promise<Blob>((resolve, reject) =>
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error('Image vide'))),
      'image/png',
    ),
  )
}

/**
 * Panneau d'export d'un scrutin en image pour les réseaux sociaux.
 *
 * Trois choix, pas davantage : le réseau, qui fixe les dimensions et les
 * marges, le visuel (l'hémicycle ou la répartition par groupe) et le thème. Le contenu, lui, ne
 * se règle pas — retirer les non-votants ou un groupe ferait dire à l'image
 * autre chose que la page (README.md §2).
 *
 * La story et le post Instagram font exception : leur dessin est fixe (le vote
 * sur fond sombre), avec une seconde image, l'exposé, seulement quand il existe.
 *
 * Le partage natif n'est proposé que là où le navigateur sait partager un
 * fichier (mobile surtout) ; ailleurs, on télécharge l'image et on copie le
 * texte qui l'accompagne.
 */
export default function VoteImageExport({
  scrutin,
  onClose,
}: {
  scrutin: ScrutinDetailDto
  onClose: () => void
}) {
  const avecSieges = scrutin.groups.some((g) =>
    g.votes.some((v) => v.seat !== null),
  )

  const [reseau, setReseau] = useState<ReseauImage>('instagram')
  const [visuel, setVisuel] = useState<VisuelImage>(
    avecSieges ? 'hemicycle' : 'groupes',
  )
  const [theme, setTheme] = useState<ThemeImage>(() =>
    document.documentElement.dataset.theme === 'dark' ? 'sombre' : 'clair',
  )
  const [statut, setStatut] = useState<string | null>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const canvasContenuRef = useRef<HTMLCanvasElement>(null)

  const diaporama = estDiaporama(reseau)
  const avecContenu = diaporama && aUnExpose(scrutin)

  const lien = `${window.location.origin}/scrutins/${scrutin.uid}`
  const nomFichier = (numero: number | null) =>
    `quivotequoi-scrutin-${scrutin.number}-${reseau}${numero === null ? '' : `-${numero}`}.png`

  /** Les images à publier, dans l'ordre, chacune avec son nom de fichier. */
  function images() {
    const liste = [canvasRef.current, canvasContenuRef.current].filter(
      (c): c is HTMLCanvasElement => c !== null,
    )
    return liste.map((canvas, i) => ({
      canvas,
      nom: nomFichier(liste.length > 1 ? i + 1 : null),
    }))
  }

  useEffect(() => {
    let annule = false
    chargerPolices().then(() => {
      if (!annule && canvasRef.current) {
        dessinerVote(canvasRef.current, scrutin, { reseau, visuel, theme, lien })
      }
      if (!annule && canvasContenuRef.current) {
        dessinerExpose(canvasContenuRef.current, scrutin, { reseau, lien })
      }
    })
    return () => {
      annule = true
    }
  }, [scrutin, reseau, visuel, theme, lien])

  async function telecharger() {
    const liste = images()
    for (const { canvas, nom } of liste) {
      const url = URL.createObjectURL(await enBlob(canvas))
      const a = document.createElement('a')
      a.href = url
      a.download = nom
      a.click()
      URL.revokeObjectURL(url)
    }
    setStatut(liste.length > 1 ? 'Images téléchargées.' : 'Image téléchargée.')
  }

  const fichierPartageable =
    typeof navigator.canShare === 'function' &&
    navigator.canShare({
      files: [new File([], nomFichier(null), { type: 'image/png' })],
    })

  async function partager() {
    const fichiers = await Promise.all(
      images().map(
        async ({ canvas, nom }) => new File([await enBlob(canvas)], nom, { type: 'image/png' }),
      ),
    )
    try {
      await navigator.share({
        files: fichiers,
        text: texteDePartage(scrutin, lien, reseau),
      })
    } catch (erreur) {
      // Fermer la feuille de partage n'est pas une erreur à signaler.
      if (erreur instanceof DOMException && erreur.name === 'AbortError') return
      setStatut('Le partage a échoué. Téléchargez l’image à la place.')
    }
  }

  async function copierTexte() {
    try {
      await navigator.clipboard.writeText(texteDePartage(scrutin, lien, reseau))
      setStatut('Texte copié : à coller avec l’image.')
    } catch {
      setStatut('Copie refusée par le navigateur.')
    }
  }

  const { largeur, hauteur } = dimensions(reseau)

  return (
    <Card className="mb-5 px-5 py-4">
      <div className="mb-3 flex items-baseline justify-between gap-4">
        <h3 className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
          Partager en image
        </h3>
        <button
          type="button"
          onClick={onClose}
          className="text-xs font-medium link"
        >
          Fermer
        </button>
      </div>

      <div className="grid gap-5 md:grid-cols-[minmax(0,1fr)_16rem]">
        <div className="flex min-w-0 items-center justify-center gap-3 rounded bg-surface-soft p-3">
          <canvas
            ref={canvasRef}
            role="img"
            aria-label={`Aperçu de l'image du scrutin n° ${scrutin.number}`}
            className={`block h-auto max-h-[28rem] w-auto rounded shadow-card ${
              avecContenu ? 'min-w-0 max-w-[48%]' : 'max-w-full'
            }`}
          />
          {avecContenu && (
            <canvas
              ref={canvasContenuRef}
              role="img"
              aria-label={`Aperçu de la seconde image du scrutin n° ${scrutin.number} : l'exposé`}
              className="block h-auto max-h-[28rem] w-auto min-w-0 max-w-[48%] rounded shadow-card"
            />
          )}
        </div>

        <div className="space-y-4">
          {/* Dix réseaux ne tiennent pas dans un contrôle segmenté sur une
              ligne : une grille, même apparence. */}
          <div className="space-y-1.5">
            <span className="text-xs font-medium text-ink-faint">Réseau</span>
            <div
              role="radiogroup"
              aria-label="Réseau"
              className="grid grid-cols-3 gap-0.5 rounded bg-surface-soft p-0.5"
            >
              {Object.entries(RESEAUX).map(([valeur, r]) => (
                <button
                  key={valeur}
                  type="button"
                  role="radio"
                  aria-checked={reseau === valeur}
                  onClick={() => setReseau(valeur as ReseauImage)}
                  className={`truncate rounded px-1.5 py-1 text-xs font-medium transition-colors ${
                    reseau === valeur
                      ? 'bg-surface text-ink shadow-card'
                      : 'text-ink-soft hover:text-ink'
                  }`}
                >
                  {r.libelle}
                </button>
              ))}
            </div>
            <p className="text-[11px] text-ink-faint">
              {largeur} × {hauteur} px · {RESEAUX[reseau].usage}
            </p>
          </div>

          {!diaporama && (
            <>
              <SegmentedControl
                label="Visuel"
                options={[
                  ...(avecSieges ? [{ value: 'hemicycle', label: 'Hémicycle' }] : []),
                  { value: 'groupes', label: 'Groupes' },
                ]}
                value={visuel}
                onChange={(v) => setVisuel(v as VisuelImage)}
              />

              <SegmentedControl
                label="Thème"
                options={[
                  { value: 'clair', label: 'Clair' },
                  { value: 'sombre', label: 'Sombre' },
                ]}
                value={theme}
                onChange={(v) => setTheme(v as ThemeImage)}
              />
            </>
          )}

          {diaporama && !avecContenu && (
            <p className="text-xs text-ink-soft">
              Une seule image : aucun exposé disponible pour ce vote.
            </p>
          )}

          <div className="flex flex-wrap gap-2 border-t border-line pt-4">
            <Button onClick={telecharger}>Télécharger</Button>
            {fichierPartageable && <Button onClick={partager}>Partager…</Button>}
            <Button onClick={copierTexte}>Copier le texte</Button>
          </div>

          <p className="text-[11px] leading-relaxed text-ink-faint" aria-live="polite">
            {statut ??
              'L’image porte la source et l’adresse de cette page. Un intitulé trop long y est tronqué.'}
          </p>
        </div>
      </div>
    </Card>
  )
}
