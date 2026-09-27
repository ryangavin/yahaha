// When to suggest a larger audio buffer. The engine counts audio dropouts
// (`io.synth.dropouts`: the device's overload reports and its own late buffers); a single
// blip is ignored, dropouts that keep coming raise a notice (DropoutNotice.svelte) that
// leads to Settings › Audio › Buffer size. Dismissed, it stays quiet for a while; a new
// buffer size starts the count again.

/** Dropouts, within `HINT_WINDOW_MS`, that raise the notice. */
export const HINT_COUNT = 3
export const HINT_WINDOW_MS = 30_000
/** How long a dismissed notice stays away. */
export const SNOOZE_MS = 10 * 60_000
/** The largest buffer `setAudioBuffer` offers: past it there is nothing to suggest. */
export const MAX_BUFFER = 1024

export class DropoutWatch {
  /** The notice is up. */
  show = $state(false)
  /** When recent dropouts were seen (ms), at most `HINT_COUNT` of them. */
  private times: number[] = []
  /** The count and buffer size last seen (null: nothing seen yet). */
  private last: number | null = null
  private buffer: number | null = null
  private snoozedUntil = 0

  /** The synth's dropout count and buffer size as of `now` (ms). Null synth: no synth. */
  observe(synth: { dropouts: number; bufferFrames: number | null } | null, now: number) {
    if (!synth) {
      this.show = false
      this.times = []
      this.last = null
      return
    }
    const { dropouts, bufferFrames } = synth
    // The first sight, a synth that restarted, or a new buffer size: count from here.
    if (this.last === null || dropouts < this.last || bufferFrames !== this.buffer) {
      this.times = []
      this.show = false
      this.last = dropouts
      this.buffer = bufferFrames
      return
    }
    const fresh = Math.min(dropouts - this.last, HINT_COUNT)
    this.last = dropouts
    for (let i = 0; i < fresh; i++) this.times.push(now)
    this.times = this.times.filter((t) => now - t <= HINT_WINDOW_MS).slice(-HINT_COUNT)
    if (!this.show && this.times.length >= HINT_COUNT && now >= this.snoozedUntil && (bufferFrames ?? 0) < MAX_BUFFER) {
      this.show = true
    }
  }

  /** Put the notice away for `SNOOZE_MS`. */
  dismiss(now: number) {
    this.show = false
    this.times = []
    this.snoozedUntil = now + SNOOZE_MS
  }

  reset() {
    this.show = false
    this.times = []
    this.last = null
    this.buffer = null
    this.snoozedUntil = 0
  }
}

export const dropouts = new DropoutWatch()
