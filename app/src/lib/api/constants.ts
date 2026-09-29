// Constants and helpers the app itself needs that used to live in the mocks. The mocks
// re-export them, so mock code and tests are unchanged; everything that isn't the mock
// imports them from here, so the production bundle never loads the mock (session.ts
// loads it with a dynamic import for the mock backend only).

import { defaultControllers } from './assignable'
import { emptyQuickRacks } from './quick-racks'
import { emptyMap, type SoundLibraryState } from './sound-library'
import {
  defaultControlMap,
  eqPresetBands,
  FLAT_EQ,
  KEYBOARD_PART_NAMES,
  OFF_INSERT,
  PAD_PAGES,
  STYLE_PART_NAMES,
  type AppState,
  type ClockState,
  type ControlId,
  type LibraryList,
  type Neighbour,
  type SurfaceControl,
  type SurfaceFader,
} from './types'

/** The General MIDI program names, 0–127 (the same list as `gm` in mock-fixture.json,
 * which the Rust dev mock reads; a test keeps the two equal). */
export const GM: string[] = [
  'Grand Piano', 'Bright Piano', 'E.Grand', 'Honky-tonk', 'E.Piano 1', 'E.Piano 2', 'Harpsichord', 'Clavinet',
  'Celesta', 'Glockenspiel', 'Music Box', 'Vibraphone', 'Marimba', 'Xylophone', 'Tubular Bells', 'Dulcimer',
  'Drawbar Organ', 'Perc. Organ', 'Rock Organ', 'Church Organ', 'Reed Organ', 'Accordion', 'Harmonica', 'Bandoneon',
  'Nylon Gtr', 'Steel Gtr', 'Jazz Gtr', 'Clean Gtr', 'Muted Gtr', 'Overdrive Gtr', 'Distortion Gtr', 'Gtr Harmonics',
  'Acoustic Bass', 'Finger Bass', 'Pick Bass', 'Fretless Bass', 'Slap Bass 1', 'Slap Bass 2', 'Synth Bass 1', 'Synth Bass 2',
  'Violin', 'Viola', 'Cello', 'Contrabass', 'Tremolo Str', 'Pizzicato Str', 'Harp', 'Timpani',
  'Strings', 'Slow Strings', 'Synth Str 1', 'Synth Str 2', 'Choir Aahs', 'Voice Oohs', 'Synth Voice', 'Orch. Hit',
  'Trumpet', 'Trombone', 'Tuba', 'Muted Trumpet', 'French Horn', 'Brass Section', 'Synth Brass 1', 'Synth Brass 2',
  'Soprano Sax', 'Alto Sax', 'Tenor Sax', 'Baritone Sax', 'Oboe', 'English Horn', 'Bassoon', 'Clarinet',
  'Piccolo', 'Flute', 'Recorder', 'Pan Flute', 'Blown Bottle', 'Shakuhachi', 'Whistle', 'Ocarina',
  'Square Lead', 'Saw Lead', 'Calliope', 'Chiff Lead', 'Charang', 'Voice Lead', 'Fifths Lead', 'Bass+Lead',
  'New Age Pad', 'Warm Pad', 'Polysynth', 'Choir Pad', 'Bowed Pad', 'Metallic Pad', 'Halo Pad', 'Sweep Pad',
  'Rain', 'Soundtrack', 'Crystal', 'Atmosphere', 'Brightness', 'Goblins', 'Echoes', 'Sci-fi',
  'Sitar', 'Banjo', 'Shamisen', 'Koto', 'Kalimba', 'Bagpipe', 'Fiddle', 'Shanai',
  'Tinkle Bell', 'Agogo', 'Steel Drums', 'Woodblock', 'Taiko', 'Melodic Tom', 'Synth Drum', 'Reverse Cymbal',
  'Fret Noise', 'Breath Noise', 'Seashore', 'Bird', 'Telephone', 'Helicopter', 'Applause', 'Gunshot',
]

export const NOTE_NAMES = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B']

/** Yamaha numbering: C3 = middle C (60). */
export function noteName(n: number): string {
  return `${NOTE_NAMES[n % 12]}${Math.floor(n / 12) - 2}`
}

/** The styles either side of `position` that Track ◀/▶ would load (skipping ones that
 * failed to parse), as the engine picks them. */
export function neighbours(lib: LibraryList, position: number): { prev: Neighbour | null; next: Neighbour | null } {
  const n = lib.entries.length
  const step = (d: number): Neighbour | null => {
    let i = position
    for (let k = 0; k < n; k++) {
      i = (((i + d) % n) + n) % n
      if (i === position) return null
      const e = lib.entries[i]
      if (e.status !== 'error') return { id: e.id, name: e.name, path: e.path }
    }
    return null
  }
  return n ? { prev: step(-1), next: step(1) } : { prev: null, next: null }
}

const CONTROL_IDS: ControlId[] = [
  'padBankUp', 'padBankDown', 'trackPrev', 'trackNext', 'play', 'stop', 'scene', 'function',
  'faderButton1', 'faderButton2', 'faderButton3', 'faderButton4', 'faderButton5', 'faderButton6', 'faderButton7', 'faderButton8', 'masterButton',
]

/**
 * What the app shows before the session's first state arrives: nothing loaded, stopped,
 * every part and control present but empty and unlit. Its version is 0, so the first
 * real state (version 1 and up) replaces it. (The mock's `initialState` is a full demo
 * session instead.)
 */
export function emptyState(): AppState {
  const clock: ClockState = {
    atMs: 0, running: false, tempo: 120, beatsPerBar: 4, bar: 1, beat: 1, phase: 0,
    sectionAnchorMs: 0, sectionAnchorBeats: 0, ledAnchorMs: 0, ledAnchorBeats: 0,
  }
  const control = (id: ControlId): SurfaceControl => ({
    id, cc: 0, label: '', action: null, shiftLabel: '', shiftAction: null, rgb: [0, 0, 0], level: 'off', anim: 'solid', colour: null,
  })
  const fader = (): SurfaceFader => ({ label: '', value: null, waiting: false, position: null, set: null })
  const soundLibrary: SoundLibraryState = {
    patches: [], categories: [], families: [], map: emptyMap(), styleMap: emptyMap(), styleKey: '', usage: [],
    portSendsMapped: false, auditioning: null, browse: null, file: '', extraSoundFonts: [], lastAdded: null, gmMap: [],
  }
  return {
    version: 0,
    style: { id: 0, path: '', name: '', format: '', tempo: 120, timeSignature: [4, 4], sections: [] },
    transport: {
      running: false, syncStart: false, syncStop: false, syncStopAvailable: false, autoFill: false, stopAcmp: false,
      section: null, queued: null, landing: null, pendingIntro: null, main: 0, bar: 1, beat: 1,
      beatsPerBar: 4, tempo: 120, lamps: [], sectionBars: null,
      halfBarFill: false, stopAcmpMode: 'off',
      fade: 'off', retrigger: false, ritardando: false, acmp: false,
      unison: false, unisonLatched: false, unisonType: 'root',
    },
    chord: {
      name: null, fingered: null, fingering: 'fingeredOnBass', fingeringName: '', upper: false,
      manualBass: false, manualBassActive: false, split: 54, splitName: noteName(54), transposeKeyboard: 0, transposeMaster: 0, settleMs: 0, leftHold: false,
    },
    keyboardParts: KEYBOARD_PART_NAMES.map((name, i) => ({
      name, channel: [1, 3, 4, 2][i], on: false, sounding: false, selected: i === 0,
      volume: 100, waiting: false, program: 0, voiceName: '', playsBass: false, octave: 0, pan: 64, reverb: 0, chorus: 0, variation: 0,
      eq: { ...FLAT_EQ }, insert: { ...OFF_INSERT }, fader: null, patch: null,
    })),
    keyboard: { held: [], leftSplit: 54, chordTones: [], chordBass: null, detection: [0, 54] },
    mixer: {
      faderPage: 'panel', faderLayer: 'volume', sendWaiting: 0, styleSendWaiting: 0,
      styleParts: STYLE_PART_NAMES.map((name, i) => ({
        name, channel: 9 + i, on: false, mutedByManualBass: false, volume: 100, waiting: false, fader: null,
        reverb: 0, chorus: 0, variation: 0, sendsSet: [], voice: { bankMsb: 0, bankLsb: 0, program: 0, kit: false, label: '' },
      })),
      master: 100, masterWaiting: false, styleVolume: 100, styleVolumeWaiting: false, multiPadVolume: 100, multiPadVolumeWaiting: false,
      styleSolo: null, partSolo: null,
    },
    pads: { page: 'sections', pageName: PAD_PAGES[0].name, pageNumber: 1, pageCount: PAD_PAGES.length, pads: [], connected: false, paletteLeds: false },
    ots: { settings: [], applied: 0, link: false, linkTiming: 'mainChange', racks: [], racksReadOnly: false },
    library: { revision: 0, count: 0, position: 0, pending: 0, roots: [], scanning: false },
    io: {
      outputPort: '', inputs: [],
      synth: { soundFont: '', device: '', sampleRate: 0, bufferFrames: 0, channels: 2, outputPair: [1, 2], muted: false, dropouts: 0 },
      engine: { realtime: false, wakeP99Us: 0, chordP99Us: 0, midiInP99Us: 0 },
      lastControl: 0, unmapped: '', offline: false, sources: [], allInputs: true, soundFonts: [], soundFontFile: '', soundFontLoading: false,
    },
    message: null,
    styleChange: { tempo: 'hold', parts: 'hold', sectionSet: null },
    surface: { shift: false, controls: CONTROL_IDS.map(control), faders: Array.from({ length: 9 }, fader), trackPrev: null, trackNext: null, clock },
    preview: { audition: null, queued: null },
    chart: {
      on: false, playlists: [], selected: null, song: null, choruses: 1, intro: 0, ending: 0, loop: null,
      autoStyle: true, suggestedStyle: null, bar: null, overridden: false,
    },
    styleSettings: {
      mainTiming: 'nextBar', introEndingTiming: 'nextBar', syncStopWindowMs: 0,
      fadeInMs: 5000, fadeOutMs: 5000, fadeHoldMs: 2000, sectionReset: true, retriggerRate: 8, swing: 0, swingGrid: 8, sectionTempo: true,
    },
    looper: {
      mode: 'off', hasData: false, bar: null, bars: 0, chords: [], memory: null, pendingMemory: null,
      memories: [], bankName: '', bankPath: null, banks: [],
    },
    metronome: { on: false, volume: 90, bell: true, audible: true },
    multiPad: {
      bank: null, loading: false,
      pads: [0, 1, 2, 3].map((i) => ({ index: i, name: '', lamp: 'empty' as const, repeat: false, chordMatch: false, channel: 5 + i })),
      synchroStop: { styleStop: true, ending: false }, banks: [],
    },
    controllers: defaultControllers(),
    plugins: { available: false, scanning: false, list: [], missing: [], needsAttention: [], instances: 0 },
    harmonyArp: {
      on: false, mode: 'harmony', harmonyType: 0, arpPattern: 0, typeName: '', category: '',
      volume: 100, speed: '1/8', assign: 'auto', chordNoteOnly: false, touchLimit: 1,
      arp: { quantize: 'off', hold: false, pedalHold: false, velocity: 'original', fixedVelocity: 100, keepKeyOn: false },
    },
    soundLibrary,
    paramLocks: { splitPoint: false, fingeringType: false },
    sounds: { revision: 0, count: 0, scanning: false, listingPresets: [] },
    dynamics: { control: true, level: 127, touch: false, accent: false, accentThreshold: 110, accentMode: 'hits', accentSource: 'left' },
    knobs: { page: 'style', pageName: '', pageNumber: 1, pageCount: 1, knobs: [] },
    effects: {
      blocks: [], inserts: [], insertsOn: true, rotaryFast: false,
      master: {
        compressor: { on: false, preset: 'natural', compression: 30, texture: 50, output: 1, edited: false },
        eq: { on: false, preset: 'flat', bands: eqPresetBands('flat'), edited: false },
      },
    },
    home: { mains: [], progress: { running: false, bar: 1, beat: 1, bars: null, beatsPerBar: 4, fraction: 0 }, ots: null, bandSends: [] },
    liveRack: { name: '', id: null, modified: false, controls: defaultControlMap(), prompt: null },
    racks: [],
    quickRacks: emptyQuickRacks(),
  }
}
