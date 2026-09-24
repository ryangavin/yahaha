// A mock session that behaves like the engine closely enough to develop and screenshot
// the UI: the band advances bar by bar, queued sections flash and take over at the bar
// (fills at the beat), chords change, faders wait for pickup. No audio, no MIDI.
// It speaks #16's API (types.ts); `app/src-tauri/src/mock.rs` is the Rust twin that the
// app shell runs until the engine's `Session` is wired in.

import fixture from './mock-fixture.json'
import { deriveSurface } from '../surface'
import { padsFor } from './mock-pads'
import type { Session } from './session'
import {
  BREAK, ENDINGS, FILLS, FINGERINGS, INTROS, KEYBOARD_PART_NAMES, MAINS, PAD_PAGES, STYLE_PART_NAMES,
  type AppCmd, type AppState, type LibraryEntry, type LibraryList, type OtsPart, type StyleState,
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

export const LIBRARY: LibraryList = {
  revision: 1,
  entries: STYLES.map((s): LibraryEntry => ({
    id: s.id,
    name: s.name,
    folder: s.folder,
    path: `${ROOT}/${s.folder}/${s.file}`,
    status: s.error ? 'error' : 'ok',
    error: s.error ?? null,
    tempo: s.error ? null : s.tempo,
    timeSignature: s.error ? null : [s.timeSignature[0], s.timeSignature[1]],
    sections: s.error ? '' : sectionsText(s.sections),
  })),
}

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
    path: `${ROOT}/${s.folder}/${s.file}`,
    name: s.name,
    format: s.file.endsWith('.sty') ? 'SFF1' : 'SFF2',
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

/** A stopped session with the first style loaded and Sync Start armed. */
export function initialState(): AppState {
  const s = STYLES[0]
  const part = (i: number, program: number, on: boolean) => ({
    name: KEYBOARD_PART_NAMES[i], channel: [1, 3, 4, 2][i], on, sounding: on, selected: i === 0,
    volume: 100, waiting: false, program, voiceName: GM[program], playsBass: false, octave: 0,
  })
  const state: AppState = {
    version: 1,
    style: styleState(s),
    transport: {
      running: false, syncStart: true, syncStop: false, syncStopAvailable: true, autoFill: false, stopAcmp: false,
      section: null, queued: null, pendingIntro: null, main: 0, bar: 1, beat: 1,
      beatsPerBar: beatsPerBar([s.timeSignature[0], s.timeSignature[1]]), tempo: s.tempo, lamps: [],
    },
    chord: {
      name: null, fingered: null, fingering: 'fingeredOnBass', fingeringName: 'Fingered On Bass', upper: false,
      manualBass: true, manualBassActive: false, split: 54, splitName: noteName(54), transposeKeyboard: 0, transposeMaster: 0,
    },
    keyboardParts: [part(0, 0, true), part(1, 48, false), part(2, 61, false), part(3, 48, false)],
    mixer: {
      faderPage: 'panel',
      styleParts: STYLE_PART_NAMES.map((name, i) => ({
        name, channel: 9 + i, on: true, mutedByManualBass: false,
        volume: [100, 100, 96, 80, 76, 70, 88, 84][i], waiting: false,
        voice: { bankMsb: STYLE_VOICES[i][0], bankLsb: STYLE_VOICES[i][1], program: STYLE_VOICES[i][2], kit: STYLE_VOICES[i][3], label: STYLE_VOICES[i][4] },
      })),
      master: 100,
      masterWaiting: false,
    },
    pads: { page: 'sections', pageName: 'Sections', pageNumber: 1, pageCount: 3, pads: [], connected: true },
    ots: { settings: otsSettings(s.ots), applied: 0, link: false },
    library: { revision: LIBRARY.revision, count: LIBRARY.entries.length, position: 0, pending: 0 },
    io: {
      outputPort: 'yahaha',
      inputs: ['Launchkey 49 MK4 LKMK4 MIDI Out', 'Launchkey 49 MK4 LKMK4 DAW Out (pads)'],
      synth: {
        soundFont: 'GeneralUser-GS', device: 'MacBook Pro Speakers', sampleRate: 48000, bufferFrames: 64,
        channels: 2, outputPair: [1, 2], muted: false,
      },
      engine: { realtime: true, wakeP99Us: 3, chordP99Us: 15, midiInP99Us: 120 },
      lastControl: 0,
      unmapped: '',
      offline: false,
    },
    message: null,
  }
  derive(state)
  return state
}

/** The fields the engine computes from the others: pads, lamps, names, flags, and the
 * provisional `surface` (with the mock's hardware fader positions and beat clock). */
function derive(st: AppState, hw: { faders: number[]; beats: number; atMs: number } | null = null) {
  const c = st.chord
  c.fingeringName = c.upper ? 'Fingered*' : FINGERINGS.find((f) => f.id === c.fingering)!.name
  c.manualBassActive = c.upper && c.manualBass
  c.splitName = noteName(c.split)
  st.transport.syncStopAvailable = c.upper || !(c.fingering === 'fullKeyboard' || c.fingering === 'aiFullKeyboard')
  st.keyboardParts.forEach((p, i) => {
    p.playsBass = i === 3 && c.manualBassActive
    p.sounding = p.on || p.playsBass
    p.voiceName = p.playsBass ? 'Finger Bass' : GM[p.program]
  })
  st.mixer.styleParts.forEach((p, i) => (p.mutedByManualBass = i === 2 && c.manualBassActive))
  const page = PAD_PAGES.findIndex((p) => p.id === st.pads.page)
  st.pads.pageName = PAD_PAGES[page].name
  st.pads.pageNumber = page + 1
  st.transport.lamps = padsFor(st, 'sections')
  st.pads.pads = padsFor(st, st.pads.page)
  const surface = deriveSurface(st, LIBRARY)
  if (hw) {
    surface.faders.forEach((f, i) => (f.position = f.set ? hw.faders[i] : null))
    surface.clock = { ...surface.clock, phase: hw.beats - Math.floor(hw.beats), atMs: hw.atMs }
  }
  st.surface = surface
}

/** How many bars a section lasts before it moves on (Intro/Ending: 2, Break/Fill: 1). */
function sectionBars(s: string): number {
  return INTROS.includes(s) || ENDINGS.includes(s) ? 2 : 1
}

export interface MockOptions {
  /** Script some activity: start mid-song, change Main every few bars, move faders. */
  demo?: boolean
  /** Drive the clock yourself with `advance(ms)` instead of a 60 Hz timer (tests). */
  manual?: boolean
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
  private now = 0
  private progression = 0
  private messageSeq = 0
  private demo: boolean
  /** Where the (imaginary) hardware faders physically are: 1–8, master. */
  private hwFaders = [100, 72, 100, 100, 0, 0, 0, 0, 100]

  constructor(opts: MockOptions = {}) {
    this.demo = opts.demo ?? false
    this.state = initialState()
    if (this.demo) this.demoStart()
    derive(this.state, { faders: this.hwFaders, beats: this.clock, atMs: this.now })
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
    st.mixer.styleParts[5].waiting = true
    st.mixer.styleParts[5].volume = 58
    this.position()
  }

  subscribe(fn: (s: AppState) => void) {
    this.subs.add(fn)
    fn(this.snapshot())
    return () => this.subs.delete(fn)
  }

  library() {
    return Promise.resolve(LIBRARY)
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
    this.state.version++
    derive(this.state, { faders: this.hwFaders, beats: this.clock, atMs: this.now })
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
    const t = this.state.transport
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
  }

  private onBeat() {
    const t = this.state.transport
    if (t.queued && FILLS.includes(t.queued)) {
      t.section = t.queued
      t.queued = null
      this.sectionStart = Math.floor(this.clock / t.beatsPerBar)
    }
  }

  private onBar(bar: number) {
    const t = this.state.transport
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
    if (this.demo) this.demoBar(bar)
    // The style follows a new chord every other bar.
    if (t.running && bar % 2 === 0) this.chordArrives(PROGRESSION[this.progression++ % PROGRESSION.length])
  }

  private demoBar(bar: number) {
    const t = this.state.transport
    // Every 8 bars, queue the next Main (half a bar early, so the flashing shows).
    if (bar % 8 === 6 && !t.queued) this.cmd({ type: 'main', index: (t.main + 1) % 4 })
    // A pattern's volume change moves a Style fader away from the hardware (soft takeover).
    if (bar % 8 === 0) {
      const p = this.state.mixer.styleParts[(bar / 8) % 8 | 0]
      p.volume = Math.max(40, Math.min(127, p.volume + (bar % 16 === 0 ? -14 : 14)))
      p.waiting = true
    }
  }

  private enter(s: string, bar: number) {
    const t = this.state.transport
    t.section = s
    this.sectionStart = bar
    const m = MAINS.indexOf(s)
    if (m >= 0) {
      t.main = m
      if (this.state.ots.link && m < this.state.ots.settings.length) this.recallOts(m)
    }
  }

  private chordArrives(chord: string) {
    const t = this.state.transport
    const k = this.state.chord.transposeKeyboard
    this.state.chord.name = transposeChord(chord, k)
    this.state.chord.fingered = chord
    if (!t.running && t.syncStart) this.startBand()
  }

  private startBand() {
    const t = this.state.transport
    t.running = true
    t.syncStart = false
    this.clock = 0
    this.sectionStart = 0
    t.section = t.pendingIntro !== null && this.has(INTROS[t.pendingIntro]) ? INTROS[t.pendingIntro] : MAINS[t.main]
    t.pendingIntro = null
    t.queued = null
    this.position()
  }

  private stopBand() {
    const t = this.state.transport
    t.running = false
    t.section = null
    t.queued = null
    t.bar = 1
    t.beat = 1
  }

  private recallOts(n: number) {
    const panel = this.state.mixer.faderPage === 'panel'
    this.state.ots.settings[n].parts.forEach((o, i) => {
      const p = this.state.keyboardParts[i]
      if (o.program !== null) p.program = o.program
      p.on = o.on
      p.octave = o.octave
      if (p.volume !== o.volume) p.waiting = panel
      p.volume = o.volume
    })
    this.state.ots.applied = n + 1
  }

  private loadStyle(id: number) {
    const s = STYLES[id]
    if (!s) return
    if (s.error) {
      this.message(`${s.folder}/${s.file}: ${s.error}`, true)
      return
    }
    const st = this.state
    const t = st.transport
    st.style = styleState(s)
    t.tempo = s.tempo
    t.beatsPerBar = beatsPerBar(st.style.timeSignature)
    st.ots = { settings: otsSettings(s.ots), applied: 0, link: st.ots.link }
    if (st.ots.link && t.main < s.ots) this.recallOts(t.main)
    for (const p of st.mixer.styleParts) {
      p.volume = 100
      p.waiting = st.mixer.faderPage === 'style'
    }
    if (t.section && !this.has(t.section)) t.section = MAINS.find((m) => this.has(m)) ?? null
    st.library.position = LIBRARY.entries.findIndex((e) => e.id === id)
    st.message = null
  }

  send(cmd: AppCmd) {
    this.cmd(cmd)
    this.publish()
  }

  private cmd(cmd: AppCmd) {
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
        break
      }
      case 'break':
        if (t.running && this.has(BREAK)) t.queued = BREAK
        break
      case 'ending':
        if (t.running && this.has(ENDINGS[cmd.index])) t.queued = ENDINGS[cmd.index]
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
        t.stopAcmp = !t.stopAcmp
        break
      case 'tapTempo': {
        this.taps = [...this.taps.filter((x) => this.now - x < 2000), this.now].slice(-4)
        if (this.taps.length >= 2) {
          const avg = (this.taps[this.taps.length - 1] - this.taps[0]) / (this.taps.length - 1)
          if (avg > 0) t.tempo = clamp(Math.round(60000 / avg), 30, 300)
        }
        break
      }
      case 'tempoUp':
        t.tempo = clamp(t.tempo + 1, 30, 300)
        break
      case 'tempoDown':
        t.tempo = clamp(t.tempo - 1, 30, 300)
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
        break
      case 'stepVoice': {
        const p = st.keyboardParts.find((x) => x.selected) ?? st.keyboardParts[0]
        p.program = (p.program + cmd.delta + 128) % 128
        break
      }
      case 'setPartVolume':
        st.keyboardParts[cmd.part].volume = vol(cmd.volume)
        st.keyboardParts[cmd.part].waiting = false
        break
      case 'setPartOctave':
        st.keyboardParts[cmd.part].octave = clamp(cmd.octave, -2, 2)
        break
      case 'setFaderPage':
      case 'toggleFaderPage': {
        const page = cmd.type === 'setFaderPage' ? cmd.page : st.mixer.faderPage === 'panel' ? 'style' : 'panel'
        if (page === st.mixer.faderPage) break
        st.mixer.faderPage = page
        // The hardware faders are wherever they were: every level on the new page waits.
        for (const p of page === 'panel' ? st.keyboardParts : st.mixer.styleParts) p.waiting = true
        break
      }
      case 'setPadPage':
        st.pads.page = cmd.page
        break
      case 'cyclePadPage': {
        const i = PAD_PAGES.findIndex((p) => p.id === st.pads.page)
        st.pads.page = PAD_PAGES[(((i + cmd.delta) % 3) + 3) % 3].id
        break
      }
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
      case 'loadStyle':
        this.loadStyle(cmd.id)
        break
      case 'loadStylePath': {
        const e = LIBRARY.entries.find((x) => x.path === cmd.path)
        if (e) this.loadStyle(e.id)
        else this.message(`${cmd.path}: not found`, true)
        break
      }
      case 'stepStyle': {
        const n = LIBRARY.entries.length
        let i = st.library.position
        for (let k = 0; k < n; k++) {
          i = (((i + cmd.delta) % n) + n) % n
          if (LIBRARY.entries[i].status === 'ok') break
        }
        this.loadStyle(LIBRARY.entries[i].id)
        break
      }
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
      case 'panic':
        this.stopBand()
        this.message('All notes off')
        break
      case 'clearMessage':
        st.message = null
        break
    }
  }
}
