// The app's TEMPO −/+ buttons held down (OM p.46): one step at once, then repeating while
// held, faster the longer it is held; the other one pressed as well goes back to the
// style's own tempo. The same schedule as the engine's for the Launchkey buttons
// (src/engine/tempo_repeat.rs).

import type { AppCmd } from './api/types'

/** How long a button is held before it starts repeating. */
export const REPEAT_DELAY_MS = 400
/** A hold longer than this stops repeating (a release the page never saw). */
export const MAX_HOLD_MS = 30_000

/** The time to the next step after `repeats` repeats. */
export function repeatIntervalMs(repeats: number): number {
  return repeats < 8 ? 100 : repeats < 32 ? 50 : 25
}

export class TempoHold {
  private held = 0
  private timer: ReturnType<typeof setTimeout> | null = null

  constructor(private readonly send: (cmd: AppCmd) => void) {}

  /** TEMPO − (`dir` −1) or + (1) went down or up. */
  set(dir: -1 | 1, down: boolean) {
    const bit = dir < 0 ? 1 : 2
    this.stop()
    if (!down) {
      this.held &= ~bit
      return
    }
    this.held |= bit
    if (this.held === 3) {
      this.send({ type: 'resetTempo' })
      return
    }
    const cmd: AppCmd = { type: dir < 0 ? 'tempoDown' : 'tempoUp' }
    this.send(cmd)
    let repeats = 0
    let waited = REPEAT_DELAY_MS
    const tick = () => {
      this.send(cmd)
      repeats++
      const next = repeatIntervalMs(repeats)
      waited += next
      this.timer = waited > MAX_HOLD_MS ? null : setTimeout(tick, next)
    }
    this.timer = setTimeout(tick, REPEAT_DELAY_MS)
  }

  /** Everything let go (the window lost focus, say). */
  releaseAll() {
    this.stop()
    this.held = 0
  }

  private stop() {
    if (this.timer !== null) clearTimeout(this.timer)
    this.timer = null
  }
}
