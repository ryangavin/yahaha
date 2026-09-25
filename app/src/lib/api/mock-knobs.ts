// The mock's Knob Assign pages (#197): a port of `src/knobs.rs` (the same pages, names,
// steps and readings). A turn gives back the command it runs, which the mock then runs as
// the session does. Only the mock uses this; with the real engine the knobs come in the state.

import type { AppCmd, AppState, KnobFunction, KnobPage, KnobState, KnobsState } from './types'

type Fn = { fn: KnobFunction; part?: number }
const NONE: Fn = { fn: 'none' }
const PAGES: Record<KnobPage, Fn[]> = {
  style: [{ fn: 'dynamics' }, { fn: 'retriggerRate' }, { fn: 'retriggerOnOff' }, { fn: 'trackMuteA' }, { fn: 'trackMuteB' }, NONE, NONE, { fn: 'tempo' }],
  parts: [0, 1, 2, 3].map((part): Fn => ({ fn: 'partVolume', part })).concat([{ fn: 'harmonyVolume' }, { fn: 'metronomeVolume' }, NONE, { fn: 'tempo' }]),
}
const ORDER: KnobPage[] = ['style', 'parts']
const PAGE_NAME: Record<KnobPage, string> = { style: 'Style', parts: 'Parts' }
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
      const [name, short] = f.fn === 'partVolume' ? [PART_NAME[f.part!], PART_SHORT[f.part!]] : NAMES[f.fn]
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
      }
    })
    return { page: this.page, pageName: PAGE_NAME[this.page], pageNumber: ORDER.indexOf(this.page) + 1, pageCount: ORDER.length, knobs }
  }
}
