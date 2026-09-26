// The mock's Knob Assign pages (#197): a port of `src/knobs.rs` (the same pages, names,
// steps and readings). A turn gives back the command it runs, which the mock then runs as
// the session does. Only the mock uses this; with the real engine the knobs come in the state.

import type { AppCmd, AppState, FxBlock, FxParam, FxParamState, KnobFunction, KnobPage, KnobState, KnobsState } from './types'

type Fn = { fn: KnobFunction; part?: number; param?: FxParam }
const NONE: Fn = { fn: 'none' }
const PAGES: Record<KnobPage, Fn[]> = {
  style: [{ fn: 'dynamics' }, { fn: 'retriggerRate' }, { fn: 'retriggerOnOff' }, { fn: 'trackMuteA' }, { fn: 'trackMuteB' }, NONE, NONE, { fn: 'tempo' }],
  parts: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partVolume', part })).concat([{ fn: 'harmonyVolume' }, { fn: 'metronomeVolume' }, NONE, { fn: 'tempo' }]),
  pan: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partPan', part })).concat([0, 1, 2].map((part): Fn => ({ fn: 'fxReturn', part })), [{ fn: 'tempo' }]),
  effects: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partReverb', part })).concat([0, 1, 2, 3].map((part): Fn => ({ fn: 'partChorus', part }))),
  fx: [
    { fn: 'fxParam', param: 'reverbTime' }, { fn: 'fxParam', param: 'preDelay' }, { fn: 'fxParam', param: 'reverbTone' }, { fn: 'delayTime' },
    { fn: 'fxParam', param: 'delayFeedback' }, { fn: 'fxParam', param: 'chorusRate' }, { fn: 'fxParam', param: 'chorusDepth' }, { fn: 'tempo' },
  ],
}
const ORDER: KnobPage[] = ['style', 'parts', 'pan', 'effects', 'fx']
const PAGE_NAME: Record<KnobPage, string> = { style: 'Style', parts: 'Parts', pan: 'Pan', effects: 'Effects', fx: 'FX' }
/** The FX page's parameters (#236): full and short names, and how far a knob step moves each. */
const PARAM_KNOB: Partial<Record<FxParam, [string, string, number]>> = {
  reverbTime: ['Reverb Time', 'RevTime', 1],
  preDelay: ['Reverb Pre-delay', 'PreDly', 2],
  reverbTone: ['Reverb Tone', 'RevTone', 2],
  delayNote: ['Delay Note', 'DlyNote', 1],
  delayTime: ['Delay Time', 'DlyTime', 10],
  delayFeedback: ['Delay Feedback', 'DlyFdbk', 2],
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
  partVolume: ['', ''],
  harmonyVolume: ['Harmony Volume', 'HarmVol'],
  metronomeVolume: ['Metronome Volume', 'MetroVol'],
  partPan: ['', ''],
  partReverb: ['', ''],
  partChorus: ['', ''],
  fxReturn: ['', ''],
  fxParam: ['', ''],
  delayTime: ['Delay Time', 'DlyTime'],
}
const RATES = [1, 2, 4, 8, 16, 32]
const RTG_STEPS = 3
const MUTE_STEP = 4
const LEVEL_STEP = 2
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

  /** Knob `knob` turned `delta` steps: the command it runs, or null. */
  turn(knob: number, delta: number, s: AppState): AppCmd | null {
    const f = PAGES[this.page][knob] ?? NONE
    const level = (v: number) => clamp(v + delta * LEVEL_STEP, 0, 127)
    switch (f.fn) {
      case 'none':
        return null
      case 'dynamics':
        return { type: 'setDynamics', level: level(s.dynamics.level) }
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
    const knobs = PAGES[this.page].map((f): KnobState => {
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
        case 'partVolume': return r(String(s.keyboardParts[f.part!].volume), s.keyboardParts[f.part!].volume)
        case 'harmonyVolume': return r(String(s.harmonyArp.volume), s.harmonyArp.volume)
        case 'metronomeVolume': return r(String(s.metronome.volume), s.metronome.volume)
        case 'partPan': return r(panText(s.keyboardParts[f.part!].pan), s.keyboardParts[f.part!].pan)
        case 'partReverb': return r(String(s.keyboardParts[f.part!].reverb), s.keyboardParts[f.part!].reverb)
        case 'partChorus': return r(String(s.keyboardParts[f.part!].chorus), s.keyboardParts[f.part!].chorus)
        case 'fxReturn': return r(String(s.effects.blocks[f.part!].returnLevel), s.effects.blocks[f.part!].returnLevel)
        case 'fxParam':
        case 'delayTime': {
          const x = fxParam(s, f.fn === 'delayTime' ? delayParam(s) : f.param!)[1]
          return r(x.display, Math.floor(((x.value - x.min) * 127) / Math.max(1, x.max - x.min)))
        }
      }
    })
    return { page: this.page, pageName: PAGE_NAME[this.page], pageNumber: ORDER.indexOf(this.page) + 1, pageCount: ORDER.length, knobs }
  }
}
