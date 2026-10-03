/**
 * What the app bar's health slot says: the one text for the audio's state, picked from a few
 * inputs the wiring passes (a failed plugin first, since it's silent; then no audio, dropouts,
 * a busy CPU; else the calm "Audio").
 */

/** Where a click on the slot takes the player. */
export type HealthTarget = { page: 'channel'; part: 0 | 1 | 2 | 3 } | { page: 'settings'; tab: 'system' }

/** The slot's inputs; a missing field takes the prop's default. */
export type HealthInput = {
  failedPart?: 0 | 1 | 2 | 3 | null
  synthOn?: boolean
  dropouts?: number
  bufferFrames?: number | null
  cpu?: number | null
}

/** The text, its hue role, where a click goes (null when calm) and the live region's words. */
export type Health = { text: string; hue: 'ending' | 'm'; target: HealthTarget | null; label: string }

const PARTS = ['R1', 'R2', 'R3', 'L'] as const
const SYSTEM: HealthTarget = { page: 'settings', tab: 'system' }
const LARGEST_BUFFER = 1024
const HINT_DROPOUTS = 3
const BUSY_CPU = 0.7

/** The next buffer size up to try, at most 1024. */
export function suggestBuffer(frames: number): number {
  return Math.min(frames * 2, LARGEST_BUFFER)
}

/** The first keyboard part (R1 R2 R3 L) whose plugin failed and isn't missing; null if none. */
export function firstFailedPart(
  plugins: ({ status: string; missing?: boolean } | undefined)[],
): 0 | 1 | 2 | 3 | null {
  for (let i = 0; i < 4; i++) {
    const plugin = plugins[i]
    if (plugin?.status === 'failed' && plugin.missing !== true) return i as 0 | 1 | 2 | 3
  }
  return null
}

function trouble(text: string, hue: Health['hue'], target: HealthTarget): Health {
  return { text, hue, target, label: `Audio health: ${text}` }
}

/** Picks the slot's text: the first row that applies. */
export function health(input: HealthInput): Health {
  const failedPart = input.failedPart ?? null
  const synthOn = input.synthOn ?? true
  const dropouts = input.dropouts ?? 0
  const bufferFrames = input.bufferFrames ?? null
  const cpu = input.cpu ?? null

  if (failedPart !== null) {
    return trouble(`${PARTS[failedPart]} failed`, 'ending', { page: 'channel', part: failedPart })
  }
  if (!synthOn) return trouble('Audio off', 'm', SYSTEM)
  if (dropouts >= HINT_DROPOUTS && bufferFrames !== null && bufferFrames < LARGEST_BUFFER) {
    return trouble(`${dropouts} dropouts · buffer ${suggestBuffer(bufferFrames)}?`, 'ending', SYSTEM)
  }
  if (dropouts >= 1) return trouble(dropouts === 1 ? '1 dropout' : `${dropouts} dropouts`, 'ending', SYSTEM)
  if (cpu !== null && cpu >= BUSY_CPU) return trouble(`CPU ${Math.round(cpu * 100)}%`, 'ending', SYSTEM)
  return { text: 'Audio', hue: 'm', target: null, label: 'Audio health: fine' }
}
