// The stores every panel reads:
//
// - `app.state`: the latest `AppState` from the session, replaced whole on every change
//   (up to ~60 times a second while playing). Only a snapshot with a higher `version`
//   than the last one applied replaces it. Read slices of it with `$derived`; Svelte
//   only touches the DOM where a value really changed.
// - `app.send(cmd)`: every action goes through here.
// - `app.library`: the style list, re-fetched when `state.library.revision` changes.
// - `clock.beats`: the engine's LED clock (lamps flash and pulse on it) and `clock.pos`:
//   the position in the section, both run on from the state's anchors every frame.
// - `ui`: app-only state (overlays, theme) that the engine doesn't know about.

import { initialState } from './api/mock'
import type { Session } from './api/session'
import type { AppCmd, AppState, ClockState, LibraryList } from './api/types'

class AppStore {
  state = $state.raw<AppState>(initialState())
  library = $state.raw<LibraryList>({ revision: 0, entries: [], voices: [] })
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
 * The engine's clocks, run on between states (docs/app-api.md, "surface.clock"). A state
 * carries anchors, not a ticking position: on each one we note when it arrived, and every
 * animation frame reads
 *
 *   t   = atMs + (now − receivedMs)                       session ms, now
 *   pos = running ? max(0, sectionAnchorBeats + (t − sectionAnchorMs)·tempo/60000) : 0
 *   led = ledAnchorBeats + (t − ledAnchorMs)·tempo/60000
 *
 * - `beats`: the LED clock, free-running. Lamps flash and pulse on it, as the hardware pads do.
 * - `pos`: quarter notes into the section playing (0 stopped), for position displays.
 */
class BeatClock {
  beats = $state(0)
  pos = $state(0)
  private c: ClockState | null = null
  private receivedMs = 0
  private raf = 0

  sync(s: AppState) {
    this.c = s.surface?.clock ?? null
    this.receivedMs = now()
    this.tick()
  }

  tick() {
    const c = this.c
    if (!c) return
    const t = c.atMs + (now() - this.receivedMs)
    const perMs = c.tempo / 60000
    // Never before the section's start (a local clock a hair behind the engine's).
    this.pos = c.running ? Math.max(0, c.sectionAnchorBeats + (t - c.sectionAnchorMs) * perMs) : 0
    this.beats = c.ledAnchorBeats + (t - c.ledAnchorMs) * perMs
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
  looper = $state(false)
  multipad = $state(false)
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
  toggleDrawer(d: 'parts' | 'mixer' | 'settings' | 'looper' | 'multipad') {
    const open = !this[d]
    this.parts = this.mixer = this.settings = this.looper = this.multipad = false
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
    if (this.looper) return !(this.looper = false)
    if (this.multipad) return !(this.multipad = false)
    return false
  }
}

export const ui = new UiStore()
