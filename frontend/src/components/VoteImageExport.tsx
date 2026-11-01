import { useEffect, useRef, useState } from 'react'
import {
  FORMATS,
  chargerPolices,
  dessinerVote,
  texteDePartage,
  type FormatImage,
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
 * Trois choix, pas davantage : le format que le réseau attend, le visuel
 * (l'hémicycle ou la répartition par groupe) et le thème. Le contenu, lui, ne
 * se règle pas — retirer les non-votants ou un groupe ferait dire à l'image
 * autre chose que la page (README.md §2).
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

  const [format, setFormat] = useState<FormatImage>('carre')
  const [visuel, setVisuel] = useState<VisuelImage>(
    avecSieges ? 'hemicycle' : 'groupes',
  )
  const [theme, setTheme] = useState<ThemeImage>(() =>
    document.documentElement.dataset.theme === 'dark' ? 'sombre' : 'clair',
  )
  const [statut, setStatut] = useState<string | null>(null)
  const canvasRef = useRef<HTMLCanvasElement>(null)

  const lien = `${window.location.origin}/scrutins/${scrutin.uid}`
  const nomFichier = `quivotequoi-scrutin-${scrutin.number}-${format}.png`

  useEffect(() => {
    let annule = false
    chargerPolices().then(() => {
      if (!annule && canvasRef.current) {
        dessinerVote(canvasRef.current, scrutin, { format, visuel, theme, lien })
      }
    })
    return () => {
      annule = true
    }
  }, [scrutin, format, visuel, theme, lien])

  async function telecharger() {
    if (!canvasRef.current) return
    const url = URL.createObjectURL(await enBlob(canvasRef.current))
    const a = document.createElement('a')
    a.href = url
    a.download = nomFichier
    a.click()
    URL.revokeObjectURL(url)
    setStatut('Image téléchargée.')
  }

  const fichierPartageable =
    typeof navigator.canShare === 'function' &&
    navigator.canShare({
      files: [new File([], nomFichier, { type: 'image/png' })],
    })

  async function partager() {
    if (!canvasRef.current) return
    const fichier = new File([await enBlob(canvasRef.current)], nomFichier, {
      type: 'image/png',
    })
    try {
      await navigator.share({
        files: [fichier],
        text: texteDePartage(scrutin, lien),
      })
    } catch (erreur) {
      // Fermer la feuille de partage n'est pas une erreur à signaler.
      if (erreur instanceof DOMException && erreur.name === 'AbortError') return
      setStatut('Le partage a échoué. Téléchargez l’image à la place.')
    }
  }

  async function copierTexte() {
    try {
      await navigator.clipboard.writeText(texteDePartage(scrutin, lien))
      setStatut('Texte copié : à coller avec l’image.')
    } catch {
      setStatut('Copie refusée par le navigateur.')
    }
  }

  const { largeur, hauteur, usage } = FORMATS[format]

  return (
    <Card className="mb-5 px-5 py-4">
      <div className="mb-3 flex items-baseline justify-between gap-4">
        <h3 className="text-xs font-semibold uppercase tracking-[0.06em] text-ink-faint">
          Partager en image
        </h3>
        <button
          type="button"
          onClick={onClose}
          className="text-xs font-medium text-accent hover:underline"
        >
          Fermer
        </button>
      </div>

      <div className="grid gap-5 md:grid-cols-[minmax(0,1fr)_16rem]">
        <div className="flex min-w-0 items-center justify-center rounded-lg bg-surface-soft p-3">
          <canvas
            ref={canvasRef}
            role="img"
            aria-label={`Aperçu de l'image du scrutin n° ${scrutin.number}`}
            className="block h-auto max-h-[28rem] w-auto max-w-full rounded-md shadow-card"
          />
        </div>

        <div className="space-y-4">
          <div className="space-y-1">
            <SegmentedControl
              label="Format"
              options={Object.entries(FORMATS).map(([value, f]) => ({
                value,
                label: f.libelle,
              }))}
              value={format}
              onChange={(v) => setFormat(v as FormatImage)}
            />
            <p className="text-[11px] text-ink-faint">
              {largeur} × {hauteur} px · {usage}
            </p>
          </div>

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
