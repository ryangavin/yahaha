// The app's view of the engine: `AppState` (a snapshot of everything the UI shows),
// `AppCmd` (every action) and the library list. These mirror #16's `src/api.rs` field for
// field (docs/app-api.md is the contract): camelCase JSON, commands tagged by `type`.
//
// Tracks the draft PR #71 (branch m3/engine-api, commit 8df4f31). If the draft changes,
// change this file and `app/src-tauri/src/api.rs` to match; components only see these
// types and the `Session` interface.

export type Fingering =
  | 'singleFinger' | 'multiFinger' | 'fingered' | 'fingeredOnBass'
  | 'aiFingered' | 'fullKeyboard' | 'aiFullKeyboard'

/** The Launchkey pad pages, switched with Pad Bank ▲/▼. */
export type PadPage = 'sections' | 'chordSetup' | 'otsParts'

/** What the Launchkey faders control, like the Genos Mixer's Panel and Style tabs. */
export type FaderPage = 'panel' | 'style'

export type Level = 'off' | 'dim' | 'bright'
/** flash: queued (alternates every half beat) · pulse: armed (breathes over two beats). */
export type Anim = 'solid' | 'flash' | 'pulse'
export type Rgb = [number, number, number]

/** Every user action. Indices are 0-based. Keyboard parts: 0–3 = Right 1, Right 2,
 * Right 3, Left. Style parts: 0–7 = Rhythm 1 … Phrase 2 (channels 9–16). */
export type AppCmd =
  // Sections and transport
  | { type: 'intro'; index: number }
  | { type: 'main'; index: number }
  | { type: 'break' }
  | { type: 'ending'; index: number }
  | { type: 'startStop' }
  | { type: 'stop' }
  | { type: 'toggleSyncStart' }
  | { type: 'toggleSyncStop' }
  | { type: 'toggleAutoFill' }
  | { type: 'toggleStopAcmp' }
  | { type: 'tapTempo' }
  | { type: 'tempoUp' }
  | { type: 'tempoDown' }
  | { type: 'toggleStylePart'; part: number }
  | { type: 'setStylePartVolume'; part: number; volume: number }
  // Chord detection, split, transpose
  | { type: 'setFingering'; fingering: Fingering }
  | { type: 'nextFingering' }
  | { type: 'setUpper'; on: boolean }
  | { type: 'toggleUpper' }
  | { type: 'setManualBass'; on: boolean }
  | { type: 'toggleManualBass' }
  | { type: 'setSplit'; note: number }
  | { type: 'moveSplit'; delta: number }
  | { type: 'setTranspose'; keyboard: number; master: number }
  | { type: 'stepTranspose'; keyboard: number; master: number }
  | { type: 'resetTranspose' }
  // Keyboard parts
  | { type: 'setPartOn'; part: number; on: boolean }
  | { type: 'togglePart'; part: number }
  | { type: 'selectPart'; part: number }
  | { type: 'setPartVoice'; part: number; program: number }
  | { type: 'stepVoice'; delta: number }
  | { type: 'setPartVolume'; part: number; volume: number }
  | { type: 'setPartOctave'; part: number; octave: number }
  // Mixer and Launchkey pages
  | { type: 'setFaderPage'; page: FaderPage }
  | { type: 'toggleFaderPage' }
  | { type: 'setPadPage'; page: PadPage }
  | { type: 'cyclePadPage'; delta: number }
  | { type: 'setMasterVolume'; volume: number }
  // One Touch Settings
  | { type: 'recallOts'; index: number }
  | { type: 'setOtsLink'; on: boolean }
  | { type: 'toggleOtsLink' }
  // Styles
  | { type: 'loadStyle'; id: number }
  | { type: 'loadStylePath'; path: string }
  | { type: 'stepStyle'; delta: number }
  // Output
  | { type: 'setSynthMuted'; on: boolean }
  | { type: 'toggleSynthMute' }
  | { type: 'setAudioOutput'; first: number }
  | { type: 'nextAudioOutput' }
  | { type: 'panic' }
  | { type: 'clearMessage' }

export type CmdError = { kind: 'busy' } | { kind: 'failed'; message: string }

/** Notifications; they carry no state (read `state()` / `library()`). */
export type SessionEvent =
  | { type: 'stateChanged'; version: number }
  | { type: 'libraryChanged'; revision: number }
  | { type: 'stopped' }

export interface Pad {
  /** The pad's note on the Launchkey DAW port (top row 96–103, bottom row 112–119). */
  note: number
  /** e.g. "MAIN A", "FINGERED", "OTS 1"; empty for an unused pad. */
  label: string
  /** The terminal UI's shortcut, e.g. "1", "spc", "F10". */
  key: string
  /** Full-brightness colour, 0–127 per channel. */
  rgb: Rgb
  level: Level
  anim: Anim
  /** What pressing it sends (null: an unused pad). */
  action: AppCmd | null
}

export interface StyleState {
  id: number
  path: string
  name: string
  /** "SFF1" or "SFF2". */
  format: string
  /** The style's own tempo; the current tempo is `transport.tempo`. */
  tempo: number
  timeSignature: [number, number]
  /** Sections the style has: "Intro A", "Main B", "Fill In AA", "Fill In BA" (Break), "Ending C". */
  sections: string[]
}

export interface TransportState {
  running: boolean
  /** Sync Start is armed: the first chord starts the style. */
  syncStart: boolean
  syncStop: boolean
  /** Not in the Full Keyboard fingering types in Lower. */
  syncStopAvailable: boolean
  autoFill: boolean
  stopAcmp: boolean
  /** The section playing, e.g. "Main A", "Fill In AA"; null when stopped. */
  section: string | null
  /** The section queued next (at the next bar; a fill at the next beat). */
  queued: string | null
  /** The Intro (0–2) armed to play when the style starts. */
  pendingIntro: number | null
  /** The Main (0–3 = A–D) playing, or returned to after a fill. */
  main: number
  /** Position in the section playing, both 1-based (1, 1 when stopped). */
  bar: number
  beat: number
  beatsPerBar: number
  /** Current tempo in BPM. */
  tempo: number
  /** Page 1 of the pads, whatever page the hardware is on: the section lamps. */
  lamps: Pad[]
  /** Provisional (NEED on the board): how many bars the section playing lasts (a Main's
   * pattern length; it loops), for the lead-sheet band's progress. Absent: unknown. */
  sectionBars?: number | null
}

export interface ChordState {
  /** The chord the style follows (after Keyboard transpose), e.g. "Am7/G". */
  name: string | null
  /** The chord as fingered, before Keyboard transpose. */
  fingered: string | null
  fingering: Fingering
  /** e.g. "Fingered On Bass". */
  fingeringName: string
  upper: boolean
  manualBass: boolean
  /** Upper with Manual Bass on: Left plays the Style's Bass voice. */
  manualBassActive: boolean
  /** Split point (MIDI note) and its name, Yamaha numbering (C3 = 60). */
  split: number
  splitName: string
  transposeKeyboard: number
  transposeMaster: number
}

export interface KeyboardPart {
  /** "Right 1", "Right 2", "Right 3", "Left". */
  name: string
  /** 1-based: Right 1 = 1, Left = 2, Right 2 = 3, Right 3 = 4. */
  channel: number
  on: boolean
  /** It sounds: on, or Left playing the bass under Manual Bass. Light the lamp from this. */
  sounding: boolean
  selected: boolean
  volume: number
  /** The Launchkey fader hasn't reached `volume` yet (soft takeover). */
  waiting: boolean
  program: number
  voiceName: string
  playsBass: boolean
  octave: number
}

export interface Voice {
  bankMsb: number
  bankLsb: number
  program: number
  kit: boolean
  /** What the synth plays for it, e.g. "≈ Strings  [Yamaha 104/0/49]". */
  label: string
}

export interface StylePart {
  /** "Rhythm 1" … "Phrase 2". */
  name: string
  /** 9–16. */
  channel: number
  on: boolean
  mutedByManualBass: boolean
  volume: number
  waiting: boolean
  voice: Voice | null
}

export interface MixerState {
  faderPage: FaderPage
  styleParts: StylePart[]
  /** Synth master volume (100 = unity); null without the synth. */
  master: number | null
  masterWaiting: boolean
}

export interface PadsState {
  page: PadPage
  /** "Sections", "Chord/Setup", "OTS/Parts". */
  pageName: string
  pageNumber: number
  pageCount: number
  /** This page's 16 pads: the top row, then the bottom row. */
  pads: Pad[]
  connected: boolean
}

export interface OtsPart {
  on: boolean
  /** GM program, or null for a drum kit voice. */
  program: number | null
  voiceName: string
  volume: number
  octave: number
}

export interface OtsState {
  /** The style's One Touch Settings (0–4): "OTS 1" … with Right 1–3 and Left as it sets them. */
  settings: { name: string; parts: OtsPart[] }[]
  /** The last one recalled, 1-based; 0 = none since the style loaded. */
  applied: number
  link: boolean
}

export interface LibraryStatus {
  revision: number
  count: number
  /** The loaded style's position in library order. */
  position: number
  /** Entries still being indexed. */
  pending: number
}

export interface SynthState {
  soundFont: string
  device: string
  sampleRate: number
  bufferFrames: number | null
  channels: number
  /** 1-based, e.g. [1, 2]. */
  outputPair: [number, number]
  muted: boolean
}

export interface IoState {
  outputPort: string
  /** Connected MIDI sources; the Launchkey DAW port shows " (pads)". */
  inputs: string[]
  synth: SynthState | null
  engine: { realtime: boolean; wakeP99Us: number; chordP99Us: number; midiInP99Us: number }
  lastControl: number
  /** e.g. "unmapped CC 103 = 127"; empty if none. */
  unmapped: string
  offline: boolean
}

// ── Provisional: the Launchkey surface (the follow-up API PR after #71) ──────
// Everything the mirror needs beyond the pads, so no button's function is hard-coded in
// the UI. Not in the engine yet: `surfaceOf()` (lib/surface.ts) derives it until the
// engine sends `state.surface`, and the mock sends it already. Rename fields here and in
// lib/surface.ts to whatever the API PR settles on.

/** The Launchkey's non-pad controls, in hardware terms. */
export type ControlId =
  | 'padBankUp' | 'padBankDown' | 'trackPrev' | 'trackNext' | 'play' | 'stop' | 'scene' | 'function'
  | 'faderButton1' | 'faderButton2' | 'faderButton3' | 'faderButton4'
  | 'faderButton5' | 'faderButton6' | 'faderButton7' | 'faderButton8' | 'masterButton'

export interface SurfaceControl {
  id: ControlId
  /** What it does now, and with Shift held ('' and null: nothing). */
  label: string
  action: AppCmd | null
  shiftLabel: string
  shiftAction: AppCmd | null
  /** Its light, as a Pad's (0–127 colour, level, animation). */
  rgb: Rgb
  level: Level
  anim: Anim
}

export interface SurfaceFader {
  /** What the fader controls on the active page ('' = unused), and the level. */
  label: string
  value: number | null
  /** The level is waiting for the hardware fader (soft takeover). */
  waiting: boolean
  /** Where the hardware fader physically is (0–127), if known. */
  position: number | null
  /** What moving it sends: `set` with `volume` filled in (setPartVolume, setStylePartVolume, setMasterVolume). */
  set: Extract<AppCmd, { volume: number }> | null
}

/** A library entry next to the loaded style. */
export interface Neighbour {
  id: number
  name: string
  path: string
}

export interface SurfaceState {
  /** Shift is held on the Launchkey. */
  shift: boolean
  /** Pad Bank ▲/▼, Track ◀/▶, Play, Stop, Scene/Function, the 8 fader buttons, master button. */
  controls: SurfaceControl[]
  /** Faders 1–8 and master, for the active fader page. */
  faders: SurfaceFader[]
  /** The styles Track ◀/▶ would load (the engine sends `{ id, name, path }`). */
  trackPrev: Neighbour | null
  trackNext: Neighbour | null
  /** The beat clock the LEDs run on: position at `atMs` (engine clock, ms), and tempo. */
  clock: { bar: number; beat: number; phase: number; tempo: number; atMs: number }
}

// ── Provisional: the keyboard (NEED on the board) ────────────────────────
// What the keyboard strip under the mirror shows. Not in the engine yet; the mock sends it.

/** A key held on the controller now. */
export interface HeldNote {
  /** MIDI note as played (after the controller's octave, before Keyboard transpose). */
  note: number
  /** Which side of the split it's on: 'left' = the chord section / Left part. */
  zone: 'left' | 'right'
  /** The keyboard parts sounding it (0–3 = Right 1, Right 2, Right 3, Left); empty if none
   * (a left-hand key that only feeds chord detection). */
  parts: number[]
}

export interface KeyboardState {
  /** Keys held now, low to high. */
  held: HeldNote[]
  /** Split Point (Left): keys at or below it play the Left part. `chord.split` is the
   * style's (chord detection) split. The engine uses one split for both today. */
  leftSplit: number
  /** Pitch classes (0–11, C = 0) of the recognised chord, root first; empty for none. */
  chordTones: number[]
  /** The bass the style plays (pitch class): the root, or the slash / on-bass note. */
  chordBass: number | null
}

export interface AppState {
  version: number
  style: StyleState
  transport: TransportState
  chord: ChordState
  /** Right 1, Right 2, Right 3, Left. */
  keyboardParts: KeyboardPart[]
  mixer: MixerState
  pads: PadsState
  ots: OtsState
  library: LibraryStatus
  io: IoState
  message: { seq: number; text: string; error: boolean } | null
  /** Provisional (see SurfaceState); absent from the engine until the follow-up API PR. */
  surface?: SurfaceState
  /** Provisional (see KeyboardState): held keys and chord tones for the keyboard strip. */
  keyboard?: KeyboardState
}

export interface LibraryEntry {
  id: number
  name: string
  /** Relative to the scanned root, `/`-separated: the category. */
  folder: string
  path: string
  status: 'pending' | 'ok' | 'error'
  error: string | null
  tempo: number | null
  timeSignature: [number, number] | null
  /** e.g. "Main ABCD · Intro ABC · Ending ABC · Fill ABCD · Break". */
  sections: string
}

export interface LibraryList {
  revision: number
  entries: LibraryEntry[]
}

// ── Names the UI uses ─────────────────────────────────────────────────────

export const FINGERINGS: { id: Fingering; name: string; short: string }[] = [
  { id: 'singleFinger', name: 'Single Finger', short: 'Single' },
  { id: 'fingered', name: 'Fingered', short: 'Fingered' },
  { id: 'fingeredOnBass', name: 'Fingered On Bass', short: 'On Bass' },
  { id: 'multiFinger', name: 'Multi Finger', short: 'Multi' },
  { id: 'aiFingered', name: 'AI Fingered', short: 'AI Fing.' },
  { id: 'fullKeyboard', name: 'Full Keyboard', short: 'Full Kbd' },
  { id: 'aiFullKeyboard', name: 'AI Full Keyboard', short: 'AI Full' },
]

export const PAD_PAGES: { id: PadPage; name: string }[] = [
  { id: 'sections', name: 'Sections' },
  { id: 'chordSetup', name: 'Chord/Setup' },
  { id: 'otsParts', name: 'OTS/Parts' },
]

/** Section names as the engine reports them. */
export const INTROS = ['Intro A', 'Intro B', 'Intro C']
export const MAINS = ['Main A', 'Main B', 'Main C', 'Main D']
export const FILLS = ['Fill In AA', 'Fill In BB', 'Fill In CC', 'Fill In DD']
export const BREAK = 'Fill In BA'
export const ENDINGS = ['Ending A', 'Ending B', 'Ending C']

/** A section as the panel labels it: "Intro A" → "Intro I", "Fill In BA" → "Break". */
export function sectionLabel(name: string): string {
  const roman: Record<string, string> = { A: 'I', B: 'II', C: 'III', D: 'IV' }
  if (name === BREAK) return 'Break'
  const m = /^(Intro|Ending) ([A-D])$/.exec(name)
  if (m) return `${m[1]} ${roman[m[2]]}`
  const f = /^Fill In ([A-D])\1$/.exec(name)
  if (f) return `Fill ${f[1]}`
  return name
}

export const STYLE_PART_NAMES = ['Rhythm 1', 'Rhythm 2', 'Bass', 'Chord 1', 'Chord 2', 'Pad', 'Phrase 1', 'Phrase 2']
export const KEYBOARD_PART_NAMES = ['Right 1', 'Right 2', 'Right 3', 'Left']
