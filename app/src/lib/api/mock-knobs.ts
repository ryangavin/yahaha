// The mock's Knob Assign pages (#197): a port of `src/knobs.rs` (the same pages, names,
// steps and readings). A turn gives back the command it runs, which the mock then runs as
// the session does. Only the mock uses this; with the real engine the knobs come in the state.

import type { AppCmd, AppState, ControlMap, ControlTarget, FxBlock, FxParam, FxParamState, KnobFunction, KnobPage, KnobState, KnobsState } from './types'
import { defaultControlMap } from './types'

type Fn = { fn: KnobFunction; part?: number; param?: FxParam }
const NONE: Fn = { fn: 'none' }

/** What a controller map target does on a knob (`knobs::rack_function`). */
export function rackFn(t: ControlTarget): Fn {
  switch (t.kind) {
    case 'partLevel': return { fn: 'partVolume', part: t.part }
    case 'partPan':
    case 'partReverb':
    case 'partChorus': return { fn: t.kind, part: t.part }
    case 'harmonyArp':
    case 'splitPoint':
    case 'harmonyVolume':
    case 'metronomeVolume':
    case 'tempo': return { fn: t.kind }
    default: return NONE
  }
}

/** The Rack page's knobs for controller map `m`. */
function rackFns(m: ControlMap): Fn[] {
  return m.knobs.slice(0, 8).map(rackFn)
}

/** A fader's route on the input thread for target `t` of fader `f` (`knobs::fader_routes`). */
export function faderRoute(t: ControlTarget, f: number): 'own' | 'off' | 'control' {
  if (t.kind === 'partLevel' && t.part === f) return 'own'
  return rackFn(t).fn === 'none' || t.kind === 'tempo' ? 'off' : 'control'
}

/** The command a fader at `v` runs for target `t` (`knobs::fader_command`), or null. */
export function faderCommand(t: ControlTarget, v: number, s: AppState): AppCmd | null {
  v = clamp(v, 0, 127)
  switch (t.kind) {
    case 'partLevel': return { type: 'setPartVolume', part: t.part, volume: v }
    case 'partPan': return { type: 'setPartPan', part: t.part, pan: v }
    case 'partReverb': return { type: 'setPartSend', part: t.part, send: 'reverb', value: v }
    case 'partChorus': return { type: 'setPartSend', part: t.part, send: 'chorus', value: v }
    case 'harmonyArp': return v >= 64 === s.harmonyArp.on ? null : { type: 'setHarmonyArpOn', on: v >= 64 }
    case 'splitPoint': return { type: 'setSplit', note: SPLIT_MIN + Math.floor((v * (SPLIT_MAX - SPLIT_MIN)) / 127) }
    case 'harmonyVolume': return { type: 'setHarmonyVolume', volume: v }
    case 'metronomeVolume': return { type: 'setMetronomeVolume', volume: v }
    default: return null
  }
}

const SPLIT_MIN = 24
const SPLIT_MAX = 96
const DEFAULT_SPLIT = 54
const NOTE_NAMES = ['C', 'C#', 'D', 'Eb', 'E', 'F', 'F#', 'G', 'Ab', 'A', 'Bb', 'B']
const noteName = (n: number) => `${NOTE_NAMES[n % 12]}${Math.floor(n / 12) - 2}`
const splitLevel = (n: number) => Math.floor(((clamp(n, SPLIT_MIN, SPLIT_MAX) - SPLIT_MIN) * 127) / (SPLIT_MAX - SPLIT_MIN))

const PAGES: Record<KnobPage, Fn[]> = {
  style: [{ fn: 'dynamics' }, { fn: 'retriggerRate' }, { fn: 'retriggerOnOff' }, { fn: 'trackMuteA' }, { fn: 'trackMuteB' }, { fn: 'swing' }, NONE, { fn: 'tempo' }],
  // The live rack's controller map (`fns`); this is the default map's.
  rack: rackFns(defaultControlMap()),
  pan: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partPan', part })).concat([0, 1, 2].map((part): Fn => ({ fn: 'fxReturn', part })), [{ fn: 'tempo' }]),
  // One page per effect: the parts' sends to it, its parameters, its return on knob 8.
  reverb: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partReverb', part })).concat([
    { fn: 'fxParam', param: 'reverbTime' }, { fn: 'fxParam', param: 'preDelay' }, { fn: 'fxParam', param: 'reverbTone' }, { fn: 'fxReturn', part: 0 },
  ]),
  chorus: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partChorus', part })).concat([
    { fn: 'fxParam', param: 'chorusRate' }, { fn: 'fxParam', param: 'chorusDepth' }, NONE, { fn: 'fxReturn', part: 1 },
  ]),
  delay: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partDelay', part })).concat([
    { fn: 'delayTime' }, { fn: 'fxParam', param: 'delayFeedback' }, { fn: 'fxParam', param: 'delayTone' }, { fn: 'fxReturn', part: 2 },
  ]),
}
const ORDER: KnobPage[] = ['style', 'rack', 'pan', 'reverb', 'chorus', 'delay']
const PAGE_NAME: Record<KnobPage, string> = { style: 'Style', rack: 'Rack', pan: 'Pan', reverb: 'Reverb', chorus: 'Chorus', delay: 'Delay' }
/** The effect pages' parameters (#236): full and short names, and how far a knob step moves each. */
const PARAM_KNOB: Partial<Record<FxParam, [string, string, number]>> = {
  reverbTime: ['Reverb Time', 'RevTime', 1],
  preDelay: ['Reverb Pre-delay', 'PreDly', 2],
  reverbTone: ['Reverb Tone', 'RevTone', 2],
  delayNote: ['Delay Note', 'DlyNote', 1],
  delayTime: ['Delay Time', 'DlyTime', 10],
  delayFeedback: ['Delay Feedback', 'DlyFdbk', 2],
  delayTone: ['Delay Tone', 'DlyTone', 2],
  chorusRate: ['Chorus Rate', 'ChoRate', 2],
  chorusDepth: ['Chorus Depth', 'ChoDepth', 1],
}
/** An effect parameter's state and its block. */
function fxParam(s: AppState, p: FxParam): [FxBlock, FxParamState] {
  for (const b of s.effects.blocks) {
    const x = b.params.find((q) => q.param === p)
    if (x) return [b.block, x]
  }
  throw new Error(`no ${p}`)
}
/** The delay time knob's parameter: the note value with tempo sync on, the ms with it off. */
function delayParam(s: AppState): FxParam {
  return fxParam(s, 'delaySync')[1].value ? 'delayNote' : 'delayTime'
}
const PART_FX: Partial<Record<KnobFunction, [string[], string[]]>> = {
  partPan: [['Right 1 Pan', 'Right 2 Pan', 'Right 3 Pan', 'Left Pan'], ['PanR1', 'PanR2', 'PanR3', 'PanL']],
  partReverb: [['Right 1 Reverb', 'Right 2 Reverb', 'Right 3 Reverb', 'Left Reverb'], ['RevR1', 'RevR2', 'RevR3', 'RevL']],
  partChorus: [['Right 1 Chorus', 'Right 2 Chorus', 'Right 3 Chorus', 'Left Chorus'], ['ChoR1', 'ChoR2', 'ChoR3', 'ChoL']],
  partDelay: [['Right 1 Delay', 'Right 2 Delay', 'Right 3 Delay', 'Left Delay'], ['DlyR1', 'DlyR2', 'DlyR3', 'DlyL']],
  // `part` is the effect block here: Reverb, Chorus, Variation (#204).
  fxReturn: [['Reverb Return', 'Chorus Return', 'Delay Return'], ['RevRtn', 'ChoRtn', 'DlyRtn']],
}
const FX_BLOCKS = ['reverb', 'chorus', 'variation'] as const
/** A pan as the Genos shows it: L63 … C … R63. */
export function panText(v: number): string {
  const d = Math.min(127, v) - 64
  return d === 0 ? 'C' : d < 0 ? `L${-d}` : `R${d}`
}
const PART_SHORT = ['Right1', 'Right2', 'Right3', 'Left']
const PART_NAME = ['Right 1 Volume', 'Right 2 Volume', 'Right 3 Volume', 'Left Volume']
const NAMES: Record<KnobFunction, [string, string]> = {
  none: ['No Assign', '---'],
  dynamics: ['Dynamics Control', 'DynCtrl'],
  retriggerRate: ['Retrigger Rate', 'RtgRate'],
  retriggerOnOff: ['Retrigger On/Off', 'RtgOnOff'],
  trackMuteA: ['Style Track Mute A', 'StyMuteA'],
  trackMuteB: ['Style Track Mute B', 'StyMuteB'],
  tempo: ['Tempo', 'Tempo'],
  swing: ['Swing', 'Swing'],
  partVolume: ['', ''],
  harmonyVolume: ['Harmony Volume', 'HarmVol'],
  metronomeVolume: ['Metronome Volume', 'MetroVol'],
  partPan: ['', ''],
  partReverb: ['', ''],
  partChorus: ['', ''],
  partDelay: ['', ''],
  fxReturn: ['', ''],
  fxParam: ['', ''],
  delayTime: ['Delay Time', 'DlyTime'],
  harmonyArp: ['Harmony/Arpeggio', 'HarmArp'],
  splitPoint: ['Split Point', 'Split'],
}
const RATES = [1, 2, 4, 8, 16, 32]
const RTG_STEPS = 3
const MUTE_STEP = 4
const LEVEL_STEP = 2
/** A part's and the Harmony's volume after a reset (the Genos default; `knobs::DEFAULT_VOLUME`). */
const DEFAULT_VOLUME = 100
const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v))

export class MockKnobs {
  page: KnobPage = 'style'
  private acc = [0, 0, 0, 0, 0, 0, 0, 0]
  /** The Track Mute A and B knob positions: they start fully right (every part on). */
  private mute = [127, 127]

  setPage(page: KnobPage) {
    if (page !== this.page) {
      this.page = page
      this.acc.fill(0)
    }
  }

  step(delta: number) {
    this.setPage(ORDER[clamp(ORDER.indexOf(this.page) + delta, 0, ORDER.length - 1)])
  }

  /** The page's knobs: the Rack page's follow the live rack's controller map. */
  private fns(s: AppState): Fn[] {
    return this.page === 'rack' ? rackFns(s.liveRack.controls) : PAGES[this.page]
  }

  /** Knob `knob` turned `delta` steps: the command it runs, or null. */
  turn(knob: number, delta: number, s: AppState): AppCmd | null {
    const f = this.fns(s)[knob] ?? NONE
    const level = (v: number) => clamp(v + delta * LEVEL_STEP, 0, 127)
    switch (f.fn) {
      case 'none':
        return null
      case 'dynamics':
        return { type: 'setDynamics', level: level(s.dynamics.level) }
      case 'swing': {
        const v = clamp(s.styleSettings.swing + delta * 2, 0, 100)
        return v === s.styleSettings.swing ? null : { type: 'setSwing', amount: v }
      }
      case 'retriggerRate': {
        const n = this.stepped(knob, delta)
        return n ? { type: 'stepRetriggerRate', delta: n } : null
      }
      case 'retriggerOnOff': {
        const n = this.stepped(knob, delta)
        return n && n > 0 !== s.transport.retrigger ? { type: 'toggleRetrigger' } : null
      }
      case 'trackMuteA':
      case 'trackMuteB': {
        const i = f.fn === 'trackMuteA' ? 0 : 1
        const v = clamp(this.mute[i] + delta * MUTE_STEP, 0, 127)
        if (v === this.mute[i]) return null
        this.mute[i] = v
        return { type: 'styleTrackMute', order: i === 0 ? 'a' : 'b', value: v }
      }
      case 'tempo':
        return { type: 'setTempo', bpm: clamp(Math.round(s.transport.tempo) + delta, 5, 500) }
      case 'partVolume':
        return { type: 'setPartVolume', part: f.part!, volume: level(s.keyboardParts[f.part!].volume) }
      case 'harmonyVolume':
        return { type: 'setHarmonyVolume', volume: level(s.harmonyArp.volume) }
      case 'metronomeVolume':
        return { type: 'setMetronomeVolume', volume: level(s.metronome.volume) }
      case 'partPan':
        return { type: 'setPartPan', part: f.part!, pan: level(s.keyboardParts[f.part!].pan) }
      case 'partReverb':
        return { type: 'setPartSend', part: f.part!, send: 'reverb', value: level(s.keyboardParts[f.part!].reverb) }
      case 'partChorus':
        return { type: 'setPartSend', part: f.part!, send: 'chorus', value: level(s.keyboardParts[f.part!].chorus) }
      case 'partDelay':
        return { type: 'setPartSend', part: f.part!, send: 'variation', value: level(s.keyboardParts[f.part!].variation) }
      case 'fxReturn':
        return { type: 'setEffectReturn', block: FX_BLOCKS[f.part!], level: level(s.effects.blocks[f.part!].returnLevel) }
      case 'fxParam':
      case 'delayTime': {
        const p = f.fn === 'delayTime' ? delayParam(s) : f.param!
        const [block, x] = fxParam(s, p)
        let to: number
        if (p === 'delayNote') {
          const n = this.stepped(knob, delta)
          if (!n) return null
          to = clamp(x.value + n, x.min, x.max)
        } else {
          to = clamp(x.value + delta * PARAM_KNOB[p]![2], x.min, x.max)
        }
        return to === x.value ? null : { type: 'setEffectParam', block, param: p, value: to }
      }
      case 'harmonyArp': {
        const n = this.stepped(knob, delta)
        return n && n > 0 !== s.harmonyArp.on ? { type: 'toggleHarmonyArp' } : null
      }
      case 'splitPoint': {
        const to = clamp(s.chord.split + delta, SPLIT_MIN, SPLIT_MAX)
        return to === s.chord.split ? null : { type: 'setSplit', note: to }
      }
    }
  }

  /** Knob `knob` put back to its default (a double-click): the command, or null (`src/knobs.rs` reset). */
  reset(knob: number, s: AppState): AppCmd | null {
    const f = this.fns(s)[knob] ?? NONE
    switch (f.fn) {
      case 'none':
        return null
      case 'dynamics':
        return s.dynamics.level === 127 ? null : { type: 'setDynamics', level: 127 }
      case 'retriggerRate':
        this.acc[knob] = 0
        return s.styleSettings.retriggerRate === 8 ? null : { type: 'setRetriggerRate', rate: 8 }
      case 'retriggerOnOff':
        this.acc[knob] = 0
        return s.transport.retrigger ? { type: 'toggleRetrigger' } : null
      case 'trackMuteA':
      case 'trackMuteB': {
        const i = f.fn === 'trackMuteA' ? 0 : 1
        if (this.mute[i] === 127) return null
        this.mute[i] = 127
        return { type: 'styleTrackMute', order: i === 0 ? 'a' : 'b', value: 127 }
      }
      case 'tempo':
        return { type: 'resetTempo' }
      case 'swing':
        return s.styleSettings.swing === 0 ? null : { type: 'setSwing', amount: 0 }
      case 'partVolume':
        return { type: 'setPartVolume', part: f.part!, volume: DEFAULT_VOLUME }
      case 'harmonyVolume':
        return { type: 'setHarmonyVolume', volume: DEFAULT_VOLUME }
      case 'metronomeVolume':
        return { type: 'setMetronomeVolume', volume: 90 }
      case 'partPan':
        return { type: 'setPartPan', part: f.part!, pan: 64 }
      case 'partReverb':
        return { type: 'setPartSend', part: f.part!, send: 'reverb', value: 0 }
      case 'partChorus':
        return { type: 'setPartSend', part: f.part!, send: 'chorus', value: 0 }
      case 'partDelay':
        return { type: 'setPartSend', part: f.part!, send: 'variation', value: 0 }
      case 'fxReturn':
        return { type: 'setEffectReturn', block: FX_BLOCKS[f.part!], level: 64 }
      case 'fxParam':
      case 'delayTime': {
        if (f.fn === 'delayTime') this.acc[knob] = 0
        const p = f.fn === 'delayTime' ? delayParam(s) : f.param!
        const [block, x] = fxParam(s, p)
        return { type: 'setEffectParam', block, param: p, value: x.default }
      }
      case 'harmonyArp':
        this.acc[knob] = 0
        return s.harmonyArp.on ? { type: 'toggleHarmonyArp' } : null
      case 'splitPoint':
        return s.chord.split === DEFAULT_SPLIT ? null : { type: 'setSplit', note: DEFAULT_SPLIT }
    }
  }

  private stepped(knob: number, d: number): number {
    let a = this.acc[knob]
    if ((a > 0 && d < 0) || (a < 0 && d > 0)) a = 0
    a += d
    const n = Math.trunc(a / RTG_STEPS)
    this.acc[knob] = a - n * RTG_STEPS
    return clamp(n, -6, 6)
  }

  state(s: AppState): KnobsState {
    const knobs = this.fns(s).map((f) => this.read(f, s))
    return { page: this.page, pageName: PAGE_NAME[this.page], pageNumber: ORDER.indexOf(this.page) + 1, pageCount: ORDER.length, knobs }
  }

  /** Function `f` as it reads now (on a knob, or on a fader the controller map gives it). */
  read(f: Fn, s: AppState): KnobState {
      const fx = PART_FX[f.fn]
      const [name, short] =
        f.fn === 'partVolume' ? [PART_NAME[f.part!], PART_SHORT[f.part!]]
        : fx ? [fx[0][f.part!], fx[1][f.part!]]
        : f.fn === 'fxParam' ? PARAM_KNOB[f.param!]!.slice(0, 2) as [string, string]
        : NAMES[f.fn]
      const r = (value: string, level: number | null) => ({ function: f.fn, name, short, value, level })
      switch (f.fn) {
        case 'none': return r('', null)
        case 'dynamics': return r(String(s.dynamics.level), s.dynamics.level)
        case 'retriggerRate': {
          const rate = s.styleSettings.retriggerRate
          return r(`1/${rate}`, Math.floor((Math.max(0, RATES.indexOf(rate)) * 127) / (RATES.length - 1)))
        }
        case 'retriggerOnOff': return r(s.transport.retrigger ? 'On' : 'Off', s.transport.retrigger ? 127 : 0)
        case 'trackMuteA':
        case 'trackMuteB': {
          const v = this.mute[f.fn === 'trackMuteA' ? 0 : 1]
          const n = 1 + Math.floor((v * 7 + 63) / 127)
          return r(n === 8 ? 'All' : `${n} of 8`, v)
        }
        case 'tempo': return r(`${Math.round(s.transport.tempo)} BPM`, null)
        case 'swing': return r(`${s.styleSettings.swing}%`, Math.floor((s.styleSettings.swing * 127) / 100))
        case 'partVolume': return r(String(s.keyboardParts[f.part!].volume), s.keyboardParts[f.part!].volume)
        case 'harmonyVolume': return r(String(s.harmonyArp.volume), s.harmonyArp.volume)
        case 'metronomeVolume': return r(String(s.metronome.volume), s.metronome.volume)
        case 'partPan': return r(panText(s.keyboardParts[f.part!].pan), s.keyboardParts[f.part!].pan)
        case 'partReverb': return r(String(s.keyboardParts[f.part!].reverb), s.keyboardParts[f.part!].reverb)
        case 'partChorus': return r(String(s.keyboardParts[f.part!].chorus), s.keyboardParts[f.part!].chorus)
        case 'partDelay': return r(String(s.keyboardParts[f.part!].variation), s.keyboardParts[f.part!].variation)
        case 'fxReturn': return r(String(s.effects.blocks[f.part!].returnLevel), s.effects.blocks[f.part!].returnLevel)
        case 'fxParam':
        case 'delayTime': {
          const x = fxParam(s, f.fn === 'delayTime' ? delayParam(s) : f.param!)[1]
          return r(x.display, Math.floor(((x.value - x.min) * 127) / Math.max(1, x.max - x.min)))
        }
        case 'harmonyArp': return r(s.harmonyArp.on ? 'On' : 'Off', s.harmonyArp.on ? 127 : 0)
        case 'splitPoint': return r(noteName(s.chord.split), splitLevel(s.chord.split))
      }
  }
}
