// The stores every panel reads:
//
// - `app.state`: the latest `AppState` from the session, replaced whole on every change
//   (up to ~60 times a second while playing). Only a snapshot with a higher `version`
//   than the last one applied replaces it. Read slices of it with `$derived`; Svelte
//   only touches the DOM where a value really changed.
// - `app.send(cmd)`: every action goes through here.
// - `app.library`: the style list, re-fetched when `state.library.revision` changes.
// - `app.sounds`: the sound catalog (#117), re-fetched when `state.sounds.revision` changes.
// - `clock.beats`: the engine's LED clock (lamps flash and pulse on it) and `clock.pos`:
//   the position in the section, both run on from the state's anchors every frame.
// - `ui`: app-only state (overlays, theme) that the engine doesn't know about.

import { initialState } from './api/mock'
import type { Session } from './api/session'
import type { AppCmd, AppState, ClockState, LibraryList, Meters, SoundCatalog } from './api/types'

class AppStore {
  state = $state.raw<AppState>(initialState())
  library = $state.raw<LibraryList>({ revision: 0, entries: [], voices: [], harmonyTypes: [], arpPatterns: [] })
  sounds = $state.raw<SoundCatalog>({ revision: 0, entries: [], recents: [] })
  kind = $state<'mock' | 'tauri' | null>(null)
  private session: Session | null = null
  private unsub: (() => void) | null = null
  private libraryRevision = -1
  private soundsRevision = -1

  /** The version of the state last applied; older or repeated snapshots are dropped. */
  private version = -Infinity

  attach(session: Session) {
    this.detach()
    this.session = session
    this.kind = session.kind
    this.version = -Infinity
    this.soundsRevision = -1
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
    // An engine before #117 has no catalog.
    const sounds = s.sounds?.revision
    if (sounds !== undefined && sounds !== this.soundsRevision && this.session?.sounds) {
      this.soundsRevision = sounds
      this.session.sounds().then((c) => (this.sounds = c))
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

  /** The latest meters (levels and, #340, each track's CPU); null without a session. */
  meters(): Promise<Meters | null> {
    return this.session ? this.session.meters() : Promise.resolve(null)
  }

  /** Open (focus) or close a keyboard part's plugin editor window (the app shell's, on
   * its main thread). */
  pluginEditor(part: number, open: boolean) {
    this.session?.pluginEditor?.(part, open)
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

/** A sound picked for a program map rule (`SoundPicker`). */
export interface SoundPick {
  /** "Piano family", "Drum rule". */
  title: string
  /** The library patch it names now (null: none). */
  value: string | null
  /** Gets the catalog id picked. */
  onpick: (id: string) => void
}

/** The top-level page under the header (docs/racks.md, "Screens"): the stage, or Library. */
export type View = 'stage' | 'library'

/** Library's tabs. */
export type LibraryTab = 'racks' | 'sounds' | 'instruments' | 'map'

class UiStore {
  /** The page in place of the stage: Stage | Library (the header's switch, Alt+B). Drawers
   * open over either. */
  view = $state<View>('stage')
  /** Library's tab, kept while it's closed. */
  libraryTab = $state<LibraryTab>('sounds')
  /** The keyboard part (0-3) Library's Sounds and Instruments load into: "Loads into". */
  libraryPart = $state(0)
  /** Overlays and drawers around the hardware view. */
  browser = $state(false)
  settings = $state(false)
  /** The Rack panel's drawer on Stage (docs/racks.md). */
  rack = $state(false)
  mixer = $state(false)
  /** The Effects screen: the Reverb, Chorus and Delay blocks and the style's inserts. */
  effects = $state(false)
  charts = $state(false)
  looper = $state(false)
  multipad = $state(false)
  harmony = $state(false)
  /** The sound picker for a program map rule (Library › Style map, #117): what for, the
   * patch it names now, and where the pick goes. Null: not picking. */
  soundPick = $state.raw<SoundPick | null>(null)
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
  toggleDrawer(d: 'rack' | 'mixer' | 'effects' | 'settings' | 'charts' | 'looper' | 'multipad' | 'harmony') {
    const open = !this[d]
    this.rack = this.mixer = this.effects = this.settings = this.charts = this.looper = this.multipad = this.harmony = false
    this[d] = open
  }

  /** Show Library, on `tab` (else the last one) and loading into `part` (else the last one). */
  openLibrary(tab?: LibraryTab, part?: number) {
    if (tab) this.libraryTab = tab
    if (part !== undefined) this.libraryPart = Math.max(0, Math.min(3, part))
    this.view = 'library'
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
    if (this.soundPick !== null) {
      this.soundPick = null
      return true
    }
    if (this.browser) return !(this.browser = false)
    if (this.settings) return !(this.settings = false)
    if (this.rack) return !(this.rack = false)
    if (this.mixer) return !(this.mixer = false)
    if (this.effects) return !(this.effects = false)
    if (this.charts) return !(this.charts = false)
    if (this.looper) return !(this.looper = false)
    if (this.multipad) return !(this.multipad = false)
    if (this.harmony) return !(this.harmony = false)
    // Library is a page, not an overlay: Esc goes back to Stage once nothing is open over it.
    if (this.view === 'library') {
      this.view = 'stage'
      return true
    }
    return false
  }
}

export const ui = new UiStore()
