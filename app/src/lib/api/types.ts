// The app's view of the engine: `AppState` (a snapshot of everything the UI shows),
// `AppCmd` (every action) and the library list. These mirror #16's `src/api.rs` field for
// field (docs/app-api.md is the contract): camelCase JSON, commands tagged by `type`.
//
// Tracks the draft PR #71 (branch m3/engine-api, commit 8df4f31). If the draft changes,
// change this file and `app/src-tauri/src/api.rs` to match; components only see these
// types and the `Session` interface.

import type { PlaylistCmd, PlaylistState, RegistrationCmd, RegistrationState } from './registration'
import type { SoundLibraryCmd, SoundLibraryState } from './sound-library'
import type { SoundsCmd, SoundsState } from './sounds'
export type * from './sound-library'
export type * from './sounds'

export type Fingering =
  | 'singleFinger' | 'multiFinger' | 'fingered' | 'fingeredOnBass'
  | 'aiFingered' | 'fullKeyboard' | 'aiFullKeyboard'

/** The Launchkey pad pages, switched with Pad Bank ▲/▼. */
export type PadPage = 'sections' | 'chordSetup' | 'otsParts' | 'registration' | 'multiPads'

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
  /** Fill Down (-1), Fill Self (0), Fill Up (1): the fill, then the Main to the left,
   * the same one or the one to the right. */
  | { type: 'fill'; delta: number }
  | { type: 'ending'; index: number }
  | { type: 'startStop' }
  | { type: 'stop' }
  | { type: 'toggleSyncStart' }
  | { type: 'toggleSyncStop' }
  | { type: 'toggleAutoFill' }
  | { type: 'toggleStopAcmp' }
  /** Stop Accompaniment mode (Style Setting > Stop ACMP). */
  | { type: 'setStopAcmp'; mode: StopAcmpMode }
  /** Genos assignable fill functions: a fill, then the Main to the right / left; the
   * Main's own fill; the Break. */
  | { type: 'fillUp' }
  | { type: 'fillDown' }
  | { type: 'fillSelf' }
  | { type: 'fillBreak' }
  /** Half Bar Fill In. */
  | { type: 'toggleHalfBarFill' }
  | { type: 'setHalfBarFill'; on: boolean }
  | { type: 'tapTempo' }
  | { type: 'tempoUp' }
  | { type: 'tempoDown' }
  /** FADE IN/OUT: stopped, arm a fade in; playing, fade out and stop (`transport.fade`). */
  | { type: 'toggleFade' }
  /** Style Section Reset: the section playing starts again from its top, now. */
  | { type: 'sectionReset' }
  /** Style Retrigger on/off (`transport.retrigger`). */
  | { type: 'toggleRetrigger' }
  /** Tempo in BPM, 5–500 (clamped). */
  | { type: 'setTempo'; bpm: number }
  | { type: 'toggleStylePart'; part: number }
  | { type: 'setStylePartVolume'; part: number; volume: number }
  | { type: 'setStyleVolume'; volume: number }
  | { type: 'setMultiPadVolume'; volume: number }
  /** Solo a Style part 0–7 (only it plays, even if off); null ends the solo. */
  | { type: 'setStyleSolo'; part: number | null }
  /** Style Track Mute (a Genos Live Control knob): `value` 0–127 turns parts on in `order`. */
  | { type: 'styleTrackMute'; order: TrackMuteOrder; value: number }
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
  /** The chord-settle window, ms (0–`CHORD_SETTLE_MAX_MS`). */
  | { type: 'setChordSettle'; ms: number }
  /** LEFT HOLD: Left rings on after its keys are let go, until its next key, a stop, or off. */
  | { type: 'setLeftHold'; on: boolean }
  | { type: 'toggleLeftHold' }
  // Keyboard parts
  | { type: 'setPartOn'; part: number; on: boolean }
  | { type: 'togglePart'; part: number }
  | { type: 'selectPart'; part: number }
  | { type: 'setPartVoice'; part: number; program: number }
  | { type: 'stepVoice'; delta: number }
  | { type: 'setPartVolume'; part: number; volume: number }
  | { type: 'setPartOctave'; part: number; octave: number }
  | { type: 'setPartPan'; part: number; pan: number }
  | { type: 'setPartSend'; part: number; send: PartSend; value: number }
  /** Solo a keyboard part 0–3 (only it sounds from the keys); null ends the solo. */
  | { type: 'setPartSolo'; part: number | null }
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
  /** OTS Link Timing: recall as the Main is pressed, or when it starts playing. */
  | { type: 'setOtsLinkTiming'; timing: OtsLinkTiming }
  // Style Setting > Change Behavior
  | { type: 'setTempoChange'; rule: ChangeRule }
  | { type: 'setPartsChange'; rule: ChangeRule }
  /** The Main (0–3) a style chosen while stopped starts on; null = Off. */
  | { type: 'setSectionSet'; section: number | null }
  /** Assignable "Style Tempo Lock/Reset" and "Style Tempo Hold/Reset". */
  | { type: 'toggleStyleTempoLock' }
  | { type: 'toggleStyleTempoHold' }
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
  /** Make a `.sf2` in the folder (`io.soundFonts`) the default sound set; as
   * `setDefaultSoundSet` with a file (kept for older clients). */
  | { type: 'setSoundFont'; file: string }
  /** The default sound set (#117): the SoundFont whatever the program map leaves unmapped
   * plays. A file in the folder, or null for Auto (`io.autoSoundSet`). Saved; a new font
   * loads in the background (`io.soundFontLoading`). */
  | { type: 'setDefaultSoundSet'; file: string | null }
  /** Keyboard sources: every one (`all`), or those whose name contains one of `names`
   * (`all` false, no names: a Launchkey's keys, else every source). */
  | { type: 'setMidiInputs'; all: boolean; names: string[] }
  /** Launchkey LEDs in Novation palette colours instead of RGB. */
  | { type: 'setPaletteLeds'; on: boolean }
  /** The synth's audio buffer, 64, 128 or 256 frames (`io.synth.bufferFrames`). The
   * output reopens; voices, plugins and held notes carry over. */
  | { type: 'setAudioBuffer'; frames: 64 | 128 | 256 }
  /** Re-walk the style folders (`library.roots`); `library.scanning` while it runs. */
  | { type: 'rescanLibrary' }
  // iReal Pro chart player: see ChartState below.
  | ChartCmd
  // Style settings (`styleSettings`)
  | StyleSettingsCmd
  // Registration Memory and the Playlist (lib/api/registration.ts)
  | RegistrationCmd
  | PlaylistCmd
  // Chord Looper (docs/chord-looper.md)
  | { type: 'looperRec' }
  | { type: 'looperOnOff' }
  | { type: 'selectLooperMemory'; index: number }
  | { type: 'storeLooperMemory'; index: number }
  | { type: 'clearLooperMemory'; index: number }
  | { type: 'newLooperBank' }
  /** Save the bank: to its file (`name` null), or as a new file named `name` (Save As). */
  | { type: 'saveLooperBank'; name: string | null; overwrite?: boolean }
  /** Load a bank file (`looper.banks`): its memories replace the eight. */
  | { type: 'loadLooperBank'; path: string }
  // Metronome: the built-in synth's click voice, never on the MIDI port.
  | { type: 'toggleMetronome' }
  | { type: 'setMetronome'; on: boolean }
  | { type: 'setMetronomeVolume'; volume: number }
  | { type: 'setMetronomeBell'; on: boolean }
  | MultiPadCmd
  // Controllers: pedals, wheels, assignable functions (docs/controllers.md)
  | ControllersCmd
  | PluginCmd
  // Sound library: patches, the program map (docs/sound-library.md)
  | SoundLibraryCmd
  // The sound catalog (#117): favourites, audition, assigning a sound to a part
  | SoundsCmd
  // Keyboard Harmony / Arpeggio (docs/app-api.md): see HarmonyArpState below.
  | HarmonyArpCmd
  // Parameter Lock: groups that Registration, OTS and Playlist recalls leave alone.
  | { type: 'setParamLock'; item: LockItem; on: boolean }
  // Style Dynamics Control, Touch and Accent (#180): see DynamicsState below.
  | DynamicsCmd
  // Knob Assign pages for the Launchkey's encoders (#197): see KnobsState below.
  | KnobsCmd
  // The effect bus (#204): see EffectsState below.
  | FxCmd

/** The effect bus's blocks (#204; docs/app-api.md › Effects). */
export type FxCmd =
  | { type: 'setEffectType'; block: FxBlock; effect: FxType }
  | { type: 'setEffectReturn'; block: FxBlock; level: number }
  /** #236: every Style part's send to the block scaled, 0-127 % (100 = as the style wrote it). */
  | { type: 'setBandSend'; block: FxBlock; level: number }
  /** #236: one of the block's parameters, in its own unit (see FxParamState). */
  | { type: 'setEffectParam'; block: FxBlock; param: FxParam; value: number }

/**
 * Reverb: reverbTime (0.1 s), preDelay (ms), reverbTone (100 Hz). Variation (delay): delaySync
 * (0/1), delayNote (0-7: 1/16 … 1/2), delayTime (ms), delayFeedback (%), delayTone (100 Hz),
 * pingPong (0/1).
 */
export type FxParam =
  | 'reverbTime' | 'preDelay' | 'reverbTone'
  | 'delaySync' | 'delayNote' | 'delayTime' | 'delayFeedback' | 'delayTone' | 'pingPong'

/** One effect parameter (#236). */
export interface FxParamState {
  param: FxParam
  /** "Time". */
  name: string
  /** In the parameter's own unit, min–max. */
  value: number
  min: number
  max: number
  /** Where the block's type starts it (a type change goes back to it). */
  default: number
  /** "2.4 s". */
  display: string
}

export type FxBlock = 'reverb' | 'chorus' | 'variation'
/** Reverb: hall, room, stage, plate. Chorus: chorus, celeste, flanger. Variation (tempo delay): eighth, dottedEighth, quarter, pingPong. */
export type FxType =
  | 'hall' | 'room' | 'stage' | 'plate'
  | 'chorus' | 'celeste' | 'flanger'
  | 'eighth' | 'dottedEighth' | 'quarter' | 'pingPong'

/** The effect bus: Reverb, Chorus and Variation, in that order. */
export interface EffectsState {
  blocks: EffectBlockState[]
}

export interface EffectBlockState {
  block: FxBlock
  /** "Reverb". */
  name: string
  effect: FxType
  /** "Hall", "Delay 1/8.". */
  effectName: string
  /** The block's own types. */
  types: { effect: FxType; name: string }[]
  /** 0-127: 64 = 0 dB, 127 = +6 dB, 0 = off. */
  returnLevel: number
  /** The band send (#236): every Style part's send to this block scaled, 0-127 % (100 = as written). Reverb 100, Chorus 0, Variation 0 at start. */
  bandSend: number
  /** Its parameters (#236), in order. A 0–1 parameter is a switch. */
  params: FxParamState[]
}

/** Knob Assign pages (#197; docs/app-api.md › Knob Assign pages). */
export type KnobsCmd =
  | { type: 'setKnobPage'; page: KnobPage }
  | { type: 'stepKnobPage'; delta: number }
  | { type: 'turnKnob'; knob: number; delta: number }

export type KnobPage = 'style' | 'parts' | 'pan' | 'effects'
export type KnobFunction =
  | 'none'
  | 'dynamics'
  | 'retriggerRate'
  | 'retriggerOnOff'
  | 'trackMuteA'
  | 'trackMuteB'
  | 'tempo'
  | 'partVolume'
  | 'harmonyVolume'
  | 'metronomeVolume'
  | 'partPan'
  | 'partReverb'
  | 'partChorus'
  | 'fxReturn'

/** The Knob Assign page and its eight knobs. */
export interface KnobsState {
  page: KnobPage
  pageName: string
  /** 1-based. */
  pageNumber: number
  pageCount: number
  /** Knobs 1-8. */
  knobs: KnobState[]
}

export interface KnobState {
  function: KnobFunction
  /** "Dynamics Control"; `short` is up to 8 characters ("DynCtrl", "---"). */
  name: string
  short: string
  /** The value as text ("64", "1/8", "On", "3 of 8", "120 BPM"); empty for No Assign. */
  value: string
  /** Where the knob is, 0-127 (the LED ring); null for tempo and No Assign. */
  level: number | null
}

/** Style Dynamics Control, Touch and Accent (#180; docs/app-api.md › Style Dynamics). */
export type DynamicsCmd =
  | { type: 'setDynamicsControl'; on: boolean }
  | { type: 'setDynamics'; level: number }
  | { type: 'stepDynamics'; delta: number }
  | { type: 'setDynamicsTouch'; on: boolean }
  | { type: 'toggleDynamicsTouch' }
  | { type: 'setAccent'; on: boolean }
  | { type: 'toggleAccent' }
  | { type: 'setAccentThreshold'; velocity: number }

/** Style Dynamics: System settings, not in Registration. */
export interface DynamicsState {
  /** Style Setting › Dynamics Control: the level acts on the Style. */
  control: boolean
  /** The level in effect, 0-127 (64: as written); Touch moves it. */
  level: number
  /** Chord-section strikes set the level. */
  touch: boolean
  /** A hard chord-section strike plays the Main's fill. */
  accent: boolean
  /** The Accent threshold (velocity 1-127). */
  accentThreshold: number
}

/** A Parameter Lock group (the Genos Data List's lock groups that yahaha has). */
export type LockItem = 'splitPoint' | 'fingeringType'

/** Parameter Lock: true = locked (a recall leaves the group as the player set it). */
export interface ParamLockState {
  splitPoint: boolean
  fingeringType: boolean
}

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
  /** The Arpeggio Hold setting (RM p.41). */
  | { type: 'setArpHold'; on: boolean }
  | { type: 'toggleArpHold' }
  /** The Arpeggio Hold pedal function (RM p.141), apart from the setting. */
  | { type: 'setArpPedalHold'; on: boolean }
  | { type: 'toggleArpPedalHold' }
  /** `velocity` (1-127) is used by `fixed`. */
  | { type: 'setArpVelocity'; mode: ArpVelocityMode; velocity: number }
  | { type: 'setArpKeepKeyOn'; on: boolean }

/** Style Track Mute order (RM p.148). A: Rhythm 2 first; B: Chord 1 first. */
export type TrackMuteOrder = 'a' | 'b'

export type CmdError = { kind: 'busy' } | { kind: 'failed'; message: string }

/** Stop Accompaniment: what a chord sounds on with the band stopped and Sync Start off. */
export type StopAcmpMode = 'off' | 'style' | 'fixed'
/** OTS Link Timing: as the Main is pressed, or when that Main starts playing. */
export type OtsLinkTiming = 'immediate' | 'mainChange'
/** Change Behavior: keep the old style's value, keep it only while playing, or take the new one's. */
export type ChangeRule = 'lock' | 'hold' | 'reset'

/** Style Setting > Change Behavior. */
export interface StyleChangeState {
  tempo: ChangeRule
  parts: ChangeRule
  /** The Main (0–3) a style chosen while stopped starts on; null = Off (keep it). */
  sectionSet: number | null
}

/** Section Change Timing, To Main (and a style change while playing). */
export type MainTiming = 'immediate' | 'nextBar'
/** Section Change Timing, Inside Intro/Ending. */
export type IntroEndingTiming = 'nextBar' | 'endOfSection'
/** The Style Retrigger lengths: a whole note .. a 32nd. */
export const RETRIGGER_RATES = [1, 2, 4, 8, 16, 32] as const

/** Genos Style Setting, Tap Tempo › Style Section Reset, Fade and Retrigger settings. */
export type StyleSettingsCmd =
  | { type: 'setMainTiming'; timing: MainTiming }
  | { type: 'setIntroEndingTiming'; timing: IntroEndingTiming }
  /** 0 = Off, up to 5000. */
  | { type: 'setSyncStopWindow'; ms: number }
  /** 0–20000. */
  | { type: 'setFadeInTime'; ms: number }
  | { type: 'setFadeOutTime'; ms: number }
  /** 0–5000. */
  | { type: 'setFadeHoldTime'; ms: number }
  | { type: 'setSectionReset'; on: boolean }
  /** 1, 2, 4, 8, 16 or 32. */
  | { type: 'setRetriggerRate'; rate: number }
  /** Positive: shorter. */
  | { type: 'stepRetriggerRate'; delta: number }

export interface StyleSettingsState {
  mainTiming: MainTiming
  introEndingTiming: IntroEndingTiming
  /** Synchro Stop Window; 0 = Off. */
  syncStopWindowMs: number
  fadeInMs: number
  fadeOutMs: number
  fadeHoldMs: number
  /** TAP TEMPO while playing rewinds the section (else sets the tempo). */
  sectionReset: boolean
  /** 1, 2, 4, 8, 16 or 32. */
  retriggerRate: number
}

/** Fade In/Out: armed = stopped, START fades in; holding = faded out, silent for the hold. */
export type FadeState = 'off' | 'armed' | 'fadingIn' | 'fadingOut' | 'holding'

/** Notifications; they carry no state (read `state()` / `library()`). */
export type SessionEvent =
  | { type: 'stateChanged'; version: number }
  | { type: 'libraryChanged'; revision: number }
  | { type: 'soundsChanged'; revision: number }
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
  /** Stop Accompaniment sounds the chord (`stopAcmpMode` is not 'off'). */
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
  /** Half Bar Fill In. */
  halfBarFill: boolean
  stopAcmpMode: StopAcmpMode
  /** Fade In/Out. */
  fade: FadeState
  /** Style Retrigger is on. */
  retrigger: boolean
  /** The Ending is slowing down (pressed again while it plays). */
  ritardando: boolean
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
  /** The chord-settle window in ms: while the style plays, a chord change reaches the
   * accompaniment once the chord has held still this long (a rolled chord is followed once). */
  settleMs: number
  /** Left Hold (`setLeftHold`). */
  leftHold: boolean
}

/** The widest chord-settle window, ms (`setChordSettle`). */
export const CHORD_SETTLE_MAX_MS = 30

/** A keyboard part's effect send (`setPartSend`): reverb (CC 91), chorus (CC 93) or variation, the tempo delay (CC 94). */
export type PartSend = 'reverb' | 'chorus' | 'variation'

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
  /** Pan (CC 10): 0 left, 64 centre, 127 right. 64 until something sets it. */
  pan: number
  /** Reverb send depth (CC 91); 50 (Left 40) until something sets it. */
  reverb: number
  /** Chorus send depth (CC 93); 10 until something sets it. */
  chorus: number
  /** Variation (tempo delay) send depth (CC 94); 0 until something sets it. */
  variation: number
  /** Where its Launchkey fader (Panel page, faders 1–4) physically is; null until it moves. */
  fader: number | null
  /** The instrument plugin it plays instead of its SoundFont voice (absent: the SoundFont). */
  plugin?: PartPlugin
  /** Its own sound library patch (`setPartPatch`); null: its GM voice, through the map. */
  patch: string | null
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
  /** The Style volume (Panel fader 5): 100 = the Style parts' CC 7 as written. */
  styleVolume: number
  /** Panel fader 5 hasn't reached `styleVolume` yet. */
  styleVolumeWaiting: boolean
  /** The Multi Pad volume (Panel fader 6): 100 = the pads' CC 7 as written. */
  multiPadVolume: number
  /** Panel fader 6 hasn't reached `multiPadVolume` yet. */
  multiPadVolumeWaiting: boolean
  /** The Style part soloed (0–7), or null. */
  styleSolo: number | null
  /** The keyboard part soloed (0–3), or null. */
  partSolo: number | null
}

/** Where the Chord Looper is. */
export type LooperMode = 'off' | 'recArmed' | 'recording' | 'loopArmed' | 'looping'

export interface LoopChord {
  /** 1-based bar of the sequence. */
  bar: number
  /** 1-based beat in quarter notes (2.5 = the "and" of 2). */
  beat: number
  chord: string
}

export interface LooperMemory {
  /** "CLD_001"…; null when empty. */
  name: string | null
  bars: number
  chords: LoopChord[]
}

export interface LooperState {
  mode: LooperMode
  hasData: boolean
  /** Recording: the bar recorded; looping: the loop's bar (1-based). */
  bar: number | null
  bars: number
  /** The current sequence (empty while recording). */
  chords: LoopChord[]
  memory: number | null
  pendingMemory: number | null
  /** Always 8. */
  memories: LooperMemory[]
  /** The bank's name ("New Bank" until saved or loaded), its file (null: unsaved), and the
   * bank files in the ChordLooper folder. */
  bankName: string
  bankPath: string | null
  banks: { name: string; path: string }[]
}

export interface MetronomeState {
  on: boolean
  /** 0–127. */
  volume: number
  bell: boolean
  /** The built-in synth runs (the only place the click sounds). */
  audible: boolean
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
  /** When OTS Link recalls during playback. */
  linkTiming: OtsLinkTiming
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
  /** The `.sf2` files in the SoundFont folder. */
  soundFonts: string[]
  /** The file the synth plays as its default sound set; null without the synth. */
  soundFontFile: string | null
  /** A `setDefaultSoundSet` is loading. */
  soundFontLoading: boolean
  /** The default sound set chosen; null: Auto. */
  defaultSoundSet: string | null
  /** The font Auto picks: the most GM-complete in the folder (null: no fonts). */
  autoSoundSet: string | null
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

// ── iReal Pro chart player (#89) ─────────────────────────────────────────
// The band takes its chords and Mains from an iReal chart instead of the left hand
// (docs/app-api.md "iReal Pro chart player", docs/ireal.md "Chart player").

export type ChartCmd =
  /** Import playlists from an `irealb://` link or an exported `.html` playlist's text. */
  | { type: 'importCharts'; text: string }
  /** The same, reading a file (Tauri / terminal). */
  | { type: 'importChartFile'; path: string }
  /** Choose the chart (loads its suggested style with `autoStyle`; stopped: its tempo). */
  | { type: 'selectChart'; playlist: number; song: number }
  | { type: 'stepChart'; delta: number }
  | { type: 'removeChartPlaylist'; playlist: number }
  | { type: 'setChartMode'; on: boolean }
  | { type: 'toggleChartMode' }
  /** 1–99 times through the form. */
  | { type: 'setChartChoruses'; choruses: number }
  /** Loop bars [start, end) of `chart.song.bars`; null: no loop. */
  | { type: 'setChartLoop'; range: [number, number] | null }
  /** Intro / Ending 0–2 (A–C) around the chart; null: none. */
  | { type: 'setChartIntro'; index: number | null }
  | { type: 'setChartEnding'; index: number | null }
  | { type: 'setChartAutoStyle'; on: boolean }

export interface ChartSongInfo {
  title: string
  /** As iReal stores it, usually "Last First". */
  composer: string
  /** iReal's style label ("Medium Swing", "Bossa Nova"). */
  style: string
  /** "C", "Eb", "A-" (minor). */
  key: string
  tempo: number | null
}

export interface ChartBar {
  /** "A", "B", "V", "i", or null before any mark. */
  section: string | null
  sectionStart: boolean
  /** The Main it plays, 0–3. */
  main: number
  time: [number, number]
  /** 1-based. */
  chorus: number
  /** Chords on beats (0-based); a bar with none holds the chord before. */
  chords: { beat: number; name: string }[]
}

export interface ChartSection {
  label: string
  chorus: number
  start: number
  bars: number
}

export interface ChartSong extends ChartSongInfo {
  /** The form played `choruses` times through. */
  bars: ChartBar[]
  sections: ChartSection[]
}

export interface ChartState {
  /** Chart mode: the chart gives the chords while the band plays. */
  on: boolean
  playlists: { name: string; songs: ChartSongInfo[] }[]
  /** [playlist, song] */
  selected: [number, number] | null
  song: ChartSong | null
  choruses: number
  intro: number | null
  ending: number | null
  /** Bars [start, end) looped, or null. */
  loop: [number, number] | null
  autoStyle: boolean
  /** The library style the chart's label suggests (`LibraryEntry.id`). */
  suggestedStyle: number | null
  /** The bar of `song.bars` playing; null stopped, in the Intro/Ending or chart mode off. */
  bar: number | null
  /** Your chord has taken over until the next bar line. */
  overridden: boolean
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
  /** Style Setting > Change Behavior. */
  styleChange: StyleChangeState
  /** The Launchkey beyond the pads: controls, Shift, faders, Track neighbours, clocks. */
  surface: SurfaceState
  /** The keys held and the chord, for the keyboard strip. */
  keyboard: KeyboardState
  /** The style preview and the style waiting for the bar line. */
  preview: PreviewState
  /** The iReal Pro chart player. */
  chart: ChartState
  /** Section Change Timing, Synchro Stop Window, fade times, Section Reset, Retrigger length. */
  styleSettings: StyleSettingsState
  /** Registration Memory: the bank, its ten buttons, Freeze, the Registration Sequence. */
  registration: RegistrationState
  /** The Playlist. */
  playlist: PlaylistState
  /** The Chord Looper. */
  looper: LooperState
  metronome: MetronomeState
  /** Multi Pads: the bank, the four pads, Synchro Stop, the bank files. */
  multiPad: MultiPadState
  /** Pedals, wheels, the parts they reach and the pedals' assignable functions. */
  controllers: ControllersState
  /** The instrument plugin host (docs/plugin-hosting.md). */
  plugins: PluginsState
  /** Keyboard Harmony / Arpeggio. */
  harmonyArp: HarmonyArpState
  /** The sound library: patches, the program map, what the current style uses. */
  soundLibrary: SoundLibraryState
  /** Parameter Lock: the locked groups. */
  paramLocks: ParamLockState
  /** The sound catalog's summary (#117); the list is `session.sounds()`. */
  sounds: SoundsState
  /** Style Dynamics Control, Touch and Accent (#180). */
  dynamics: DynamicsState
  /** Knob Assign pages for the Launchkey's encoders (#197). */
  knobs: KnobsState
  /** The effect bus's Reverb, Chorus and Variation blocks (#204). */
  effects: EffectsState
}

// ── Instrument plugins (docs/plugin-hosting.md) ──────────────────────────

export type PluginCmd =
  /** Play a keyboard part (0-3) on a plugin from `plugins.list`; `state` a saved preset (base64). */
  | { type: 'setPartPlugin'; part: number; id: string; state: string | null }
  /** Back to the part's SoundFont voice. */
  | { type: 'clearPartPlugin'; part: number }
  /** Keep the plugin's current preset with the part (send when its editor closes). */
  | { type: 'savePartPluginState'; part: number }
  /** Scan the installed instruments again. */
  | { type: 'rescanPlugins' }
  /** Run plugin `id` in yahaha's process (true) or its own (false); from its next load. */
  | { type: 'setPluginInProcess'; id: string; inProcess: boolean }
  /** Load a part's plugin again after it stopped or failed (null: the selected part). */
  | { type: 'reloadPartPlugin'; part: number | null }

/** loading: still on the SoundFont; failed: back on it; muted: the plugin crashed. */
export type PluginStatus = 'loading' | 'playing' | 'failed' | 'muted'

export interface PartPlugin {
  id: string
  name: string
  manufacturer: string
  status: PluginStatus
  /** While loading: queued, instantiating, initializing, restoringState. */
  stage: string | null
  error: string | null
  outOfProcess: boolean
  /** The system refused to host it in its own process, so it loaded in yahaha's process
   * instead: a crash in it takes yahaha down. */
  inProcessFallback: boolean
  /** Share of real time (0.05 = 5% of a core), once a second. */
  cpu: number
  overruns: number
  /** Overruns in the last 10 seconds (once a second): the live readout. */
  recentOverruns: number
  /** Its editor window can be opened. */
  editor: boolean
}

export interface PluginEntry {
  /** "aumu dls  appl": what setPartPlugin takes. */
  id: string
  name: string
  manufacturer: string
  version: string
  format: 'AUv2' | 'AUv3'
  lastError: string | null
  /** The player chose to run it in yahaha's process (setPluginInProcess). */
  inProcess: boolean
  /** It can run in yahaha's process: every AUv2, and an AUv3 that allows it. */
  canRunInProcess: boolean
}

export interface PluginsState {
  /** The build hosts plugins and the built-in synth runs. */
  available: boolean
  scanning: boolean
  list: PluginEntry[]
}

// ── Controllers (docs/controllers.md) ────────────────────────────────────

/** An assignable function's id: a row of `assignable-functions.json` (`AssignableFunction`). */
export type FunctionId = string

/** Sustain, Sostenuto, Soft: how the pedal drives them. */
export type ControlType = 'holdA' | 'holdB' | 'toggle'
/** Which half of the bend a Pitch Bend pedal sweeps. */
export type BendRange = 'upper' | 'lower' | 'full'

/** A row of the assignable-function table (app/src/lib/api/assignable-functions.json). */
export interface AssignableFunction {
  id: FunctionId
  name: string
  category: 'voice' | 'style' | 'ots' | 'registration' | 'overall' | 'chordLooper'
  /** switch: Control Type applies; trigger: fires on the press; continuous: an expression pedal. */
  kind: 'switch' | 'trigger' | 'continuous'
  /** yahaha has it (Registration Bank +/− not yet). */
  available: boolean
}

export type ControllersCmd =
  /** Pedal 0-2: the CC it listens for (null: none), its function, Control Type, polarity, Range. */
  | { type: 'setPedal'; pedal: number; cc: number | null; function: FunctionId; controlType: ControlType; reverse: boolean; range: BendRange }
  /** The pedal takes the CC of the next pedal pressed on a keyboard; null stops. */
  | { type: 'learnPedal'; pedal: number | null }
  /** Which controllers reach a keyboard part (0-3). */
  | { type: 'setPartControllers'; part: number; sustain: boolean; pitchBend: boolean; modulation: boolean }
  /** A keyboard part's Pitch Bend Range, 0-12 semitones. */
  | { type: 'setBendRange'; part: number; semitones: number }
  /** Run an assignable function now, as a pedal press would. */
  | { type: 'triggerFunction'; function: FunctionId }

export interface PedalState {
  cc: number | null
  function: FunctionId
  controlType: ControlType
  reverse: boolean
  range: BendRange
  /** Held down now. */
  down: boolean
}

export interface PartControllers {
  /** The pedal switches (sustain, sostenuto, soft) reach it. */
  sustain: boolean
  pitchBend: boolean
  modulation: boolean
  /** Semitones, 0-12. */
  bendRange: number
}

export interface ControllersState {
  /** Always 3. */
  pedals: PedalState[]
  /** The pedal learning its CC, or null. */
  learning: number | null
  /** Right 1, Right 2, Right 3, Left. */
  parts: PartControllers[]
  sustain: boolean
  sostenuto: boolean
  soft: boolean
}

// ── Multi Pads (docs/app-api.md "Multi Pads", docs/multipad.md) ───────────────

/** Pads are 0–3. */
export type MultiPadCmd =
  /** Load a bank from `multiPad.banks`. */
  | { type: 'loadMultiPad'; id: number }
  /** Load any `.pad` file (added to `multiPad.banks`). */
  | { type: 'loadMultiPadPath'; path: string }
  /** No bank: the pads go dark. */
  | { type: 'clearMultiPad' }
  /** Press a pad: at once when stopped, at the next bar line while the band plays. */
  | { type: 'triggerMultiPad'; pad: number }
  /** STOP + pad. */
  | { type: 'stopMultiPad'; pad: number }
  /** STOP: every pad, and Synchro Start standby. */
  | { type: 'stopAllMultiPads' }
  /** SELECT + pad: toggle Synchro Start standby. */
  | { type: 'armMultiPad'; pad: number }
  | { type: 'setMultiPadRepeat'; pad: number; on: boolean }
  | { type: 'setMultiPadChordMatch'; pad: number; on: boolean }
  /** Multi Pad Synchro Stop: repeating pads stop when the band stops / an Ending starts. */
  | { type: 'setMultiPadSynchroStop'; styleStop: boolean; ending: boolean }

/** A pad's lamp: off, blue, red flashing (Synchro Start), waiting for the bar line, red. */
export type PadLamp = 'empty' | 'ready' | 'armed' | 'queued' | 'playing'

export interface MultiPadPad {
  /** 0–3. */
  index: number
  /** From the bank file; empty for an empty pad. */
  name: string
  lamp: PadLamp
  repeat: boolean
  chordMatch: boolean
  /** The MIDI channel it plays on (5–8). */
  channel: number
}

export interface MultiPadBankEntry {
  id: number
  name: string
  /** Relative to the scanned root, `/`-separated. */
  folder: string
  path: string
}

export interface MultiPadState {
  /** The bank loaded; null when none. */
  bank: { id: number; name: string; path: string } | null
  /** A bank is on its way to the engine. */
  loading: boolean
  /** Always 4. */
  pads: MultiPadPad[]
  synchroStop: { styleStop: boolean; ending: boolean }
  /** The `.pad` files in the style folders, folder then name. */
  banks: MultiPadBankEntry[]
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
    /** The Hold setting. */
    hold: boolean
    /** The Arpeggio Hold pedal function is on; the arpeggio holds while either is. */
    pedalHold: boolean
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
  { id: 'registration', name: 'Registration' },
  { id: 'multiPads', name: 'Multi Pads' },
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
