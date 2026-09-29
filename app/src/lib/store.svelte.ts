// The stores every panel reads:
//
// - `app.state`: the latest `AppState` from the session, replaced on every change (up to
//   ~60 times a second while playing). Only a snapshot with a higher `version` than the
//   last one applied replaces it, and every part of it equal to the last one keeps the
//   last one's object (`share`), so a `$derived` slice that didn't change stays the same
//   object and nothing downstream of it re-runs.
// - `app.send(cmd)`: every action goes through here.
// - `app.library`: the style list, re-fetched when `state.library.revision` changes.
// - `app.sounds`: the sound catalog (#117), re-fetched when `state.sounds.revision` changes.
// - `clock.beats`: the engine's LED clock (lamps flash and pulse on it) and `clock.pos`:
//   the position in the section, both run on from the state's anchors every frame while
//   anything moves on them (`clockNeeded`).
// - `ui`: app-only state (overlays, theme) that the engine doesn't know about.

import { emptyState } from './api/constants'
import type { Session } from './api/session'
import type { AppCmd, AppState, ClockState, LibraryList, Meters, SoundCatalog } from './api/types'

/**
 * `next`, with every part of it that is deep-equal to the same part of `prev` replaced by
 * `prev`'s object; `prev` itself when the two are equal. Neither is changed: a part of
 * `next` holding shared parts is a copy. For plain JSON values (objects, arrays,
 * primitives), as a state snapshot is; one pass over `next`.
 */
export function share<T>(prev: unknown, next: T): T {
  if (prev === next || typeof prev !== 'object' || typeof next !== 'object' || prev === null || next === null) return next
  if (Array.isArray(prev) !== Array.isArray(next)) return next
  const p = prev as Record<string, unknown>
  const n = next as Record<string, unknown>
  const keys = Object.keys(n)
  let same = keys.length === Object.keys(p).length
  let out: Record<string, unknown> | null = null
  for (const k of keys) {
    const had = k in p
    const v = n[k]
    const s = had ? share(p[k], v) : v
    if (!had || s !== p[k]) same = false
    if (s !== v) {
      out ??= Array.isArray(n) ? (n.slice() as unknown as Record<string, unknown>) : { ...n }
      out[k] = s
    }
  }
  return (same ? prev : (out ?? n)) as T
}

class AppStore {
  state = $state.raw<AppState>(emptyState())
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
    s = share(this.state, s)
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
 *
 * The frame loop runs only while `start()`ed, the page is visible and `clockNeeded` says
 * something moves on the clocks. Otherwise the values stay as last computed (and are
 * recomputed on every state), which is all a reader needs: stopped, `pos` is 0, and a
 * solid or dark lamp doesn't depend on `beats`.
 */
class BeatClock {
  beats = $state(0)
  pos = $state(0)
  private c: ClockState | null = null
  private receivedMs = 0
  private raf = 0
  private started = false
  private needed = false
  private hidden = false
  private readonly onVisibility = () => {
    this.hidden = document.visibilityState === 'hidden'
    this.update()
  }

  /** Whether the frame loop is running. */
  get looping(): boolean {
    return this.raf !== 0
  }

  sync(s: AppState) {
    this.c = s.surface?.clock ?? null
    this.receivedMs = now()
    this.needed = clockNeeded(s)
    this.tick()
    this.update()
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

  /** Run the frame loop whenever it's needed (App does, for its life), until `stop()`. */
  start() {
    if (this.started) return
    this.started = true
    if (typeof document !== 'undefined') {
      this.hidden = document.visibilityState === 'hidden'
      document.addEventListener('visibilitychange', this.onVisibility)
    }
    this.update()
  }

  stop() {
    if (!this.started) return
    this.started = false
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', this.onVisibility)
    this.update()
  }

  /** Start or stop the frame loop to match what's needed now. */
  private update() {
    const want = this.started && this.needed && !this.hidden
    if (want && !this.raf && typeof requestAnimationFrame !== 'undefined') {
      const loop = () => {
        this.tick()
        this.raf = requestAnimationFrame(loop)
      }
      this.tick()
      this.raf = requestAnimationFrame(loop)
    } else if (!want && this.raf) {
      cancelAnimationFrame(this.raf)
      this.raf = 0
      this.tick()
    }
  }
}

/**
 * Whether anything on screen moves on the clocks, so the frame loop must run:
 * - the band is playing (`pos` runs; position displays and the chart follow it);
 * - a lamp the engine sends flashes or pulses (any `anim` 'flash'/'pulse' not off, or a
 *   palette pad's `mode` 'flash'/'pulse', which may flash to its second colour even when
 *   off: pads, section lamps, the surface's buttons);
 * - a lamp the app draws flashes or pulses: Quick Rack Store armed, a Multi Pad armed or
 *   queued, the Chord Looper armed, a style queued for preview, a scan running.
 * One pass over the state, on each state.
 */
export function clockNeeded(s: AppState): boolean {
  if (s.transport?.running || s.surface?.clock?.running) return true
  if (s.quickRacks?.store) return true
  if (s.multiPad?.pads.some((p) => p.lamp === 'armed' || p.lamp === 'queued')) return true
  if (s.looper?.mode === 'recArmed' || s.looper?.mode === 'loopArmed') return true
  if (s.preview?.queued != null) return true
  if (s.library?.scanning || s.sounds?.scanning || s.plugins?.scanning) return true
  return animates(s)
}

function animates(v: unknown): boolean {
  if (typeof v !== 'object' || v === null) return false
  if (Array.isArray(v)) return v.some(animates)
  const o = v as Record<string, unknown>
  if ((o.anim === 'flash' || o.anim === 'pulse') && o.level !== 'off') return true
  if (o.mode === 'flash' || o.mode === 'pulse') return true
  for (const k in o) if (animates(o[k])) return true
  return false
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
