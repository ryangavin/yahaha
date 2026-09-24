// The stores every panel reads:
//
// - `app.state`: the latest `AppState` from the session, replaced whole on every change
//   (up to ~60 times a second while playing). Only a snapshot with a higher `version`
//   than the last one applied replaces it. Read slices of it with `$derived`; Svelte
//   only touches the DOM where a value really changed.
// - `app.send(cmd)`: every action goes through here.
// - `app.library`: the style list, re-fetched when `state.library.revision` changes.
// - `clock.beats`: a beat clock running at the current tempo, for lamp animation
//   (flash/pulse), advanced every animation frame.
// - `ui`: app-only state (overlays, theme) that the engine doesn't know about.

import { initialState } from './api/mock'
import type { Session } from './api/session'
import type { AppCmd, AppState, LibraryList } from './api/types'

class AppStore {
  state = $state.raw<AppState>(initialState())
  library = $state.raw<LibraryList>({ revision: 0, entries: [] })
  kind = $state<'mock' | 'tauri' | null>(null)
  private session: Session | null = null
  private unsub: (() => void) | null = null
  private libraryRevision = -1

  /** The version of the state last applied; older or repeated snapshots are dropped. */
  private version = -Infinity

  attach(session: Session) {
    this.detach()
    this.session = session
    this.kind = session.kind
    this.version = -Infinity
    this.unsub = session.subscribe((s) => this.apply(s))
  }

  /**
   * Applies a snapshot only if it's newer than the last one applied. State fetches can
   * resolve out of order (two `invoke('state')` in flight), and an older snapshot must
   * never overwrite a newer one. Returns whether it was applied.
   */
  apply(s: AppState): boolean {
    if (!(s.version > this.version)) return false
    this.version = s.version
    this.state = s
    clock.sync(s)
    if (s.library.revision !== this.libraryRevision && this.session) {
      this.libraryRevision = s.library.revision
      this.session.library().then((l) => (this.library = l))
    }
    return true
  }

  detach() {
    this.unsub?.()
    this.unsub = null
    this.session = null
  }

  send(cmd: AppCmd) {
    this.session?.send(cmd)
  }
}

export const app = new AppStore()

/**
 * The beat clock the lamps flash and pulse on. The state says which beat the band is on;
 * between beats this runs on at the tempo. Stopped, it free-runs at the tempo, so an
 * armed Sync Start still breathes.
 */
class BeatClock {
  beats = $state(0)
  private base = 0
  private at = 0
  private tempo = 120
  private key = ''
  private raf = 0

  sync(s: AppState) {
    const t = s.transport
    this.tempo = t.tempo
    // The engine's clock, when it sends one (provisional `surface.clock`), includes the
    // phase within the beat, so the lamps run in step with the hardware.
    const c = s.surface?.clock
    const key = t.running ? `${t.section}:${t.bar}:${t.beat}` : 'stopped'
    if (key === this.key && !c?.atMs) return
    this.key = key
    this.at = now()
    const phase = c && t.running ? c.phase : 0
    this.base = t.running ? (t.bar - 1) * t.beatsPerBar + (t.beat - 1) + phase : this.beats
    this.running = t.running
  }

  private running = false

  tick() {
    const elapsed = ((now() - this.at) / 60000) * this.tempo
    // While playing, never run past the next beat before the state says we're there.
    this.beats = this.base + (this.running ? Math.min(elapsed, 0.999) : elapsed)
  }

  start() {
    if (typeof requestAnimationFrame === 'undefined' || this.raf) return
    const loop = () => {
      this.tick()
      this.raf = requestAnimationFrame(loop)
    }
    this.raf = requestAnimationFrame(loop)
  }

  stop() {
    if (this.raf) cancelAnimationFrame(this.raf)
    this.raf = 0
  }
}

const now = () => (typeof performance !== 'undefined' ? performance.now() : Date.now())

export const clock = new BeatClock()

export type Theme = 'dark' | 'light'

function storedTheme(): Theme {
  try {
    return localStorage.getItem('yahaha.theme') === 'light' ? 'light' : 'dark'
  } catch {
    return 'dark'
  }
}

/** Keys on the keyboard strip: the Launchkey 49 or 61, or a full 88. */
export type KeyRange = 49 | 61 | 88

function storedKeyRange(): KeyRange | null {
  try {
    const n = Number(localStorage.getItem('yahaha.keys'))
    return n === 49 || n === 61 || n === 88 ? n : null
  } catch {
    return null
  }
}

class UiStore {
  /** Overlays and drawers around the hardware view. */
  browser = $state(false)
  settings = $state(false)
  parts = $state(false)
  mixer = $state(false)
  theme = $state<Theme>(storedTheme())
  /** The keyboard strip's size; null: match the connected Launchkey (49 or 61). */
  keyRange = $state<KeyRange | null>(storedKeyRange())
  /** The Launchkey mirror's Shift layer: latched on screen, or the Shift key held. */
  shiftLatched = $state(false)
  shiftHeld = $state(false)

  get shift(): boolean {
    return this.shiftLatched || this.shiftHeld
  }

  /** Open one side drawer (closing the others), or close it if it's open. */
  toggleDrawer(d: 'parts' | 'mixer' | 'settings') {
    const open = !this[d]
    this.parts = this.mixer = this.settings = false
    this[d] = open
  }

  setTheme(t: Theme) {
    this.theme = t
    try {
      localStorage.setItem('yahaha.theme', t)
    } catch {
      /* private window: the theme just isn't remembered */
    }
  }

  setKeyRange(k: KeyRange | null) {
    this.keyRange = k
    try {
      if (k) localStorage.setItem('yahaha.keys', String(k))
      else localStorage.removeItem('yahaha.keys')
    } catch {
      /* not remembered */
    }
  }

  /** Esc: close the topmost overlay. Returns whether anything closed. */
  escape(): boolean {
    if (this.browser) return !(this.browser = false)
    if (this.settings) return !(this.settings = false)
    if (this.parts) return !(this.parts = false)
    if (this.mixer) return !(this.mixer = false)
    return false
  }
}

export const ui = new UiStore()
