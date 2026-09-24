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
  // Style preview and queue: see PreviewState below.
  | PreviewCmd
  // Settings (docs/app-api.md)
  /** Reload the synth from another `.sf2` in its folder (`io.soundFonts`); loads in the
   * background (`io.soundFontLoading`). */
  | { type: 'setSoundFont'; file: string }
  /** Keyboard sources: every one (`all`), or those whose name contains one of `names`
   * (`all` false, no names: a Launchkey's keys, else every source). */
  | { type: 'setMidiInputs'; all: boolean; names: string[] }
  /** Launchkey LEDs in Novation palette colours instead of RGB. */
  | { type: 'setPaletteLeds'; on: boolean }
  /** Re-walk the style folders (`library.roots`); `library.scanning` while it runs. */
  | { type: 'rescanLibrary' }
  // Keyboard Harmony / Arpeggio (docs/app-api.md): see HarmonyArpState below.
  | HarmonyArpCmd

/** Keyboard Harmony / Arpeggio: one HARMONY/ARPEGGIO switch and one type. */
export type HarmonyArpCmd =
  | { type: 'toggleHarmonyArp' }
  | { type: 'setHarmonyArpOn'; on: boolean }
  /** A Harmony type, by index into `LibraryList.harmonyTypes` (Data List order). */
  | { type: 'setHarmonyType'; index: number }
  /** An arpeggio pattern, by index into `LibraryList.arpPatterns`. */
  | { type: 'setArpPattern'; index: number }
  /** Step through the Harmony types, then the arpeggios, as one list (wrapping). */
  | { type: 'stepHarmonyArpType'; delta: number }
  | { type: 'setHarmonyVolume'; volume: number }
  | { type: 'setHarmonySpeed'; speed: HarmonySpeed }
  | { type: 'setHarmonyAssign'; assign: HarmonyAssign }
  | { type: 'setChordNoteOnly'; on: boolean }
  /** Minimum Velocity, 1-127. */
  | { type: 'setTouchLimit'; velocity: number }
  | { type: 'setArpQuantize'; quantize: ArpQuantize }
  | { type: 'setArpHold'; on: boolean }
  | { type: 'toggleArpHold' }
  /** `velocity` (1-127) is used by `fixed`. */
  | { type: 'setArpVelocity'; mode: ArpVelocityMode; velocity: number }
  | { type: 'setArpKeepKeyOn'; on: boolean }

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
  /** Palette-LED mode only (`pads.paletteLeds`): what the pad was sent. Null in RGB mode. */
  palette: PaletteLed | null
}

/** A pad as sent in Novation palette mode: a palette colour, solid, flashing between two
 * colours, or pulsing. `rgb`/`level` are the palette colour's look. */
export interface PaletteLed {
  mode: Anim
  colour: number
  rgb: Rgb
  level: Level
  /** Flash only: the second colour. */
  flashColour: number | null
  flashRgb: Rgb | null
  flashLevel: Level | null
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
  /** How many bars the section playing lasts (a Main's pattern length; it loops), for
   * the lead-sheet band's progress. Null when stopped. */
  sectionBars: number | null
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
  /** Where its Launchkey fader (Panel page, faders 1–4) physically is; null until it moves. */
  fader: number | null
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
  /** Where its Launchkey fader (Style page, faders 1–8) physically is; null until it moves. */
  fader: number | null
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
  /** The LEDs run in Novation palette mode (`setPaletteLeds`, `--palette-leds`): the
   * pads carry `palette`. */
  paletteLeds: boolean
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
  /** The style folders (and files) scanned. */
  roots: string[]
  /** A rescan (`rescanLibrary`) is running. */
  scanning: boolean
}

/** A MIDI source (`io.sources`). */
export interface MidiSource {
  /** As `setMidiInputs` matches it. */
  name: string
  /** yahaha listens to it (as a keyboard, or as the pads). */
  listening: boolean
  /** The Launchkey DAW port (pads, buttons, faders). */
  pads: boolean
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
  /** Every MIDI source, and whether yahaha listens to it. */
  sources: MidiSource[]
  /** Every source is a keyboard (`setMidiInputs { all: true }`). */
  allInputs: boolean
  /** The `.sf2` files in the synth's folder, for `setSoundFont`. */
  soundFonts: string[]
  /** The file the synth plays; null without the synth. */
  soundFontFile: string | null
  /** A `setSoundFont` is loading. */
  soundFontLoading: boolean
}

/** Output levels (`meters()`): peaks since the last call, linear (1 = full scale). The
 * client applies its own decay and peak hold. */
export interface Meters {
  atMs: number
  /** Keyboard parts (ch 1–4) and Style parts (ch 9–16), before the soft clipper. Empty
   * without the synth. */
  channels: { channel: number; peak: number }[]
  /** Left, right after the soft clipper. */
  master: [number, number]
  /** Audio buffers in which the soft clipper worked, since start. */
  clips: number
}

// ── The Launchkey surface (#77, docs/app-api.md "surface") ────────────────────
// Everything the mirror needs beyond the pads, so no button's function is hard-coded in
// the UI. The engine and both mocks send it.

/** The Launchkey's non-pad controls, in hardware terms. */
export type ControlId =
  | 'padBankUp' | 'padBankDown' | 'trackPrev' | 'trackNext' | 'play' | 'stop' | 'scene' | 'function'
  | 'faderButton1' | 'faderButton2' | 'faderButton3' | 'faderButton4'
  | 'faderButton5' | 'faderButton6' | 'faderButton7' | 'faderButton8' | 'masterButton'

export interface SurfaceControl {
  id: ControlId
  /** Its CC on the DAW port, channel 1. */
  cc: number
  /** What it does now, and with Shift held ('' and null: nothing). */
  label: string
  action: AppCmd | null
  shiftLabel: string
  shiftAction: AppCmd | null
  /** Its light, as a Pad's (0–127 colour, level, animation). */
  rgb: Rgb
  level: Level
  anim: Anim
  /** The palette index yahaha sends it; null for Play, Stop, Scene and Function (not driven). */
  colour: number | null
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
  /** The beat clocks (see ClockState). */
  clock: ClockState
}

/**
 * The clocks as anchors (docs/app-api.md, "surface.clock"). Times are the session's
 * monotonic clock in ms. Each anchor moves on at `tempo` until the next state:
 *   t   = atMs + (now − receivedMs)
 *   pos = running ? sectionAnchorBeats + (t − sectionAnchorMs)·tempo/60000 : 0
 *   led = ledAnchorBeats + (t − ledAnchorMs)·tempo/60000   (the pads' flash/pulse clock)
 */
export interface ClockState {
  /** The session clock when this state was read. */
  atMs: number
  running: boolean
  tempo: number
  /** Quarter notes per bar. */
  beatsPerBar: number
  /** The position at `atMs` (1-based; 1, 1 when stopped) and how far into the beat. */
  bar: number
  beat: number
  phase: number
  sectionAnchorMs: number
  sectionAnchorBeats: number
  ledAnchorMs: number
  ledAnchorBeats: number
}

// ── The keyboard (docs/app-api.md "keyboard") ─────────────────────────────
// What the keyboard strip under the mirror shows.

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
  /** The keys chord detection reads, as [lo, hi] MIDI notes (inclusive): up to the split
   * in Lower, above it in Upper, every key in the Full Keyboard types. */
  detection: [number, number]
}

// ── Style preview (#21) ─────────────────────────────────────────────────
// The browser auditions a style while the band is stopped, and queues one for the next
// bar line while it plays (docs/app-api.md). `loadStyle`/`stepStyle` while playing wait
// for the bar line too; `preview.queued` shows the style waiting.

export type PreviewCmd =
  /** Stopped only: plays the style's Main A over the default progression (one chord a
   * bar, `PreviewState.audition.bars` bars), on the style channels, then stops by itself.
   * The loaded style, OTS, mixer and transport don't change. Refused while the band runs. */
  | { type: 'auditionStyle'; id: number }
  /** Ends an audition at once (all notes off on the style channels). */
  | { type: 'stopAudition' }
  /** Playing: load the style at the next bar line (like `loadStyle` then). Stopped: the
   * same as `loadStyle`. A second `queueStyle` replaces the first. */
  | { type: 'queueStyle'; id: number }

export interface PreviewState {
  /** The style auditioning while the band is stopped; null when none. */
  audition: {
    id: number
    /** 1-based bar of the audition, and how many it plays. */
    bar: number
    bars: number
    /** The progression's chord playing now. */
    chord: string | null
  } | null
  /** The style `queueStyle` will load at the next bar line; null when none. */
  queued: number | null
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
  /** The Launchkey beyond the pads: controls, Shift, faders, Track neighbours, clocks. */
  surface: SurfaceState
  /** The keys held and the chord, for the keyboard strip. */
  keyboard: KeyboardState
  /** The style preview and the style waiting for the bar line. */
  preview: PreviewState
  /** Keyboard Harmony / Arpeggio. */
  harmonyArp: HarmonyArpState
}

export type HarmonySpeed = '1/4' | '1/6' | '1/8' | '1/12' | '1/16' | '1/32'
export type HarmonyAssign = 'auto' | 'multi' | 'right1' | 'right2' | 'right3'
export type ArpQuantize = 'off' | 'eighth' | 'sixteenth'
export type ArpVelocityMode = 'original' | 'thru' | 'fixed'

export interface HarmonyArpState {
  /** The HARMONY/ARPEGGIO switch. */
  on: boolean
  /** Which list the selected type is in. */
  mode: 'harmony' | 'arpeggio'
  /** Index into `LibraryList.harmonyTypes`; kept while an arpeggio is selected. */
  harmonyType: number
  /** Index into `LibraryList.arpPatterns`. */
  arpPattern: number
  /** The selected type's name and category ("Harmony", "Echo", "Up & Down", ...). */
  typeName: string
  category: string
  /** Volume of the added notes and of the arpeggio, 0-127. */
  volume: number
  /** Echo, Tremolo and Trill. */
  speed: HarmonySpeed
  assign: HarmonyAssign
  chordNoteOnly: boolean
  /** Minimum Velocity, 1-127. */
  touchLimit: number
  arp: {
    quantize: ArpQuantize
    hold: boolean
    velocity: ArpVelocityMode
    fixedVelocity: number
    keepKeyOn: boolean
  }
}

/** A Harmony type or an arpeggio pattern in `LibraryList`. */
export interface HarmonyTypeInfo {
  name: string
  category: string
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
  /** "SFF1" or "SFF2" from the file's header; null until indexed, or unreadable. */
  format: string | null
}

/** A voice `setPartVoice` can pick (a GM program on bank 0). */
export interface VoiceOption {
  program: number
  bankMsb: number
  bankLsb: number
  name: string
}

export interface LibraryList {
  revision: number
  entries: LibraryEntry[]
  /** The voices `setPartVoice` picks from (the same every revision). */
  voices: VoiceOption[]
  /** The Keyboard Harmony types `setHarmonyType` picks from, Data List order (static). */
  harmonyTypes: HarmonyTypeInfo[]
  /** The arpeggio patterns `setArpPattern` picks from (static). */
  arpPatterns: HarmonyTypeInfo[]
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
