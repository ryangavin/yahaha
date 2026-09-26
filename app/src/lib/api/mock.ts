// A mock session that behaves like the engine closely enough to develop and screenshot
// the UI: the band advances bar by bar, queued sections flash and take over at the bar
// (fills at the beat), chords change, faders wait for pickup. No audio, no MIDI.
// It speaks #16's API (types.ts); `app/src-tauri/src/mock.rs` is the Rust twin that the
// app shell runs until the engine's `Session` is wired in.

import { controlSwitchSets, defaultControllers, functionCmd, functionInfo, functionSet, isPedalSwitch, pedalCcRefused, resetRelease } from './assignable'
import fixture from './mock-fixture.json'
import { DEMO_PLAYLIST, DEMO_SONGS, chordAt, emptyChart, info, nextBar, songState, styleWords } from './mock-chart'
import { syntheticStyles } from './mock-library'
import { clockAt, mockSurface, type MockHardware } from './mock-surface'
import { emptyLooper, MockLooper } from './mock-looper'
import { initialMultiPad, MockPads } from './mock-multipad'
import { initialSoundLibrary, MockSoundLibrary } from './mock-sound-library'
import { initialSounds, MockSounds } from './mock-sounds'
import { padsFor } from './mock-pads'
import { MockKnobs } from './mock-knobs'
import { initialPlugins, MockPlugins } from './mock-plugins'
import { ARP_PATTERNS, HARMONY_TYPES, harmonyArpCmd, initialHarmonyArp } from './mock-harmony'
import { MockRegistration } from './mock-registration'
import { emptyPlaylist, emptyRegistration } from './registration'
import type { Session } from './session'
import {
  BREAK, CHORD_SETTLE_MAX_MS, ENDINGS, FILLS, FINGERINGS, INTROS, KEYBOARD_PART_NAMES, MAINS, PAD_PAGES, RETRIGGER_RATES,
  STYLE_PART_NAMES, type AppCmd, type AppState, type EffectBlockState, type EffectsState, type FxBlock, type FxParam, type FxParamState, type FxType, type LibraryEntry, type LibraryList, type OtsPart, type PreviewState, type StopAcmpMode,
  type SoundLibraryCmd, type StyleSettingsState, type StyleState,
} from './types'

export const GM: string[] = fixture.gm

interface FixtureStyle {
  id: number
  name: string
  folder: string
  file: string
  tempo: number
  timeSignature: number[]
  sections: string[]
  ots: number
  /** SFF1/SFF2; the fixture's own styles go by file extension. */
  format?: string
  error?: string
}
const STYLES = fixture.styles as FixtureStyle[]
const ROOT = '/Users/me/Styles'

/** "Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break", as the engine lists sections. */
function sectionsText(sections: string[]): string {
  const letters = (names: string[]) => names.filter((n) => sections.includes(n)).map((n) => n.slice(-1)).join('')
  const parts = [
    ['Main', letters(MAINS)],
    ['Intro', letters(INTROS)],
    ['Ending', letters(ENDINGS)],
    ['Fill', letters(FILLS)],
  ].filter(([, l]) => l).map(([k, l]) => `${k} ${l}`)
  if (sections.includes(BREAK)) parts.push('Break')
  return parts.join(' · ')
}

function formatOf(s: FixtureStyle): string {
  return s.format ?? (s.file.endsWith('.sty') ? 'SFF1' : 'SFF2')
}

function stylePath(s: FixtureStyle): string {
  return s.folder ? `${ROOT}/${s.folder}/${s.file}` : `${ROOT}/${s.file}`
}

function entryOf(s: FixtureStyle): LibraryEntry {
  return {
    id: s.id,
    name: s.name,
    folder: s.folder,
    path: stylePath(s),
    status: s.error ? 'error' : 'ok',
    error: s.error ?? null,
    tempo: s.error ? null : s.tempo,
    timeSignature: s.error ? null : [s.timeSignature[0], s.timeSignature[1]],
    sections: s.error ? '' : sectionsText(s.sections),
    format: s.error ? null : formatOf(s),
  }
}

/** The voices `setPartVoice` picks from, as the engine lists them (GM, bank 0). */
export const VOICES: LibraryList['voices'] = GM.map((name, program) => ({ program, bankMsb: 0, bankLsb: 0, name }))

export const LIBRARY: LibraryList = {
  revision: 1, entries: STYLES.map(entryOf), voices: VOICES, harmonyTypes: HARMONY_TYPES, arpPatterns: ARP_PATTERNS,
}

/** The fixture plus `extra` synthetic styles, in the engine's order (folder, then name). */
function bigLibrary(extra: number): { lib: LibraryList; styles: FixtureStyle[] } {
  const styles: FixtureStyle[] = [...STYLES, ...syntheticStyles(extra, STYLES.length)]
  const entries = styles.map(entryOf)
  const key = (e: LibraryEntry) => [e.folder.toLowerCase(), e.name.toLowerCase(), e.path]
  entries.sort((a, b) => {
    const [x, y] = [key(a), key(b)]
    for (let i = 0; i < 3; i++) if (x[i] !== y[i]) return x[i] < y[i] ? -1 : 1
    return 0
  })
  return { lib: { revision: 1, entries, voices: VOICES, harmonyTypes: HARMONY_TYPES, arpPatterns: ARP_PATTERNS }, styles }
}

/** The MIDI sources the mock rig has (every one a keyboard: `allInputs`). */
const MOCK_SOURCES = [
  { name: 'Launchkey 49 MK4 LKMK4 MIDI Out', listening: true, pads: false },
  { name: 'Launchkey 49 MK4 LKMK4 DAW Out', listening: true, pads: true },
  { name: 'TASCAM Model 16', listening: true, pads: false },
  { name: 'IAC Driver Bus 1', listening: true, pads: false },
]
const MOCK_SOUND_FONTS = ['GeneralUser-GS.sf2', 'FluidR3_GM.sf2', 'MuseScore_General.sf2']
/** How long the mock's rescan takes. */
const RESCAN_MS = 1200

/** The default progression an audition plays, one chord a bar (#21). */
export const AUDITION_PROGRESSION = ['C', 'Am', 'F', 'G7']

const NOTE_NAMES = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B']

/** Yamaha numbering: C3 = middle C (60). */
export function noteName(n: number): string {
  return `${NOTE_NAMES[n % 12]}${Math.floor(n / 12) - 2}`
}

/** Move a chord name ("Am7/G") by `d` semitones. */
export function transposeChord(name: string, d: number): string {
  const shift = (root: string) => {
    const i = NOTE_NAMES.indexOf(root)
    return i < 0 ? root : NOTE_NAMES[(((i + d) % 12) + 12) % 12]
  }
  return name.replace(/^([A-G][b#]?)/, (m) => shift(m)).replace(/\/([A-G][b#]?)$/, (_, m) => '/' + shift(m))
}

/** Intervals of the chord qualities the mock's progression uses (the engine recognises many more). */
const QUALITIES: Record<string, number[]> = {
  '': [0, 4, 7], m: [0, 3, 7], '7': [0, 4, 7, 10], m7: [0, 3, 7, 10], maj7: [0, 4, 7, 11], '6': [0, 4, 7, 9],
  m6: [0, 3, 7, 9], sus4: [0, 5, 7], '7sus4': [0, 5, 7, 10], dim: [0, 3, 6], aug: [0, 4, 8],
}

/** The mock's stand-in for the engine's chord tones: pitch classes, root first, and the bass. */
export function chordTones(name: string | null): { tones: number[]; bass: number | null } {
  const m = name ? /^([A-G][b#]?)([^/]*)(?:\/([A-G][b#]?))?$/.exec(name) : null
  const root = m ? NOTE_NAMES.indexOf(m[1]) : -1
  if (!m || root < 0) return { tones: [], bass: null }
  const tones = (QUALITIES[m[2]] ?? QUALITIES['']).map((i) => (root + i) % 12)
  const slash = m[3] ? NOTE_NAMES.indexOf(m[3]) : -1
  return { tones, bass: slash >= 0 ? slash : root }
}

const PROGRESSION = ['C', 'Am7', 'Fmaj7', 'G7', 'Em7', 'A7', 'Dm7', 'G7sus4', 'C/E', 'F', 'Fm6', 'C']
const STYLE_VOICES: [number, number, number, boolean, string][] = [
  [127, 0, 0, true, 'drum kit 127/0/1'],
  [127, 0, 25, true, 'drum kit 127/0/26'],
  [0, 0, 33, false, 'Finger Bass (GM 34)'],
  [0, 112, 27, false, '≈ Clean Gtr  [Yamaha 0/112/28]'],
  [0, 112, 4, false, '≈ E.Piano 1  [Yamaha 0/112/5]'],
  [104, 0, 48, false, '≈ Strings  [Yamaha 104/0/49]'],
  [0, 0, 61, false, 'Brass Section (GM 62)'],
  [0, 0, 73, false, 'Flute (GM 74)'],
]
/** [program, on, volume, octave] for Right 1, Right 2, Right 3, Left, per OTS. */
const OTS_SETUPS: [number, boolean, number, number][][] = [
  [[0, true, 100, 0], [48, true, 70, 0], [61, false, 90, 0], [48, false, 80, 0]],
  [[4, true, 100, 0], [89, true, 60, 1], [61, false, 90, 0], [33, false, 90, -1]],
  [[16, true, 96, 0], [61, false, 90, 0], [56, false, 90, 0], [48, false, 80, 0]],
  [[65, true, 104, 0], [61, true, 80, 0], [56, false, 90, 0], [48, true, 70, 0]],
]

function styleState(s: FixtureStyle): StyleState {
  return {
    id: s.id,
    path: stylePath(s),
    name: s.name,
    format: formatOf(s),
    tempo: s.tempo,
    timeSignature: [s.timeSignature[0], s.timeSignature[1]],
    sections: s.sections,
  }
}

function otsSettings(n: number) {
  return OTS_SETUPS.slice(0, n).map((parts, i) => ({
    name: `OTS ${i + 1}`,
    parts: parts.map(([program, on, volume, octave]): OtsPart => ({ on, program, voiceName: GM[program], volume, octave })),
  }))
}

function beatsPerBar([n, d]: [number, number]): number {
  return d === 8 && n % 3 === 0 ? n / 3 : n
}

/** How long a section's pattern is, in bars (the mock's; real styles vary). */
function patternBars(s: string): number {
  return MAINS.includes(s) ? 4 : sectionBars(s)
}

/** The engine's default Style settings (src/engine/timing.rs). */
export const DEFAULT_STYLE_SETTINGS: StyleSettingsState = {
  mainTiming: 'nextBar', introEndingTiming: 'nextBar', syncStopWindowMs: 0,
  fadeInMs: 5000, fadeOutMs: 5000, fadeHoldMs: 2000, sectionReset: true, retriggerRate: 8,
}

/** A stopped session with the first style loaded and Sync Start armed. */
export function initialState(): AppState {
  const s = STYLES[0]
  const part = (i: number, program: number, on: boolean) => ({
    name: KEYBOARD_PART_NAMES[i], channel: [1, 3, 4, 2][i], on, sounding: on, selected: i === 0,
    volume: 100, waiting: false, program, voiceName: GM[program], playsBass: false, octave: 0, pan: 64, reverb: i === 3 ? 40 : 50, chorus: 10, variation: 0, fader: null, patch: null as string | null,
  })
  const state: AppState = {
    version: 1,
    style: styleState(s),
    transport: {
      running: false, syncStart: true, syncStop: false, syncStopAvailable: true, autoFill: false, stopAcmp: false,
      section: null, queued: null, pendingIntro: null, main: 0, bar: 1, beat: 1,
      beatsPerBar: beatsPerBar([s.timeSignature[0], s.timeSignature[1]]), tempo: s.tempo, lamps: [], sectionBars: null,
      halfBarFill: false, stopAcmpMode: 'off',
      fade: 'off', retrigger: false, ritardando: false,
    },
    chord: {
      name: null, fingered: null, fingering: 'fingeredOnBass', fingeringName: 'Fingered On Bass', upper: false,
      manualBass: true, manualBassActive: false, split: 54, splitName: noteName(54), transposeKeyboard: 0, transposeMaster: 0, settleMs: 10, leftHold: false,
    },
    keyboardParts: [part(0, 0, true), part(1, 48, false), part(2, 61, false), part(3, 48, false)],
    keyboard: { held: [], leftSplit: 54, chordTones: [], chordBass: null, detection: [0, 54] },
    mixer: {
      faderPage: 'panel',
      styleParts: STYLE_PART_NAMES.map((name, i) => ({
        name, channel: 9 + i, on: true, mutedByManualBass: false,
        volume: [100, 100, 96, 80, 76, 70, 88, 84][i], waiting: false, fader: null,
        voice: { bankMsb: STYLE_VOICES[i][0], bankLsb: STYLE_VOICES[i][1], program: STYLE_VOICES[i][2], kit: STYLE_VOICES[i][3], label: STYLE_VOICES[i][4] },
      })),
      master: 100,
      masterWaiting: false,
      styleVolume: 100,
      styleVolumeWaiting: false,
      multiPadVolume: 100,
      multiPadVolumeWaiting: false,
      styleSolo: null,
      partSolo: null,
    },
    pads: { page: 'sections', pageName: 'Sections', pageNumber: 1, pageCount: PAD_PAGES.length, pads: [], connected: true, paletteLeds: false },
    ots: { settings: otsSettings(s.ots), applied: 0, link: false, linkTiming: 'mainChange' },
    library: { revision: LIBRARY.revision, count: LIBRARY.entries.length, position: 0, pending: 0, roots: [ROOT], scanning: false },
    io: {
      outputPort: 'yahaha',
      inputs: MOCK_SOURCES.map((s) => (s.pads ? `${s.name} (pads)` : s.name)),
      synth: {
        soundFont: 'GeneralUser-GS', device: 'MacBook Pro Speakers', sampleRate: 48000, bufferFrames: 64,
        channels: 2, outputPair: [1, 2], muted: false,
      },
      engine: { realtime: true, wakeP99Us: 3, chordP99Us: 15, midiInP99Us: 120 },
      lastControl: 0,
      unmapped: '',
      offline: false,
      sources: MOCK_SOURCES.map((s) => ({ ...s })),
      allInputs: true,
      soundFonts: [...MOCK_SOUND_FONTS],
      soundFontFile: MOCK_SOUND_FONTS[0],
      soundFontLoading: false,
      defaultSoundSet: null,
      autoSoundSet: MOCK_SOUND_FONTS[0],
    },
    message: null,
    styleChange: { tempo: 'hold', parts: 'hold', sectionSet: null },
    surface: null as unknown as AppState['surface'], // filled in by derive()
    preview: { audition: null, queued: null },
    chart: emptyChart(),
    styleSettings: { ...DEFAULT_STYLE_SETTINGS },
    registration: emptyRegistration(),
    playlist: emptyPlaylist(),
    looper: emptyLooper(),
    metronome: { on: false, volume: 90, bell: true, audible: true },
    multiPad: initialMultiPad(),
    controllers: defaultControllers(),
    plugins: initialPlugins(),
    harmonyArp: initialHarmonyArp(),
    soundLibrary: initialSoundLibrary(),
    paramLocks: { splitPoint: false, fingeringType: false },
    sounds: initialSounds(),
    dynamics: { control: true, level: 64, touch: false, accent: false, accentThreshold: 110 },
    knobs: { page: 'style', pageName: 'Style', pageNumber: 1, pageCount: 4, knobs: [] },
    effects: initialEffects(),
  }
  derive(state, LIBRARY)
  state.knobs = new MockKnobs().state(state)
  return state
}

/** A stopped clock at session time 0, and faders that haven't moved. */
function idleHardware(st: AppState): MockHardware {
  const clock = {
    atMs: 0, running: false, tempo: st.transport.tempo, beatsPerBar: st.transport.beatsPerBar, bar: 1, beat: 1, phase: 0,
    sectionAnchorMs: 0, sectionAnchorBeats: 0, ledAnchorMs: 0, ledAnchorBeats: 0,
  }
  return { faders: Array(9).fill(null), clock }
}

/** The fields the engine computes from the others: pads, lamps, names, flags, and the
 * `surface` (with the mock's hardware fader positions and clocks). */
function derive(st: AppState, lib: LibraryList, hw: MockHardware | null = null, held: number[] = []) {
  const c = st.chord
  c.fingeringName = c.upper ? 'Fingered*' : FINGERINGS.find((f) => f.id === c.fingering)!.name
  c.manualBassActive = c.upper && c.manualBass
  c.splitName = noteName(c.split)
  st.transport.syncStopAvailable = c.upper || !(c.fingering === 'fullKeyboard' || c.fingering === 'aiFullKeyboard')
  st.keyboardParts.forEach((p, i) => {
    p.playsBass = i === 3 && c.manualBassActive
    // A keyboard solo: only that part sounds, even if it is off.
    p.sounding = st.mixer.partSolo === null ? p.on || p.playsBass : st.mixer.partSolo === i
    p.voiceName = p.playsBass ? 'Finger Bass' : GM[p.program]
  })
  st.mixer.styleParts.forEach((p, i) => (p.mutedByManualBass = i === 2 && c.manualBassActive))
  st.transport.sectionBars = st.transport.section ? patternBars(st.transport.section) : null
  // The keyboard strip: which part sounds each held key, and the chord's tones.
  const right = st.keyboardParts.slice(0, 3).flatMap((p, i) => (p.sounding ? [i] : []))
  const left = st.keyboardParts[3].sounding ? [3] : []
  const ct = chordTones(c.name)
  st.keyboard = {
    held: [...held].sort((a, b) => a - b).map((note) => {
      const lower = note <= c.split
      return { note, zone: lower ? ('left' as const) : ('right' as const), parts: lower ? left : right }
    }),
    leftSplit: c.split,
    chordTones: ct.tones,
    chordBass: ct.bass,
    // Lower: up to the split; Upper: above it; the Full Keyboard types: every key.
    detection: c.upper
      ? [Math.min(127, c.split + 1), 127]
      : c.fingering === 'fullKeyboard' || c.fingering === 'aiFullKeyboard'
        ? [0, 127]
        : [0, c.split],
  }
  const page = PAD_PAGES.findIndex((p) => p.id === st.pads.page)
  st.pads.pageName = PAD_PAGES[page].name
  st.pads.pageNumber = page + 1
  st.transport.lamps = padsFor(st, 'sections')
  st.pads.pads = padsFor(st, st.pads.page)
  const h = hw ?? idleHardware(st)
  st.keyboardParts.forEach((p, i) => (p.fader = h.faders[i] ?? null))
  st.mixer.styleParts.forEach((p, i) => (p.fader = h.faders[i] ?? null))
  st.surface = mockSurface(st, lib, h)
}

/** How many bars a section lasts before it moves on (Intro/Ending: 2, Break/Fill: 1). */
function sectionBars(s: string): number {
  return INTROS.includes(s) || ENDINGS.includes(s) ? 2 : 1
}

/** The demo player pressing Main `index` on pad page 1, as the engine packs it in
 * `io.lastControl` (0x00SSDDVV: note on, pad note 112 + index, velocity 127). */
const padPress = (index: number) => (0x90 << 16) | ((112 + index) << 8) | 127

export interface MockOptions {
  /** Script some activity: start mid-song, change Main every few bars, move faders. */
  demo?: boolean
  /** Drive the clock yourself with `advance(ms)` instead of a 60 Hz timer (tests). */
  manual?: boolean
  /** Add this many synthetic styles to the library (`?styles=60000`), to test a big one. */
  styles?: number
  /** Import the demo chart playlist and turn chart mode on (`?chart=1`). */
  chart?: boolean
  /** Start with the demo Registration banks and Playlist (default true). */
  registration?: boolean
}

export class MockSession implements Session {
  readonly kind = 'mock' as const
  state: AppState
  private subs = new Set<(s: AppState) => void>()
  private timer: ReturnType<typeof setInterval> | null = null
  private last = 0
  /** Fractional beats since the band started. */
  private clock = 0
  private sectionStart = 0
  private taps: number[] = []
  /** Steady taps in a row (the engine's count), and when a bar of them starts the band. */
  private tapRun = 0
  private tapStart: number | null = null
  /** The Stop Accompaniment mode the toggle turns back on. */
  private lastStopAcmp: StopAcmpMode = 'style'
  private now = 0
  private progression = 0
  private messageSeq = 0
  private demo: boolean
  /** Keys the (imaginary) player holds: a left-hand chord and a right-hand melody. */
  private leftHand: number[] = []
  private rightHand: number[] = []
  /** Where the (imaginary) hardware faders physically are: 1–8, master. */
  private hwFaders = [100, 72, 100, 100, 0, 0, 0, 0, 100]
  /** The clocks as the engine anchors them (docs/app-api.md "surface.clock"). */
  private anchor = { key: '', sectionMs: 0, sectionBeats: 0, ledMs: 0, ledBeats: 0, tempo: 0 }
  /** The Chord Looper, as the engine runs it (mock-looper.ts). */
  private looper = new MockLooper(() => this.state.looper)
  private knobs = new MockKnobs()

  /** The hardware faders and the clocks, read now. */
  private hardware(): MockHardware {
    const t = this.state.transport
    const a = this.anchor
    // The LED clock runs free; on a tempo change it re-anchors, carrying on.
    if (t.tempo !== a.tempo) {
      a.ledBeats = a.tempo ? a.ledBeats + ((this.now - a.ledMs) * a.tempo) / 60000 : 0
      a.ledMs = this.now
      a.tempo = t.tempo
    }
    // The section clock re-anchors when the section, its start or the tempo changes.
    const key = `${t.running}:${t.section}:${this.sectionStart}:${t.tempo}`
    if (key !== a.key) {
      a.key = key
      a.sectionMs = this.now
      a.sectionBeats = t.running ? this.clock - this.sectionStart * t.beatsPerBar : 0
    }
    const clock = clockAt({
      atMs: this.now, running: t.running, tempo: t.tempo, beatsPerBar: t.beatsPerBar, bar: 1, beat: 1, phase: 0,
      sectionAnchorMs: a.sectionMs, sectionAnchorBeats: a.sectionBeats, ledAnchorMs: a.ledMs, ledAnchorBeats: a.ledBeats,
    }, this.now)
    return { faders: this.hwFaders, clock }
  }
  private lib: LibraryList = LIBRARY
  private styles: FixtureStyle[] = STYLES
  /** Milliseconds left of a rescan (`rescanLibrary`). */
  private scanLeft = 0
  /** Beats into the audition playing (#21). */
  private auditionBeats = 0
  /** The imported playlists' songs (#89); the mock only has its demo playlist. */
  private chartSongs: (typeof DEMO_SONGS)[] = []
  /** The chart's last bar has played and it has no Ending: stop at the next bar line. */
  private chartEnd = false
  /** Milliseconds left of the fade phase playing (fading in or out, holding). */
  private fadeLeft = 0
  /** Registration Memory and the Playlist (in-memory banks and playlists). */
  private reg: MockRegistration
  /** Multi Pads (mock-multipad.ts). */
  private multiPads = new MockPads(() => this.state.multiPad)
  /** Instrument plugins (mock-plugins.ts). */
  private plugins = new MockPlugins(
    () => this.state,
    (t, e) => this.message(t, e),
  )
  /** The sound library (mock-sound-library.ts). */
  private sound = new MockSoundLibrary(() => this.state)
  /** The sound catalog (mock-sounds.ts). */
  private catalogMock = new MockSounds()

  constructor(opts: MockOptions = {}) {
    this.demo = opts.demo ?? false
    this.state = initialState()
    this.reg = new MockRegistration(STYLES.filter((s) => !s.error).map((s) => ({ path: stylePath(s), name: s.name })), opts.registration ?? true)
    this.reg.fill(this.state)
    if (opts.styles) {
      const big = bigLibrary(opts.styles)
      this.lib = big.lib
      this.styles = big.styles
      this.state.library.count = this.lib.entries.length
      this.state.library.position = this.lib.entries.findIndex((e) => e.id === this.state.style.id)
    }
    if (this.demo) this.demoStart()
    if (opts.chart) {
      this.cmd({ type: 'importCharts', text: 'irealb://demo' })
      this.cmd({ type: 'setChartMode', on: true })
      this.state.message = null
      // The demo plays the chart from its start (straight into bar 1: no Intro).
      if (this.demo) {
        this.stopBand()
        this.cmd({ type: 'setChartIntro', index: null })
        this.startBand()
      }
    }
    derive(this.state, this.lib, this.hardware(), [...this.leftHand, ...this.rightHand])
    this.sound.derive(this.state)
    this.catalogMock.derive(this.state)
    if (!opts.manual) {
      this.last = performance.now()
      this.timer = setInterval(() => {
        const t = performance.now()
        this.advance(t - this.last)
        this.last = t
      }, 1000 / 60)
    }
  }

  private demoStart() {
    const st = this.state
    const t = st.transport
    // Mid-song: Main B, a few bars in, Right 1 + Right 2 layered, a chord held.
    t.running = true
    t.syncStart = false
    t.section = 'Main B'
    t.main = 1
    this.clock = 11 * t.beatsPerBar
    this.sectionStart = 0
    st.keyboardParts[1].on = true
    st.keyboardParts[1].volume = 72
    st.ots.applied = 2
    st.chord.name = 'Am7'
    st.chord.fingered = 'Am7'
    this.leftHand = this.leftVoicing('Am7')
    this.rightHand = [72, 76]
    st.mixer.styleParts[5].waiting = true
    st.mixer.styleParts[5].volume = 58
    // The demo Multi Pad bank, its shaker loop playing and the brass hit in standby.
    this.multiPads.cmd({ type: 'loadMultiPad', id: 0 }, false)
    this.multiPads.cmd({ type: 'triggerMultiPad', pad: 0 }, false)
    this.multiPads.cmd({ type: 'armMultiPad', pad: 3 }, false)
    st.io.lastControl = padPress(MAINS.indexOf('Main B'))
    this.position()
  }

  subscribe(fn: (s: AppState) => void) {
    this.subs.add(fn)
    fn(this.snapshot())
    return () => this.subs.delete(fn)
  }

  library() {
    return Promise.resolve(this.lib)
  }

  sounds() {
    return Promise.resolve(this.catalogMock.catalog(this.state))
  }

  /** No audio: silent meters with no channels, as the engine without its synth. */
  meters() {
    return Promise.resolve({ atMs: this.now, channels: [], master: [0, 0] as [number, number], clips: 0 })
  }

  pluginEditor(part: number, open: boolean) {
    if (!open) return
    const p = this.state.keyboardParts[part & 3]?.plugin
    this.message(p ? `${p.name}'s window opens in the desktop app` : 'the part plays its SoundFont voice', !p)
    this.publish()
  }

  dispose() {
    if (this.timer) clearInterval(this.timer)
    this.subs.clear()
  }

  private snapshot(): AppState {
    // A fresh object each time, as a deserialized snapshot from the engine would be.
    return JSON.parse(JSON.stringify(this.state))
  }

  private publish() {
    // A keyboard part's own plugin patch plays its plugin (the session's sync_part_plugins).
    for (const [part, v] of this.sound.partPlugins()) {
      if (v) this.plugins.cmd({ type: 'setPartPlugin', part, id: v.componentId, state: v.state || null })
      else this.plugins.cmd({ type: 'clearPartPlugin', part })
    }
    this.state.version++
    this.reg.fill(this.state)
    this.looper.publish()
    derive(this.state, this.lib, this.hardware(), [...this.leftHand, ...this.rightHand])
    this.sound.derive(this.state)
    this.catalogMock.derive(this.state)
    this.state.knobs = this.knobs.state(this.state)
    const snap = this.snapshot()
    for (const f of this.subs) f(snap)
  }

  private has(s: string): boolean {
    return this.state.style.sections.includes(s)
  }

  private message(text: string, error = false) {
    this.state.message = { seq: ++this.messageSeq, text, error }
  }

  /** Bar and beat (1-based) within the section playing. */
  private position() {
    const t = this.state.transport
    const bar = Math.floor(this.clock / t.beatsPerBar)
    t.bar = bar - this.sectionStart + 1
    t.beat = (Math.floor(this.clock) % t.beatsPerBar) + 1
  }

  /** Move the clock on by `ms` milliseconds and publish. */
  advance(ms: number) {
    // In steps of at most 20 ms, so no beat or bar boundary is skipped.
    for (let left = ms; left > 0; left -= 20) this.step(Math.min(20, left))
    this.publish()
  }

  private step(ms: number) {
    this.now += ms
    this.stepFade(ms)
    const t = this.state.transport
    // A bar of taps while stopped: the band starts a beat after the last (OM p.46).
    if (this.tapStart !== null && this.now >= this.tapStart) {
      this.tapStart = null
      if (!t.running) this.startBand()
    }
    if (t.running) {
      const before = this.clock
      this.clock += (ms / 60000) * t.tempo
      const bpb = t.beatsPerBar
      if (Math.floor(this.clock) !== Math.floor(before)) this.onBeat()
      if (Math.floor(this.clock / bpb) !== Math.floor(before / bpb)) this.onBar(Math.floor(this.clock / bpb))
      if (t.running) this.position()
    } else if (this.demo && t.syncStart && this.now > 2500 && this.now - ms <= 2500) {
      this.chordArrives('C')
    }
    if (!t.running) this.stepAudition(ms)
    else this.state.soundLibrary.auditioning = null
    this.sound.advance(ms)
    this.catalogMock.advance(ms, t.running)
    this.multiPads.beats((ms / 60000) * t.tempo)
    this.plugins.step(ms)
    if (this.scanLeft > 0) {
      this.scanLeft -= ms
      if (this.scanLeft <= 0) this.state.library.scanning = false
    }
  }

  /** Fade In/Out: a fade in runs out, a fade out stops the band and holds, a hold ends. */
  private stepFade(ms: number) {
    const t = this.state.transport
    if (t.fade === 'off' || t.fade === 'armed') return
    this.fadeLeft -= ms
    if (this.fadeLeft > 0) return
    if (t.fade === 'fadingOut') {
      this.stopBand()
      t.fade = 'holding'
      this.fadeLeft += this.state.styleSettings.fadeHoldMs
    } else t.fade = 'off'
  }

  /** Style Section Reset: the section starts again from its top, now. */
  private resetSection() {
    const t = this.state.transport
    if (!t.running) return
    this.sectionStart = Math.floor(this.clock / t.beatsPerBar)
    this.clock = this.sectionStart * t.beatsPerBar
    this.position()
  }

  private styleSettings(cmd: Extract<AppCmd, { type: `set${string}` | 'stepRetriggerRate' }>) {
    const s = this.state.styleSettings
    const ms = (v: number, max: number) => Math.max(0, Math.min(max, Math.round(v)))
    switch (cmd.type) {
      case 'setMainTiming':
        s.mainTiming = cmd.timing
        break
      case 'setIntroEndingTiming':
        s.introEndingTiming = cmd.timing
        break
      case 'setSyncStopWindow':
        s.syncStopWindowMs = ms(cmd.ms, 5000)
        break
      case 'setFadeInTime':
        s.fadeInMs = ms(cmd.ms, 20000)
        break
      case 'setFadeOutTime':
        s.fadeOutMs = ms(cmd.ms, 20000)
        break
      case 'setFadeHoldTime':
        s.fadeHoldMs = ms(cmd.ms, 5000)
        break
      case 'setSectionReset':
        s.sectionReset = cmd.on
        break
      case 'setRetriggerRate':
        s.retriggerRate = [...RETRIGGER_RATES].reverse().find((r) => r <= Math.max(1, cmd.rate)) ?? 1
        break
      case 'stepRetriggerRate': {
        const i = RETRIGGER_RATES.indexOf(s.retriggerRate as (typeof RETRIGGER_RATES)[number])
        s.retriggerRate = RETRIGGER_RATES[Math.max(0, Math.min(RETRIGGER_RATES.length - 1, (i < 0 ? 3 : i) + Math.sign(cmd.delta)))]
        break
      }
    }
  }

  // ── Style preview (#21): what the engine does ─────────────────────────────
  private get preview(): PreviewState {
    return this.state.preview
  }

  private startAudition(id: number) {
    const s = this.styles[id]
    if (!s) return
    if (this.state.transport.running) {
      this.message('Preview works while the band is stopped; queue the style for the next bar instead', true)
      return
    }
    if (s.error) {
      this.message(`${s.folder}/${s.file}: ${s.error}`, true)
      return
    }
    this.auditionBeats = 0
    this.preview.audition = { id, bar: 1, bars: AUDITION_PROGRESSION.length, chord: AUDITION_PROGRESSION[0] }
  }

  private stepAudition(ms: number) {
    const a = this.preview.audition
    const s = a && this.styles[a.id]
    if (!a || !s) return
    this.auditionBeats += (ms / 60000) * s.tempo
    const bar = Math.floor(this.auditionBeats / beatsPerBar([s.timeSignature[0], s.timeSignature[1]]))
    if (bar >= a.bars) this.preview.audition = null
    else if (bar + 1 !== a.bar) this.preview.audition = { ...a, bar: bar + 1, chord: AUDITION_PROGRESSION[bar] }
  }

  private queueStyle(id: number) {
    if (this.state.transport.running) this.preview.queued = id
    else this.loadStyle(id)
  }

  private onBeat() {
    const t = this.state.transport
    if (this.demo) this.melody()
    const c = this.state.chart
    if (c.on && c.song && c.bar !== null && t.section && MAINS.concat(FILLS, [BREAK]).includes(t.section)) {
      this.chartChord(chordAt(c.song, c.bar, Math.floor(this.clock) % t.beatsPerBar))
    }
    if (t.queued && FILLS.includes(t.queued)) {
      t.section = t.queued
      t.queued = null
      this.sectionStart = Math.floor(this.clock / t.beatsPerBar)
    }
  }

  private onBar(bar: number) {
    const t = this.state.transport
    if (this.chartEnd) {
      this.stopBand()
      return
    }
    this.multiPads.bar()
    const q = this.preview.queued
    if (q !== null) {
      this.preview.queued = null
      this.loadStyle(q)
    }
    const played = bar - this.sectionStart
    const main = MAINS[t.main]
    if (t.section && FILLS.includes(t.section)) {
      this.enter(t.queued && MAINS.includes(t.queued) ? t.queued : main, bar)
      t.queued = null
    } else if (t.queued) {
      const q = t.queued
      t.queued = null
      this.enter(q, bar)
    } else if (t.section && !MAINS.includes(t.section) && played >= sectionBars(t.section)) {
      if (ENDINGS.includes(t.section)) {
        this.stopBand()
        return
      }
      this.enter(main, bar)
    }
    if (this.chartPlaying()) {
      this.chartBar()
      this.looper.onBar(bar, this.state.chord.fingered || null)
      return
    }
    if (this.demo) this.demoBar(bar)
    // The style follows a new chord every other bar (the imaginary left hand).
    if (t.running && bar % 2 === 0) this.keyboardChord(PROGRESSION[this.progression++ % PROGRESSION.length])
    const loop = this.looper.onBar(bar, this.state.chord.fingered || null)
    if (loop && loop !== this.state.chord.fingered) this.chordArrives(loop)
  }

  // ── iReal chart player (#89): what engine/chart.rs does, bar by bar ─────────
  private chartPlaying(): boolean {
    const c = this.state.chart
    return c.on && !!c.song && this.state.transport.running
  }

  /** A bar line: the chart moves on a bar (not in an Intro or Ending) and queues its next section. */
  private chartBar() {
    const t = this.state.transport
    const c = this.state.chart
    const song = c.song!
    if (!t.section || INTROS.includes(t.section) || ENDINGS.includes(t.section)) return
    const i = c.bar === null ? 0 : nextBar(c, c.bar)
    if (i === null) return
    c.bar = i
    this.chartChord(chordAt(song, i, 0))
    const n1 = nextBar(c, i)
    if (n1 === null) {
      const e = c.ending !== null ? ENDINGS[c.ending] : null
      if (e && this.has(e)) t.queued = e
      else this.chartEnd = true
      return
    }
    const next = song.bars[n1]
    if (next.sectionStart) {
      // The mock plays fills from the next beat: the rest of this bar leads in.
      t.main = next.main
      t.queued = t.autoFill && this.has(FILLS[next.main]) ? FILLS[next.main] : MAINS[next.main]
    }
  }

  private chartChord(name: string | null) {
    if (!name) return
    this.state.chord.fingered = name
    this.state.chord.name = transposeChord(name, this.state.chord.transposeKeyboard)
  }

  /** Choose a song: its chart, suggested style (loaded with Auto Style) and, stopped, its tempo. */
  private selectChart(playlist: number, song: number, fresh = true) {
    const def = this.chartSongs[playlist]?.[song]
    const c = this.state.chart
    if (!def) {
      this.message(`no song ${song} in playlist ${playlist}`, true)
      return
    }
    c.selected = [playlist, song]
    c.song = songState(def, c.choruses)
    if (c.bar !== null) c.bar = Math.min(c.bar, c.song.bars.length - 1)
    if (!fresh) return
    c.loop = null
    const words = styleWords(def.style)
    const hit = this.lib.entries.find((e) => e.status === 'ok' && words.some((w) => `${e.name} ${e.folder}`.toLowerCase().includes(w)))
    c.suggestedStyle = hit?.id ?? null
    if (c.autoStyle && hit && hit.id !== this.state.style.id) this.loadStyle(hit.id)
    if (!this.state.transport.running && def.tempo) this.state.transport.tempo = def.tempo
  }

  private chartCmd(cmd: Extract<AppCmd, { type: `${string}Chart${string}` | 'importCharts' | 'importChartFile' }>) {
    const c = this.state.chart
    switch (cmd.type) {
      case 'importCharts':
      case 'importChartFile':
        // The mock can't decode iReal links; it imports its demo playlist instead.
        this.chartSongs.push(DEMO_SONGS)
        c.playlists.push({ name: DEMO_PLAYLIST, songs: DEMO_SONGS.map(info) })
        if (!c.selected) this.selectChart(c.playlists.length - 1, 0)
        this.message(`Imported ${DEMO_SONGS.length} songs (1 playlist)`)
        break
      case 'selectChart':
        this.selectChart(cmd.playlist, cmd.song)
        break
      case 'stepChart': {
        if (!c.selected) break
        const [p, s] = c.selected
        const to = Math.max(0, Math.min(c.playlists[p].songs.length - 1, s + cmd.delta))
        if (to !== s) this.selectChart(p, to)
        break
      }
      case 'removeChartPlaylist':
        if (cmd.playlist >= c.playlists.length) break
        c.playlists.splice(cmd.playlist, 1)
        this.chartSongs.splice(cmd.playlist, 1)
        if (c.selected?.[0] === cmd.playlist) Object.assign(c, { selected: null, song: null, on: false, loop: null, suggestedStyle: null, bar: null })
        else if (c.selected && c.selected[0] > cmd.playlist) c.selected = [c.selected[0] - 1, c.selected[1]]
        break
      case 'setChartMode':
      case 'toggleChartMode': {
        const on = cmd.type === 'setChartMode' ? cmd.on : !c.on
        if (on && !c.song) {
          this.message('Import an iReal Pro chart first', true)
          break
        }
        // Only one of the chart and the Chord Looper gives the chords: chart mode on stops a loop.
        const m = this.state.looper.mode
        if (on && (m === 'looping' || m === 'loopArmed')) this.looper.onOff()
        c.on = on
        if (!on) c.bar = null
        break
      }
      case 'setChartChoruses':
        c.choruses = Math.max(1, Math.min(99, Math.round(cmd.choruses)))
        if (c.selected) this.selectChart(c.selected[0], c.selected[1], false)
        // Fewer choruses: a loop past the new end goes.
        if (c.loop && c.loop[1] > (c.song?.bars.length ?? 0)) c.loop = null
        break
      case 'setChartLoop': {
        const n = c.song?.bars.length ?? 0
        if (cmd.range && !(cmd.range[0] < cmd.range[1] && cmd.range[1] <= n)) this.message(`no bars ${cmd.range[0] + 1}-${cmd.range[1]} in the chart`, true)
        else c.loop = cmd.range
        break
      }
      case 'setChartIntro':
        c.intro = cmd.index
        break
      case 'setChartEnding':
        c.ending = cmd.index
        break
      case 'setChartAutoStyle':
        c.autoStyle = cmd.on
        break
    }
  }

  /** A chord from the keyboard: the Chord Looper ignores it while it loops, records it
   *  while it records. */
  private keyboardChord(chord: string) {
    const bpb = this.state.transport.beatsPerBar
    if (this.looper.keyboardChord(chord, Math.floor(this.clock / bpb), (Math.floor(this.clock) % bpb) + 1)) this.chordArrives(chord)
  }


  private demoBar(bar: number) {
    const t = this.state.transport
    // Every 8 bars, queue the next Main (half a bar early, so the flashing shows).
    if (bar % 8 === 6 && !t.queued) {
      const index = (t.main + 1) % 4
      this.cmd({ type: 'main', index })
      this.state.io.lastControl = padPress(index)
    }
    // A pattern's volume change moves a Style fader away from the hardware (soft takeover).
    if (bar % 8 === 0) {
      const p = this.state.mixer.styleParts[(bar / 8) % 8 | 0]
      p.volume = Math.max(40, Math.min(127, p.volume + (bar % 16 === 0 ? -14 : 14)))
      p.waiting = true
    }
  }

  private enter(s: string, bar: number) {
    const t = this.state.transport
    if (ENDINGS.includes(s) && !(t.section && ENDINGS.includes(t.section))) this.multiPads.endingStarted()
    t.section = s
    this.sectionStart = bar
    const m = MAINS.indexOf(s)
    if (m >= 0) {
      t.main = m
      // OTS Link Timing "At Main Section Change": as the Main starts playing.
      const ots = this.state.ots
      if (ots.link && ots.linkTiming === 'mainChange' && m < ots.settings.length && ots.applied !== m + 1) this.recallOts(m)
    }
  }

  /** The demo's right hand: a chord tone per beat above the split, resting on the last beat. */
  private melody() {
    const beat = Math.floor(this.clock)
    const bpb = this.state.transport.beatsPerBar
    const { tones } = chordTones(this.state.chord.fingered)
    if (!tones.length || beat % bpb === bpb - 1) {
      this.rightHand = []
      return
    }
    const tone = tones[(beat * 3) % tones.length]
    const top = 72 + tone
    this.rightHand = beat % bpb === 0 ? [60 + tones[0] + (tones[0] < 5 ? 12 : 0), top] : [top]
  }

  /** The demo's left hand: `chord` in close position, root at or just below the split. */
  private leftVoicing(chord: string) {
    const { tones } = chordTones(chord)
    const split = this.state.chord.split
    if (!tones.length) return []
    const root = split - ((((split - tones[0]) % 12) + 12) % 12)
    return tones.map((pc) => {
      const n = root + ((pc - tones[0] + 12) % 12)
      return n > split ? n - 12 : n
    }).sort((a, b) => a - b)
  }

  private chordArrives(chord: string) {
    const t = this.state.transport
    if (this.demo) this.leftHand = this.leftVoicing(chord)
    const k = this.state.chord.transposeKeyboard
    this.state.chord.name = transposeChord(chord, k)
    this.state.chord.fingered = chord
    if (!t.running && t.syncStart) this.startBand()
    this.multiPads.chord(t.running)
  }

  private startBand() {
    this.tapStart = null
    const t = this.state.transport
    const c = this.state.chart
    if (t.fade === 'armed') {
      t.fade = 'fadingIn'
      this.fadeLeft = this.state.styleSettings.fadeInMs
    } else if (t.fade === 'holding') t.fade = 'off'
    this.preview.audition = null
    this.chartEnd = false
    if (c.on && c.song) {
      // The chart's Intro (unless one is armed), its first Main and first chord.
      if (t.pendingIntro === null && c.intro !== null && this.has(INTROS[c.intro])) t.pendingIntro = c.intro
      t.main = c.song.bars[0].main
      c.bar = null
      this.chartChord(chordAt(c.song, 0, 0))
    }
    t.running = true
    t.syncStart = false
    this.clock = 0
    this.sectionStart = 0
    t.section = t.pendingIntro !== null && this.has(INTROS[t.pendingIntro]) ? INTROS[t.pendingIntro] : MAINS[t.main]
    t.pendingIntro = null
    t.queued = null
    this.position()
    if (this.chartPlaying() && MAINS.includes(t.section ?? '')) this.chartBar()
    // Bar 1: a Chord Looper armed starts recording (with this chord) or looping here.
    const loop = this.looper.onBar(0, this.state.chord.fingered || null)
    if (loop && loop !== this.state.chord.fingered) this.chordArrives(loop)
    this.multiPads.bandStarted()
  }

  private stopBand() {
    const t = this.state.transport
    if (t.fade === 'fadingIn' || t.fade === 'fadingOut') t.fade = 'off'
    t.ritardando = false
    this.rightHand = []
    this.state.chart.bar = null
    this.chartEnd = false
    // A style queued for the next bar loads when the band stops first.
    const q = this.preview.queued
    this.preview.queued = null
    if (q !== null) this.loadStyle(q)
    const was = t.running
    t.running = false
    t.section = null
    t.queued = null
    t.bar = 1
    t.beat = 1
    this.looper.onStop()
    if (was) this.multiPads.bandStopped()
  }

  /** Part `part` was given a GM voice (setPartVoice, Voice −/+, an OTS voice, #179): a
   * plugin picked for it ends, and its own library patch goes (with that patch's plugin). */
  private gmVoice(part: number) {
    if (this.sound.ownPlugin(part) && this.state.keyboardParts[part & 3].plugin) this.plugins.cmd({ type: 'clearPartPlugin', part })
    this.sound.partVoice(part)
  }

  private recallOts(n: number) {
    const panel = this.state.mixer.faderPage === 'panel'
    this.state.ots.settings[n].parts.forEach((o, i) => {
      const p = this.state.keyboardParts[i]
      if (o.program !== null) {
        p.program = o.program
        this.gmVoice(i)
      }
      p.on = o.on
      p.octave = o.octave
      if (p.volume !== o.volume) p.waiting = panel
      p.volume = o.volume
    })
    this.state.ots.applied = n + 1
    // OTS turns Sync Start on (ACMP is always on): the next chord starts a stopped band.
    if (!this.state.transport.running) this.state.transport.syncStart = true
  }

  /** The Main `from` + `step` the style has (Fill Up / Down), or `from` at the end of the row. */
  private neighbourMain(from: number, step: number): number {
    for (let j = from + step; j >= 0 && j < 4; j += step) if (this.has(MAINS[j])) return j
    return from
  }

  /** Fill Up / Down / Self: a fill, then Main `target`. Stopped: selects it. */
  private fillTo(target: number) {
    const t = this.state.transport
    if (!t.running) {
      t.main = target
      return
    }
    t.queued = this.has(FILLS[target]) ? FILLS[target] : MAINS[target]
    t.main = target
    if (this.state.ots.link && this.state.ots.linkTiming === 'immediate' && target < this.state.ots.settings.length) this.recallOts(target)
  }

  private loadStyle(id: number) {
    const s = this.styles[id]
    if (!s) return
    // Loading hands over cleanly from an audition: it ends, the band stays as it was.
    this.preview.audition = null
    if (s.error) {
      this.message(`${s.folder}/${s.file}: ${s.error}`, true)
      return
    }
    const st = this.state
    const t = st.transport
    st.style = styleState(s)
    // Change Behavior: Lock keeps, Hold keeps while playing, Reset takes the new style's.
    const resets = (rule: string) => rule === 'reset' || (rule === 'hold' && !t.running)
    if (resets(st.styleChange.tempo)) t.tempo = s.tempo
    if (resets(st.styleChange.parts)) for (const p of st.mixer.styleParts) p.on = true
    const set = st.styleChange.sectionSet
    if (!t.running && set !== null) t.main = [0, 1, 2, 3].map((d) => [set - d, set + d]).flat().find((j) => j >= 0 && j < 4 && s.sections.includes(MAINS[j])) ?? set
    t.beatsPerBar = beatsPerBar(st.style.timeSignature)
    st.ots = { settings: otsSettings(s.ots), applied: 0, link: st.ots.link, linkTiming: st.ots.linkTiming }
    if (st.ots.link && t.main < s.ots) this.recallOts(t.main)
    for (const p of st.mixer.styleParts) {
      p.volume = 100
      p.waiting = st.mixer.faderPage === 'style'
    }
    if (t.section && !this.has(t.section)) t.section = MAINS.find((m) => this.has(m)) ?? null
    st.library.position = this.lib.entries.findIndex((e) => e.id === id)
    st.message = null
  }

  send(cmd: AppCmd) {
    this.cmd(cmd)
    this.publish()
  }

  private cmd(cmd: AppCmd) {
    if (this.reg.handles(cmd)) {
      this.reg.cmd(cmd, {
        state: this.state,
        command: (c) => this.cmd(c),
        message: (text, error) => this.message(text, error),
        findStyle: (path, name) =>
          this.lib.entries.find((e) => e.path === path)?.path ??
          this.lib.entries.find((e) => e.path.split('/').pop() === path.split('/').pop() || e.name === name)?.path ??
          null,
      })
      return
    }
    const st = this.state
    const t = st.transport
    const c = st.chord
    const vol = (v: number) => Math.max(0, Math.min(127, Math.round(v)))
    const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v))
    switch (cmd.type) {
      case 'startStop':
        if (t.running) this.stopBand()
        else this.startBand()
        break
      case 'stop':
        this.tapStart = null
        if (t.running) this.stopBand()
        break
      case 'intro':
        if (!this.has(INTROS[cmd.index])) break
        if (t.running) t.queued = INTROS[cmd.index]
        else t.pendingIntro = t.pendingIntro === cmd.index ? null : cmd.index
        break
      case 'main': {
        const m = MAINS[cmd.index]
        if (!this.has(m)) break
        if (!t.running) {
          t.main = cmd.index
          if (st.ots.link && cmd.index < st.ots.settings.length) this.recallOts(cmd.index)
        } else if (t.section === m) {
          t.queued = FILLS[cmd.index]
        } else if (t.autoFill && t.main !== cmd.index) {
          t.queued = FILLS[t.main]
          t.main = cmd.index
        } else t.queued = m
        if (t.running && st.ots.link && st.ots.linkTiming === 'immediate' && cmd.index < st.ots.settings.length) this.recallOts(cmd.index)
        break
      }
      case 'fillUp':
        this.fillTo(this.neighbourMain(t.main, 1))
        break
      case 'fillDown':
        this.fillTo(this.neighbourMain(t.main, -1))
        break
      case 'fillSelf':
        if (t.running) this.fillTo(t.main)
        break
      case 'fillBreak':
        if (t.running && this.has(BREAK)) t.queued = BREAK
        break
      case 'toggleHalfBarFill':
      case 'setHalfBarFill':
        t.halfBarFill = cmd.type === 'setHalfBarFill' ? cmd.on : !t.halfBarFill
        break
      case 'break':
        if (t.running && this.has(BREAK)) t.queued = BREAK
        break
      case 'fill': {
        // The same as Fill Down / Self / Up.
        const d = Math.sign(cmd.delta)
        this.cmd({ type: d < 0 ? 'fillDown' : d > 0 ? 'fillUp' : 'fillSelf' })
        break
      }
      case 'ending':
        // The Ending playing, pressed again: ritardando.
        if (t.running && t.section === ENDINGS[cmd.index]) t.ritardando = true
        else if (t.running && this.has(ENDINGS[cmd.index])) t.queued = ENDINGS[cmd.index]
        break
      case 'toggleFade':
        if (!t.running) t.fade = t.fade === 'armed' ? 'off' : 'armed'
        else if (t.fade !== 'fadingOut') {
          t.fade = 'fadingOut'
          this.fadeLeft = st.styleSettings.fadeOutMs
        }
        break
      case 'sectionReset':
        this.resetSection()
        break
      case 'toggleRetrigger':
        t.retrigger = !t.retrigger
        break
      case 'setMainTiming':
      case 'setIntroEndingTiming':
      case 'setSyncStopWindow':
      case 'setFadeInTime':
      case 'setFadeOutTime':
      case 'setFadeHoldTime':
      case 'setSectionReset':
      case 'setRetriggerRate':
      case 'stepRetriggerRate':
        this.styleSettings(cmd)
        break
      case 'toggleSyncStart':
        if (t.running) this.stopBand()
        t.syncStart = !t.syncStart
        break
      case 'toggleSyncStop':
        if (t.syncStopAvailable) t.syncStop = !t.syncStop
        else this.message('Sync Stop is not available with the Full Keyboard fingering types', true)
        break
      case 'toggleAutoFill':
        t.autoFill = !t.autoFill
        break
      case 'toggleStopAcmp':
      case 'setStopAcmp': {
        const mode = cmd.type === 'setStopAcmp' ? cmd.mode : t.stopAcmpMode === 'off' ? this.lastStopAcmp : 'off'
        if (mode !== 'off') this.lastStopAcmp = mode
        t.stopAcmpMode = mode
        t.stopAcmp = mode !== 'off'
        break
      }
      case 'tapTempo': {
        if (t.running && st.styleSettings.sectionReset) {
          this.resetSection()
          break
        }
        // As the engine: taps up to 12.5 s apart count (down to 5 BPM); a jump in the
        // interval by more than half starts a fresh average from the tap before.
        const last = this.taps[this.taps.length - 1]
        if (last !== undefined && this.now - last > 12500) {
          this.taps = []
          this.tapRun = 0
        } else if (this.taps.length >= 2) {
          const r = (this.now - last) / Math.max(1, last - this.taps[this.taps.length - 2])
          if (r > 1.5 || r < 1 / 1.5) {
            this.taps = [last]
            this.tapRun = 1
          }
        }
        this.taps = [...this.taps, this.now].slice(-4)
        this.tapRun++
        if (this.taps.length >= 2) {
          const avg = (this.taps[this.taps.length - 1] - this.taps[0]) / (this.taps.length - 1)
          if (avg > 0) t.tempo = clamp(Math.round(60000 / avg), 5, 500)
        }
        // Stopped, a bar of steady taps starts the band a beat after the last one.
        this.tapStart = !t.running && this.tapRun >= Math.max(1, t.beatsPerBar) ? this.now + 60000 / t.tempo : null
        break
      }
      case 'tempoUp':
        t.tempo = clamp(t.tempo + 1, 5, 500)
        break
      case 'tempoDown':
        t.tempo = clamp(t.tempo - 1, 5, 500)
        break
      case 'resetTempo':
        t.tempo = st.style.tempo
        break
      case 'setTempo':
        t.tempo = clamp(Math.round(cmd.bpm), 5, 500)
        break
      case 'setStyleSolo':
        st.mixer.styleSolo = cmd.part === null ? null : cmd.part & 7
        break
      case 'setPartSolo':
        st.mixer.partSolo = cmd.part === null ? null : cmd.part & 3
        break
      case 'styleTrackMute': {
        const order = cmd.order === 'a' ? [1, 0, 2, 3, 4, 5, 6, 7] : [3, 4, 5, 2, 6, 7, 0, 1]
        const n = 1 + Math.floor((clamp(cmd.value, 0, 127) * 7 + 63) / 127)
        st.mixer.styleParts.forEach((p, i) => (p.on = order.slice(0, n).includes(i)))
        break
      }
      case 'looperRec':
        if (this.looper.rec(t.running)) t.syncStart = true
        break
      case 'looperOnOff': {
        // A loop about to arm turns chart mode off first (only one of them gives the chords).
        const l = this.state.looper
        if (this.state.chart.on && (l.mode === 'recording' || (l.mode === 'off' && l.hasData))) {
          this.state.chart.on = false
          this.state.chart.bar = null
        }
        this.looper.onOff()
        break
      }
      case 'selectLooperMemory': {
        const err = this.looper.select(cmd.index & 7)
        if (err) this.message(err, true)
        break
      }
      case 'storeLooperMemory': {
        const err = this.looper.store(cmd.index & 7)
        if (err) this.message(err, true)
        break
      }
      case 'clearLooperMemory':
        this.looper.clear(cmd.index & 7)
        break
      case 'newLooperBank':
        this.looper.newBank()
        break
      case 'saveLooperBank': {
        const err = this.looper.saveBank(cmd.name, cmd.overwrite ?? false)
        if (err) this.message(err, true)
        else this.message(`Saved Chord Looper bank ${this.state.looper.bankName}`)
        break
      }
      case 'loadLooperBank': {
        const err = this.looper.loadBank(cmd.path)
        if (err) this.message(err, true)
        break
      }
      case 'toggleMetronome':
      case 'setMetronome':
        st.metronome.on = cmd.type === 'setMetronome' ? cmd.on : !st.metronome.on
        break
      case 'setMetronomeVolume':
        st.metronome.volume = vol(cmd.volume)
        break
      case 'setMetronomeBell':
        st.metronome.bell = cmd.on
        break
      case 'toggleStylePart':
        st.mixer.styleParts[cmd.part].on = !st.mixer.styleParts[cmd.part].on
        break
      case 'setStylePartVolume':
        st.mixer.styleParts[cmd.part].volume = vol(cmd.volume)
        st.mixer.styleParts[cmd.part].waiting = false
        break
      case 'setFingering':
        c.fingering = cmd.fingering
        break
      case 'nextFingering': {
        const i = FINGERINGS.findIndex((f) => f.id === c.fingering)
        c.fingering = FINGERINGS[(i + 1) % FINGERINGS.length].id
        break
      }
      case 'setUpper':
      case 'toggleUpper':
        c.upper = cmd.type === 'setUpper' ? cmd.on : !c.upper
        if (c.upper) c.manualBass = true
        break
      case 'setManualBass':
      case 'toggleManualBass':
        if (!c.upper) this.message('Manual Bass is only available with chord detection Upper', true)
        else c.manualBass = cmd.type === 'setManualBass' ? cmd.on : !c.manualBass
        break
      case 'setSplit':
        c.split = clamp(cmd.note, 24, 96)
        break
      case 'moveSplit':
        c.split = clamp(c.split + cmd.delta, 24, 96)
        break
      case 'setTranspose':
        c.transposeKeyboard = clamp(cmd.keyboard, -12, 12)
        c.transposeMaster = clamp(cmd.master, -12, 12)
        break
      case 'stepTranspose':
        c.transposeKeyboard = clamp(c.transposeKeyboard + cmd.keyboard, -12, 12)
        c.transposeMaster = clamp(c.transposeMaster + cmd.master, -12, 12)
        break
      case 'resetTranspose':
        c.transposeKeyboard = 0
        c.transposeMaster = 0
        break
      case 'setChordSettle':
        c.settleMs = clamp(cmd.ms, 0, CHORD_SETTLE_MAX_MS)
        break
      case 'setLeftHold':
        c.leftHold = cmd.on
        break
      case 'toggleLeftHold':
        c.leftHold = !c.leftHold
        break
      case 'setPartOn':
      case 'togglePart': {
        const p = st.keyboardParts[cmd.part]
        const on = cmd.type === 'setPartOn' ? cmd.on : !p.on
        if (cmd.part === 3 && !on && c.upper && c.manualBass) this.message('Left plays the bass while Manual Bass is on', true)
        else p.on = on
        break
      }
      case 'selectPart':
        st.keyboardParts.forEach((p, i) => (p.selected = i === cmd.part))
        break
      case 'setPartVoice':
        st.keyboardParts[cmd.part].program = cmd.program & 127
        this.gmVoice(cmd.part)
        break
      case 'stepVoice': {
        const p = st.keyboardParts.find((x) => x.selected) ?? st.keyboardParts[0]
        p.program = (p.program + cmd.delta + 128) % 128
        this.gmVoice(st.keyboardParts.indexOf(p))
        break
      }
      case 'setPartVolume':
        st.keyboardParts[cmd.part].volume = vol(cmd.volume)
        st.keyboardParts[cmd.part].waiting = false
        break
      case 'setPartOctave':
        st.keyboardParts[cmd.part].octave = clamp(cmd.octave, -2, 2)
        break
      case 'setPartPan':
        st.keyboardParts[cmd.part].pan = vol(cmd.pan)
        break
      case 'setPartSend':
        st.keyboardParts[cmd.part][cmd.send] = vol(cmd.value)
        break
      case 'setFaderPage':
      case 'toggleFaderPage': {
        const page = cmd.type === 'setFaderPage' ? cmd.page : st.mixer.faderPage === 'panel' ? 'style' : 'panel'
        if (page === st.mixer.faderPage) break
        st.mixer.faderPage = page
        // The hardware faders are wherever they were: every level on the new page waits.
        for (const p of page === 'panel' ? st.keyboardParts : st.mixer.styleParts) p.waiting = true
        if (page === 'panel') st.mixer.styleVolumeWaiting = st.mixer.multiPadVolumeWaiting = true
        break
      }
      case 'setPadPage':
        st.pads.page = cmd.page
        break
      case 'cyclePadPage': {
        const i = PAD_PAGES.findIndex((p) => p.id === st.pads.page)
        st.pads.page = PAD_PAGES[(((i + cmd.delta) % PAD_PAGES.length) + PAD_PAGES.length) % PAD_PAGES.length].id
        break
      }
      case 'setStyleVolume':
        st.mixer.styleVolume = vol(cmd.volume)
        st.mixer.styleVolumeWaiting = false
        break
      case 'setMultiPadVolume':
        st.mixer.multiPadVolume = vol(cmd.volume)
        st.mixer.multiPadVolumeWaiting = false
        break
      case 'setMasterVolume':
        st.mixer.master = vol(cmd.volume)
        st.mixer.masterWaiting = false
        break
      case 'recallOts':
        if (cmd.index < st.ots.settings.length) this.recallOts(cmd.index)
        break
      case 'setOtsLink':
      case 'toggleOtsLink':
        st.ots.link = cmd.type === 'setOtsLink' ? cmd.on : !st.ots.link
        break
      case 'setOtsLinkTiming':
        st.ots.linkTiming = cmd.timing
        break
      case 'setTempoChange':
        st.styleChange.tempo = cmd.rule
        break
      case 'setPartsChange':
        st.styleChange.parts = cmd.rule
        break
      case 'setSectionSet':
        st.styleChange.sectionSet = cmd.section === null ? null : clamp(cmd.section, 0, 3)
        break
      case 'toggleStyleTempoLock':
      case 'toggleStyleTempoHold': {
        const to = cmd.type === 'toggleStyleTempoLock' ? 'lock' : 'hold'
        st.styleChange.tempo = st.styleChange.tempo === 'reset' ? to : 'reset'
        break
      }
      case 'loadStyle':
        this.loadStyle(cmd.id)
        break
      case 'loadStylePath': {
        const e = this.lib.entries.find((x) => x.path === cmd.path)
        if (e) this.loadStyle(e.id)
        else this.message(`${cmd.path}: not found`, true)
        break
      }
      case 'stepStyle': {
        const entries = this.lib.entries
        const n = entries.length
        let i = st.library.position
        for (let k = 0; k < n; k++) {
          i = (((i + cmd.delta) % n) + n) % n
          if (entries[i].status === 'ok') break
        }
        this.loadStyle(entries[i].id)
        break
      }
      case 'auditionStyle':
        this.startAudition(cmd.id)
        break
      case 'stopAudition':
        this.preview.audition = null
        break
      case 'queueStyle':
        this.queueStyle(cmd.id)
        break
      case 'setSynthMuted':
      case 'toggleSynthMute':
        if (st.io.synth) st.io.synth.muted = cmd.type === 'setSynthMuted' ? cmd.on : !st.io.synth.muted
        break
      case 'setAudioOutput':
        if (st.io.synth && cmd.first + 1 < st.io.synth.channels) st.io.synth.outputPair = [cmd.first + 1, cmd.first + 2]
        break
      case 'nextAudioOutput':
        if (st.io.synth) {
          const next = st.io.synth.outputPair[1] + 1
          st.io.synth.outputPair = next < st.io.synth.channels ? [next, next + 1] : [1, 2]
        }
        break
      case 'setSoundFont':
      case 'setDefaultSoundSet': {
        if (cmd.file !== null && !st.io.soundFonts.includes(cmd.file)) {
          this.message(`no SoundFont ${cmd.file} in the SoundFont folder`, true)
          break
        }
        st.io.defaultSoundSet = cmd.file
        const play = cmd.file ?? st.io.autoSoundSet
        if (play) {
          st.io.soundFontFile = play
          if (st.io.synth) st.io.synth.soundFont = play.replace(/\.sf2$/i, '')
        }
        break
      }
      case 'setMidiInputs': {
        st.io.allInputs = cmd.all
        for (const s of st.io.sources) s.listening = s.pads || cmd.all || cmd.names.some((n) => n && s.name.includes(n))
        st.io.inputs = st.io.sources.filter((s) => s.listening).map((s) => (s.pads ? `${s.name} (pads)` : s.name))
        break
      }
      case 'setPaletteLeds':
        st.pads.paletteLeds = cmd.on
        break
      case 'setAudioBuffer':
        if (!st.io.synth) this.message('the synth is off', true)
        else st.io.synth.bufferFrames = cmd.frames
        break
      case 'rescanLibrary':
        st.library.scanning = true
        this.scanLeft = RESCAN_MS
        break
      case 'toggleHarmonyArp':
      case 'setHarmonyArpOn':
      case 'setHarmonyType':
      case 'setArpPattern':
      case 'stepHarmonyArpType':
      case 'setHarmonyVolume':
      case 'setHarmonySpeed':
      case 'setHarmonyAssign':
      case 'setChordNoteOnly':
      case 'setTouchLimit':
      case 'setArpQuantize':
      case 'setArpHold':
      case 'toggleArpHold':
      case 'setArpPedalHold':
      case 'toggleArpPedalHold':
      case 'setArpVelocity':
      case 'setArpKeepKeyOn': {
        // A fresh object, so the published snapshots never share it.
        st.harmonyArp = { ...st.harmonyArp, arp: { ...st.harmonyArp.arp } }
        const err = harmonyArpCmd(st.harmonyArp, cmd)
        if (err) this.message(err, true)
        break
      }
      case 'panic':
        this.stopBand()
        this.multiPads.panic()
        Object.assign(st.controllers, { sustain: false, sostenuto: false, soft: false })
        // As the engine's reset: the pedals count as up, and the control-side switches a
        // Hold pedal was keeping on go off (`pump_pedal_releases`).
        for (const p of st.controllers.pedals) {
          const f = resetRelease(p, p.down)
          p.down = false
          const set = f && functionSet(f, false)
          if (set) this.cmd(set)
        }
        this.message('All notes off')
        break
      case 'setPedal': {
        const p = st.controllers.pedals[cmd.pedal]
        if (!p) {
          this.message(`there is no pedal ${cmd.pedal + 1}`, true)
          break
        }
        const why = cmd.cc === null ? null : pedalCcRefused(cmd.cc)
        if (why) {
          this.message(`a pedal can't use CC ${cmd.cc}: it is ${why}`, true)
          break
        }
        const sw = { sustain: 'sustain', sostenuto: 'sostenuto', soft: 'soft' } as const
        type Sw = keyof typeof sw
        // As the engine: another pedal on the switch keeps it on (Hold A held, Hold B up).
        const keptOn = (f: string) =>
          st.controllers.pedals.some((q, j) => j !== cmd.pedal && q.function === f && (q.controlType === 'holdA' ? q.down : q.controlType === 'holdB' && !q.down))
        const rebound = p.function !== cmd.function || p.cc !== cmd.cc
        const old = { cc: p.cc, function: p.function, controlType: p.controlType }
        const oldDown = p.down
        if (rebound) {
          // What the old function drove lets go, and the pedal counts as up.
          if (p.function in sw && !keptOn(p.function)) st.controllers[p.function as Sw] = false
          p.down = false
        }
        const typeChanged = p.controlType !== cmd.controlType
        Object.assign(p, { cc: cmd.cc, function: cmd.function, controlType: cmd.controlType, reverse: cmd.reverse, range: cmd.range })
        // Hold A / Hold B follow the pedal's position: Hold B picked with the pedal up is on.
        if (p.function in sw && p.controlType !== 'toggle' && (rebound || typeChanged)) {
          const on = (p.controlType === 'holdB') !== p.down
          if (on || !keptOn(p.function)) st.controllers[p.function as Sw] = on
        }
        // Kbd Harmony/Arpeggio and Arpeggio Hold: the control side keeps them, so it sets
        // them where the new setup puts them (`controllers::control_switch_sets`).
        for (const [f, on] of controlSwitchSets(old, p, oldDown, p.down)) {
          const set = functionSet(f, on)
          if (set) this.cmd(set)
        }
        break
      }
      case 'learnPedal':
        // No keyboard here: the mock "hears" the Launchkey's sustain jack (CC 64) at once.
        if (cmd.pedal !== null && st.controllers.pedals[cmd.pedal]) st.controllers.pedals[cmd.pedal].cc = 64
        st.controllers.learning = null
        break
      case 'setPartControllers':
        Object.assign(st.controllers.parts[cmd.part & 3], { sustain: cmd.sustain, pitchBend: cmd.pitchBend, modulation: cmd.modulation })
        break
      case 'setBendRange':
        st.controllers.parts[cmd.part & 3].bendRange = clamp(cmd.semitones, 0, 12)
        break
      case 'triggerFunction': {
        const info = functionInfo(cmd.function)
        if (!info || !info.available) {
          this.message(`${info?.name ?? cmd.function} is not in yahaha yet`, true)
          break
        }
        if (isPedalSwitch(cmd.function)) {
          st.controllers[cmd.function] = !st.controllers[cmd.function]
        } else if (info.kind === 'continuous') {
          this.message(`${info.name} needs a foot controller (an expression pedal)`, true)
        } else if (cmd.function === 'otsNext' || cmd.function === 'otsPrev') {
          const n = st.ots.settings.length
          if (n) {
            const a = st.ots.applied
            this.cmd({ type: 'recallOts', index: cmd.function === 'otsNext' ? a % n : a ? (a + n - 2) % n : n - 1 })
          }
        } else {
          const run = functionCmd(cmd.function, c)
          if (run) this.cmd(run)
        }
        break
      }
      case 'clearMessage':
        st.message = null
        break
      case 'importCharts':
      case 'importChartFile':
      case 'selectChart':
      case 'stepChart':
      case 'removeChartPlaylist':
      case 'setChartMode':
      case 'toggleChartMode':
      case 'setChartChoruses':
      case 'setChartLoop':
      case 'setChartIntro':
      case 'setChartEnding':
      case 'setChartAutoStyle':
        this.chartCmd(cmd)
        break
      case 'setPartPlugin':
      case 'clearPartPlugin':
        // A plugin picked here ends the part's own library patch.
        this.sound.partPlugin(cmd.part, cmd.type === 'setPartPlugin')
        this.plugins.cmd(cmd)
        break
      case 'savePartPluginState':
      case 'rescanPlugins':
      case 'setPluginInProcess':
      case 'reloadPartPlugin':
        this.plugins.cmd(cmd)
        break
      case 'loadMultiPad':
      case 'loadMultiPadPath':
      case 'clearMultiPad':
      case 'triggerMultiPad':
      case 'stopMultiPad':
      case 'stopAllMultiPads':
      case 'armMultiPad':
      case 'setMultiPadRepeat':
      case 'setMultiPadChordMatch':
      case 'setMultiPadSynchroStop': {
        const err = this.multiPads.cmd(cmd, t.running)
        if (err) this.message(err, true)
        break
      }
      case 'createPatch':
      case 'updatePatch':
      case 'deletePatch':
      case 'duplicatePatch':
      case 'movePatch':
      case 'setPatchFavourite':
      case 'savePartAsPatch':
      case 'addPresetAsPatch':
      case 'auditionPatch':
      case 'auditionPreset':
      case 'stopPatchAudition':
      case 'setFamilyRule':
      case 'setProgramOverride':
      case 'setDrumRule':
      case 'clearStyleMap':
      case 'setPartPatch':
      case 'setPortSendsMapped':
      case 'browseSoundFont':
      case 'importSoundLibrary':
      case 'exportSoundLibrary': {
        // A rule may name a catalog entry (#117): it gets that sound's library patch.
        let sc: SoundLibraryCmd = cmd
        if ((cmd.type === 'setFamilyRule' || cmd.type === 'setProgramOverride' || cmd.type === 'setDrumRule') && cmd.patch) {
          const r = this.catalogMock.patchFor(this.state, cmd.patch, (c) => this.cmd(c))
          if ('error' in r) {
            this.message(r.error, true)
            break
          }
          sc = { ...cmd, patch: r.patch }
        }
        // A SoundFont patch picked over a Plugins-tab plugin ends that plugin.
        if (cmd.type === 'setPartPatch' && cmd.id && this.sound.ownPlugin(cmd.part)) {
          const p = this.state.soundLibrary.patches.find((q) => q.id === cmd.id)
          if (p?.source.kind === 'soundFont') this.plugins.cmd({ type: 'clearPartPlugin', part: cmd.part })
        }
        const err = this.sound.cmd(sc, t.running)
        if (err) this.message(err, true)
        else if (cmd.type === 'exportSoundLibrary') this.message(`Sound library exported to ${cmd.path ?? '/Users/me/Documents/yahaha/sound-library-export.json'}`)
        break
      }
      case 'setSoundFavourite':
      case 'auditionSound':
      case 'stopSoundAudition':
      case 'assignSound':
      case 'setSoundCategory': {
        const r = this.catalogMock.cmd(this.state, cmd)
        if (r.error) this.message(r.error, true)
        // A preset from the synth's own font is the part's GM voice (setPartVoice): it ends
        // a plugin picked for the part, as a SoundFont patch does.
        for (const c of r.run ?? []) this.cmd(c)
        if (r.assignLastAdded !== undefined) this.cmd({ type: 'setPartPatch', part: r.assignLastAdded, id: this.state.soundLibrary.lastAdded })
        break
      }
      case 'setParamLock':
        this.state.paramLocks[cmd.item] = cmd.on
        break
      case 'setDynamicsControl':
        this.state.dynamics.control = cmd.on
        break
      case 'setDynamics':
        this.state.dynamics.level = clampLevel(cmd.level)
        break
      case 'stepDynamics':
        this.state.dynamics.level = clampLevel(this.state.dynamics.level + cmd.delta)
        break
      case 'setDynamicsTouch':
        this.state.dynamics.touch = cmd.on
        break
      case 'toggleDynamicsTouch':
        this.state.dynamics.touch = !this.state.dynamics.touch
        break
      case 'setAccent':
        this.state.dynamics.accent = cmd.on
        break
      case 'toggleAccent':
        this.state.dynamics.accent = !this.state.dynamics.accent
        break
      case 'setAccentThreshold':
        this.state.dynamics.accentThreshold = Math.max(1, clampLevel(cmd.velocity))
        break
      // Knob Assign pages (#197): a turn runs its function's command, as the session does.
      case 'setKnobPage':
        this.knobs.setPage(cmd.page)
        break
      case 'stepKnobPage':
        this.knobs.step(cmd.delta)
        break
      case 'turnKnob': {
        const c = this.knobs.turn(cmd.knob, cmd.delta, this.state)
        if (c) this.cmd(c)
        break
      }
      // The effect bus (#204).
      case 'setEffectType': {
        const b = this.state.effects.blocks.find((x) => x.block === cmd.block)!
        const t = b.types.find((x) => x.effect === cmd.effect)
        if (!t) {
          this.message(`${b.name} has no ${cmd.effect} type`, true)
          break
        }
        if (b.effect !== t.effect) {
          b.effect = t.effect
          b.effectName = t.name
          // A new type starts at its own parameters (#236).
          b.params = fxParams(b.block, b.effect)
        }
        // The player's own choice: style changes leave it (#237).
        b.followStyle = false
        break
      }
      case 'setFollowStyle': {
        const b = this.state.effects.blocks.find((x) => x.block === cmd.block)!
        b.followStyle = cmd.on
        // As the session: the style's own type at once (the mock's styles set none: the default).
        if (cmd.on) {
          const t = b.styleEffect?.effect ?? { reverb: 'hall', chorus: 'chorus', variation: 'dottedEighth' }[b.block] as FxType
          b.effect = t
          b.effectName = b.types.find((x) => x.effect === t)!.name
          b.params = fxParams(b.block, t)
        }
        break
      }
      case 'setEffectParam': {
        const b = this.state.effects.blocks.find((x) => x.block === cmd.block)!
        const p = b.params.find((x) => x.param === cmd.param)
        if (!p) {
          this.message(`${b.name} has no ${cmd.param} parameter`, true)
          break
        }
        p.value = Math.max(p.min, Math.min(p.max, Math.round(cmd.value)))
        p.display = FX_PARAMS[p.param].display(p.value)
        break
      }
      case 'setEffectReturn':
        this.state.effects.blocks.find((x) => x.block === cmd.block)!.returnLevel = clampLevel(cmd.level)
        break
      case 'setBandSend':
        this.state.effects.blocks.find((x) => x.block === cmd.block)!.bandSend = clampLevel(cmd.level)
        break
      case 'setPadSend':
        this.state.effects.blocks.find((x) => x.block === cmd.block)!.padSend = clampLevel(cmd.level)
        break
    }
  }
}

/** Each effect parameter's block, name, range and reading, as the session's (#236). */
export const FX_PARAMS: Record<FxParam, { block: FxBlock; name: string; min: number; max: number; display: (v: number) => string }> = {
  reverbTime: { block: 'reverb', name: 'Time', min: 3, max: 100, display: (v) => `${(v / 10).toFixed(1)} s` },
  preDelay: { block: 'reverb', name: 'Pre-delay', min: 0, max: 200, display: (v) => `${v} ms` },
  reverbTone: { block: 'reverb', name: 'Tone', min: 10, max: 200, display: (v) => `${(v / 10).toFixed(1)} kHz` },
  delaySync: { block: 'variation', name: 'Tempo sync', min: 0, max: 1, display: (v) => (v ? 'On' : 'Off') },
  delayNote: { block: 'variation', name: 'Note', min: 0, max: 7, display: (v) => ['1/16', '1/8T', '1/8', '1/4T', '1/8.', '1/4', '1/4.', '1/2'][v] },
  delayTime: { block: 'variation', name: 'Time', min: 10, max: 2000, display: (v) => `${v} ms` },
  delayFeedback: { block: 'variation', name: 'Feedback', min: 0, max: 90, display: (v) => `${v}%` },
  delayTone: { block: 'variation', name: 'Tone', min: 10, max: 200, display: (v) => `${(v / 10).toFixed(1)} kHz` },
  pingPong: { block: 'variation', name: 'Ping-pong', min: 0, max: 1, display: (v) => (v ? 'On' : 'Off') },
  chorusRate: { block: 'chorus', name: 'Rate', min: 5, max: 500, display: (v) => `${(v / 100).toFixed(2)} Hz` },
  chorusDepth: { block: 'chorus', name: 'Depth', min: 0, max: 50, display: (v) => `${(v / 10).toFixed(1)} ms` },
}
const DELAY = { delaySync: 1, delayTime: 375, delayFeedback: 38, delayTone: 50 }

/** Each type's own parameter values (the session's `type_defaults`). */
const FX_TYPE_PARAMS: Partial<Record<FxType, Partial<Record<FxParam, number>>>> = {
  hall: { reverbTime: 24, preDelay: 22, reverbTone: 45 },
  room: { reverbTime: 9, preDelay: 4, reverbTone: 60 },
  stage: { reverbTime: 17, preDelay: 12, reverbTone: 65 },
  plate: { reverbTime: 18, preDelay: 1, reverbTone: 90 },
  chorus: { chorusRate: 55, chorusDepth: 22 },
  celeste: { chorusRate: 29, chorusDepth: 9 },
  flanger: { chorusRate: 21, chorusDepth: 18 },
  eighth: { ...DELAY, delayNote: 2, pingPong: 0 },
  dottedEighth: { ...DELAY, delayNote: 4, pingPong: 0 },
  quarter: { ...DELAY, delayNote: 5, pingPong: 0 },
  pingPong: { ...DELAY, delayNote: 2, pingPong: 1 },
}

/** A block's parameters at type `effect`'s own values. */
function fxParams(block: FxBlock, effect: FxType): FxParamState[] {
  const own = FX_TYPE_PARAMS[effect] ?? {}
  return (Object.keys(FX_PARAMS) as FxParam[])
    .filter((p) => FX_PARAMS[p].block === block)
    .map((param) => {
      const { name, min, max, display } = FX_PARAMS[param]
      const value = own[param] ?? min
      return { param, name, value, min, max, default: value, display: display(value) }
    })
}

/**
 * The effect bus as a session starts it: Hall, Chorus, the dotted 1/8 delay, every return 64;
 * the band's reverb as written (100), no band chorus or delay (#236); the same for the Multi
 * Pads (#267).
 */
export function initialEffects(): EffectsState {
  const block = (block: FxBlock, name: string, effect: FxType, types: [FxType, string][], bandSend: number): EffectBlockState => ({
    block, name, effect, effectName: types.find(([t]) => t === effect)![1],
    types: types.map(([effect, name]) => ({ effect, name })), returnLevel: 64, bandSend, padSend: bandSend, params: fxParams(block, effect),
    styleEffect: null, followStyle: true,
  })
  return {
    blocks: [
      block('reverb', 'Reverb', 'hall', [['hall', 'Hall'], ['room', 'Room'], ['stage', 'Stage'], ['plate', 'Plate']], 100),
      block('chorus', 'Chorus', 'chorus', [['chorus', 'Chorus'], ['celeste', 'Celeste'], ['flanger', 'Flanger']], 0),
      block('variation', 'Variation', 'dottedEighth', [['eighth', 'Delay 1/8'], ['dottedEighth', 'Delay 1/8.'], ['quarter', 'Delay 1/4'], ['pingPong', 'Ping-Pong']], 0),
    ],
  }
}

/** A Dynamics level or velocity, as the session clamps it: a whole number 0-127. */
function clampLevel(v: number): number {
  return Math.max(0, Math.min(127, Math.round(v)))
}
