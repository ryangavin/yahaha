// The assignable functions a pedal can run (docs/controllers.md). The table is the engine's
// own (`controllers::FUNCTIONS` in src/controllers.rs), written to assignable-functions.json
// by a Rust test that keeps the two equal; here it only gets its types and helpers.

import table from './assignable-functions.json'
import type { AppCmd, AssignableFunction, ControllersState, ControlType, Fingering, FunctionId, PedalState } from './types'

export const FUNCTIONS = table as AssignableFunction[]

export const CATEGORY_NAMES: Record<AssignableFunction['category'], string> = {
  voice: 'Voice',
  style: 'Style',
  ots: 'One Touch Setting',
  registration: 'Registration',
  overall: 'Overall',
}

/** The table grouped for a picker, in the table's order. */
export function functionGroups(): { category: AssignableFunction['category']; name: string; functions: AssignableFunction[] }[] {
  const out: ReturnType<typeof functionGroups> = []
  for (const f of FUNCTIONS) {
    let g = out.find((x) => x.category === f.category)
    if (!g) out.push((g = { category: f.category, name: CATEGORY_NAMES[f.category], functions: [] }))
    g.functions.push(f)
  }
  return out
}

export function functionInfo(id: FunctionId): AssignableFunction | undefined {
  return FUNCTIONS.find((f) => f.id === id)
}

export function functionName(id: FunctionId): string {
  return functionInfo(id)?.name ?? id
}

/** Why a pedal can't listen for `cc` (null: it can), as `controllers::pedal_cc_refused`. */
export function pedalCcRefused(cc: number): string | null {
  if (cc === 0 || cc === 32) return 'bank select'
  if (cc === 1) return 'the modulation wheel'
  if (cc === 6 || cc === 38) return 'data entry'
  if (cc === 7) return 'volume'
  if (cc >= 98 && cc <= 101) return '(N)RPN selection'
  if (cc === 121) return 'Reset All Controllers'
  if (cc >= 120 && cc <= 127) return 'a channel mode message'
  if (cc > 127 || cc < 0) return 'not a control change'
  return null
}

/** The pedals as the engine starts: the GM sustain, sostenuto and soft pedals. */
export function defaultControllers(): ControllersState {
  const pedal = (cc: number, fn: FunctionId) => ({ cc, function: fn, controlType: 'holdA' as const, reverse: false, range: 'upper' as const, down: false })
  return {
    pedals: [pedal(64, 'sustain'), pedal(66, 'sostenuto'), pedal(67, 'soft')],
    learning: null,
    parts: [0, 1, 2, 3].map((p) => ({ sustain: true, pitchBend: true, modulation: p < 3, bendRange: 2 })),
    sustain: false,
    sostenuto: false,
    soft: false,
  }
}

/** The command a function stands for, as the engine runs it (for the mock), or null for
 * the pedal switches and continuous functions. */
export function functionCmd(id: FunctionId, st: { fingering: Fingering }): AppCmd | null {
  const n = (prefix: string) => Number(id.slice(prefix.length))
  const letter = (prefix: string) => 'ABCD'.indexOf(id.slice(prefix.length))
  if (/^intro[123]$/.test(id)) return { type: 'intro', index: n('intro') - 1 }
  if (/^main[ABCD]$/.test(id)) return { type: 'main', index: letter('main') }
  if (/^ending[123]$/.test(id)) return { type: 'ending', index: n('ending') - 1 }
  if (/^ots[1234]$/.test(id)) return { type: 'recallOts', index: n('ots') - 1 }
  const parts: Record<string, number> = { right1OnOff: 0, right2OnOff: 1, right3OnOff: 2, leftOnOff: 3 }
  if (id in parts) return { type: 'togglePart', part: parts[id] }
  const simple: Record<string, AppCmd> = {
    startStop: { type: 'startStop' },
    syncStart: { type: 'toggleSyncStart' },
    syncStop: { type: 'toggleSyncStop' },
    fillDown: { type: 'fill', delta: -1 },
    fillSelf: { type: 'fill', delta: 0 },
    fillUp: { type: 'fill', delta: 1 },
    fillBreak: { type: 'break' },
    autoFill: { type: 'toggleAutoFill' },
    stopAcmp: { type: 'toggleStopAcmp' },
    otsLink: { type: 'toggleOtsLink' },
    tempoUp: { type: 'tempoUp' },
    tempoDown: { type: 'tempoDown' },
    tapTempo: { type: 'tapTempo' },
    fadeInOut: { type: 'toggleFade' },
    sectionReset: { type: 'sectionReset' },
    transposeUp: { type: 'stepTranspose', keyboard: 0, master: 1 },
    transposeDown: { type: 'stepTranspose', keyboard: 0, master: -1 },
    registBankNext: { type: 'stepRegistBank', delta: 1 },
    registBankPrev: { type: 'stepRegistBank', delta: -1 },
    fingeredOnBass: { type: 'setFingering', fingering: st.fingering === 'fingeredOnBass' ? 'fingered' : 'fingeredOnBass' },
    // The control-side switches: a press (a Toggle pedal, Try) switches them.
    kbdHarmonyArp: { type: 'toggleHarmonyArp' },
    arpHold: { type: 'toggleArpPedalHold' },
  }
  return simple[id] ?? null
}

/** The pedal switches the engine keeps (a press toggles the bit). */
export function isPedalSwitch(id: FunctionId): id is 'sustain' | 'sostenuto' | 'soft' {
  return id === 'sustain' || id === 'sostenuto' || id === 'soft'
}

/** The command that sets control-side switch `id` (Kbd Harmony/Arpeggio, Arpeggio Hold) on
 * or off, as a Hold A / Hold B pedal does (`api::function_set`); null for other functions. */
export function functionSet(id: FunctionId, on: boolean): AppCmd | null {
  if (id === 'kbdHarmonyArp') return { type: 'setHarmonyArpOn', on }
  if (id === 'arpHold') return { type: 'setArpPedalHold', on }
  return null
}

type Setup = Pick<PedalState, 'cc' | 'function' | 'controlType'>
const held = (p: Setup) => functionSet(p.function, false) !== null && p.controlType !== 'toggle'
const holdOn = (ct: ControlType, down: boolean) => (ct === 'holdB') !== down

/** What a pedal's new setup does to the control-side switches, as
 * `controllers::control_switch_sets`: the switch a Hold pedal kept on goes off when it is
 * given another function or CC, and a Hold switch follows where the pedal is now. */
export function controlSwitchSets(old: Setup, next: Setup, oldDown: boolean, newDown: boolean): [FunctionId, boolean][] {
  const rebound = old.function !== next.function || old.cc !== next.cc
  const retyped = rebound || old.controlType !== next.controlType
  const out: [FunctionId, boolean][] = []
  if (rebound && held(old) && holdOn(old.controlType, oldDown)) out.push([old.function, false])
  if (retyped && held(next)) out.push([next.function, holdOn(next.controlType, newDown)])
  return out
}

/** The control-side switch a reset (PANIC, a keyboard unplugged) turns off for pedal `p`
 * (`wasDown`: down at the reset), as `controllers::reset_release`: the one a Hold pedal
 * was keeping on. A reset never turns anything on. */
export function resetRelease(p: Setup, wasDown: boolean): FunctionId | null {
  const heldOn = p.controlType === 'holdA' ? wasDown : p.controlType === 'holdB' ? !wasDown : false
  return functionSet(p.function, false) !== null && heldOn ? p.function : null
}
