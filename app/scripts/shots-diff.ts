// The pixel comparison behind `npm run shots`: a story's screenshot against its board crop.

import pixelmatch from 'pixelmatch'
import { PNG } from 'pngjs'

/** A story passes when at most this share of its pixels differ from the crop. */
export const PASS_SCORE = 0.02

export type Diff = {
  /** Share of pixels that differ, 0 (identical) to 1. A size mismatch counts the extra area as different. */
  score: number
  /** Pixels that differ. */
  pixels: number
  /** The diff image: differing pixels red over a faded copy of the shot. */
  image: PNG
  sameSize: boolean
}

/** Copies `src` onto a `width` × `height` canvas at the top left; the rest stays transparent. */
function padTo(src: PNG, width: number, height: number): PNG {
  if (src.width === width && src.height === height) return src
  const out = new PNG({ width, height })
  PNG.bitblt(src, out, 0, 0, src.width, src.height, 0, 0)
  return out
}

/** Compares a shot with its crop. Images of different sizes are compared on the larger canvas. */
export function diffImages(shot: PNG, crop: PNG): Diff {
  const width = Math.max(shot.width, crop.width)
  const height = Math.max(shot.height, crop.height)
  const a = padTo(shot, width, height)
  const b = padTo(crop, width, height)
  const image = new PNG({ width, height })
  const pixels = pixelmatch(a.data, b.data, image.data, width, height, { threshold: 0.1 })
  return {
    score: pixels / (width * height),
    pixels,
    image,
    sameSize: shot.width === crop.width && shot.height === crop.height,
  }
}
