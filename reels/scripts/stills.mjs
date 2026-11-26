// Carrousel du post du jour : la composition `<Sujet>Post` (mêmes scènes que le
// Reel, en 4:5) rendue en une image fixe par scène, à la dernière image de
// chacune (`stillFrames` de `src/timeline.ts`), en PNG.
//
// En ligne de commande, rend les exemples de `samples/` dans `out/post-<exemple>/`
// pour relecture : `node scripts/stills.mjs [votes|programmes|theme|nouveaute]`.

import { mkdirSync, readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { bundle } from '@remotion/bundler'
import { renderStill, selectComposition } from '@remotion/renderer'

const REELS = join(dirname(fileURLToPath(import.meta.url)), '..')

/**
 * Rend le carrousel et renvoie les chemins des images, dans l'ordre des scènes.
 * Les images à tirer viennent de `stillFrames` (`src/timeline.ts`), posées
 * dans les propriétés par `calculatePostMetadata` de `Root.tsx`.
 */
export async function renderPostImages(composition, props, outDir) {
  mkdirSync(outDir, { recursive: true })
  const serveUrl = await bundle({ entryPoint: join(REELS, 'src', 'index.ts') })
  const inputProps = { ...props, post: true }
  const selected = await selectComposition({ serveUrl, id: `${composition}Post`, inputProps })
  const frames = selected.props.stillFrames
  const files = []
  for (const [index, frame] of frames.entries()) {
    const output = join(outDir, `${String(index + 1).padStart(2, '0')}.png`)
    await renderStill({ serveUrl, composition: selected, inputProps, frame, output, imageFormat: 'png' })
    files.push(output)
  }
  return files
}

const SAMPLES = { votes: 'Votes', programmes: 'Programmes', theme: 'Theme', nouveaute: 'SiteNews' }

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const names = process.argv.length > 2 ? process.argv.slice(2) : Object.keys(SAMPLES)
  for (const name of names) {
    const props = JSON.parse(readFileSync(join(REELS, 'samples', `${name}.json`), 'utf8'))
    const files = await renderPostImages(SAMPLES[name], props, join(REELS, 'out', `post-${name}`))
    console.log(`${name} : ${files.length} images`)
  }
}
