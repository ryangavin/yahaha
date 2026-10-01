import { PNG } from 'pngjs'
import { describe, expect, it } from 'vitest'
import { PASS_SCORE, diffImages, matches } from './shots-diff.ts'

function solid(width: number, height: number, rgb: [number, number, number]): PNG {
  const png = new PNG({ width, height })
  for (let i = 0; i < width * height; i++) png.data.set([...rgb, 255], i * 4)
  return png
}

describe('diffImages', () => {
  it('scores identical images 0', () => {
    const d = diffImages(solid(10, 4, [159, 224, 74]), solid(10, 4, [159, 224, 74]))
    expect(d).toMatchObject({ score: 0, pixels: 0, sameSize: true })
  })

  it('scores the share of differing pixels', () => {
    const shot = solid(10, 4, [0, 0, 0])
    shot.data.set([255, 255, 255, 255], 0)
    shot.data.set([255, 255, 255, 255], 4)
    const d = diffImages(shot, solid(10, 4, [0, 0, 0]))
    expect(d.pixels).toBe(2)
    expect(d.score).toBeCloseTo(2 / 40)
    expect(d.image.width).toBe(10)
  })

  it('fails a shot 1px off the crop size, however low its score', () => {
    const d = diffImages(solid(128, 32, [159, 224, 74]), solid(127, 32, [159, 224, 74]))
    expect(d.score).toBeLessThan(PASS_SCORE)
    expect(matches(d)).toBe(false)
    expect(matches(diffImages(solid(127, 32, [0, 0, 0]), solid(127, 32, [0, 0, 0])))).toBe(true)
  })

  it('counts the extra area of a different size as different', () => {
    const d = diffImages(solid(12, 4, [0, 0, 0]), solid(10, 4, [0, 0, 0]))
    expect(d.sameSize).toBe(false)
    expect(d.pixels).toBe(8)
    expect(d.score).toBeCloseTo(8 / 48)
  })
})
