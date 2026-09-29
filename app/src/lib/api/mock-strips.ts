// Channel strips and send effects in the mock (the mixer rework), as the Rust `api::Strips`
// model (src/api/strips.rs) keeps them: state only, nothing plays. What an older command
// covers goes through it (`stripLegacy`, the Rust `StripCmd::legacy`), and `fill` puts
// every part's `strip` and `effects.sends` into the state, taking what the older state has
// (the Rust `Strips::fill`). Also the effect bus's parameter table (#236), which the send
// effects share with the buses.

import type {
  AppCmd, AppState, CompPreset, FxBlock, FxParam, FxParamState, FxType, InsertEffect, InsertSlotState, InsertType, PartCompParam, PartCompState, PartEq, PartInsert, PartSend,
  SendKind, SendState, SettingState, StripCmd, StripState,
} from './types'
import { clampEq, FLAT_EQ, OFF_INSERT } from './types'

/** Channel strips: the 4 keyboard parts', then the 8 Style parts'. */
export const STRIPS = 12
export const KEYBOARD_STRIPS = 4
/** Send effects: 0-2 the style's buses, 3-5 added by the player. */
export const SENDS = 6
export const STYLE_SENDS = 3
const INSERT_SLOTS = 2
const INSERT_VALUES = 4
const SEND_PARAMS = 6
const RETURN_UNITY = 64
const NOTES = ['1/16', '1/8T', '1/8', '1/4T', '1/8.', '1/4', '1/4.', '1/2']

/** Each effect parameter's block, name, range and reading, as the session's (#236). */
export const FX_PARAMS: Record<FxParam, { block: FxBlock; name: string; min: number; max: number; display: (v: number) => string }> = {
  reverbTime: { block: 'reverb', name: 'Time', min: 3, max: 100, display: (v) => `${(v / 10).toFixed(1)} s` },
  preDelay: { block: 'reverb', name: 'Pre-delay', min: 0, max: 200, display: (v) => `${v} ms` },
  reverbTone: { block: 'reverb', name: 'Tone', min: 10, max: 200, display: (v) => `${(v / 10).toFixed(1)} kHz` },
  delaySync: { block: 'variation', name: 'Tempo sync', min: 0, max: 1, display: (v) => (v ? 'On' : 'Off') },
  delayNote: { block: 'variation', name: 'Note', min: 0, max: 7, display: (v) => NOTES[v] },
  delayTime: { block: 'variation', name: 'Time', min: 10, max: 2000, display: (v) => `${v} ms` },
  delayFeedback: { block: 'variation', name: 'Feedback', min: 0, max: 90, display: (v) => `${v}%` },
  delayTone: { block: 'variation', name: 'Tone', min: 10, max: 200, display: (v) => `${(v / 10).toFixed(1)} kHz` },
  pingPong: { block: 'variation', name: 'Ping-pong', min: 0, max: 1, display: (v) => (v ? 'On' : 'Off') },
  chorusRate: { block: 'chorus', name: 'Rate', min: 5, max: 500, display: (v) => `${(v / 100).toFixed(2)} Hz` },
  chorusDepth: { block: 'chorus', name: 'Depth', min: 0, max: 50, display: (v) => `${(v / 10).toFixed(1)} ms` },
}
const DELAY = { delaySync: 1, delayTime: 375, delayFeedback: 38, delayTone: 50 }

/** Each type's own parameter values (the session's `type_defaults`). */
const FX_TYPE_PARAMS: Record<FxType, Partial<Record<FxParam, number>>> = {
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

const BLOCKS: FxBlock[] = ['reverb', 'chorus', 'variation']
const blockParams = (block: FxBlock) => (Object.keys(FX_PARAMS) as FxParam[]).filter((p) => FX_PARAMS[p].block === block)

/** A block's parameters at type `effect`'s own values. */
export function fxParams(block: FxBlock, effect: FxType): FxParamState[] {
  const own = FX_TYPE_PARAMS[effect] ?? {}
  return blockParams(block).map((param) => {
    const { name, min, max, display } = FX_PARAMS[param]
    const value = own[param] ?? min
    return { param, name, value, min, max, default: value, display: display(value) }
  })
}

/** A setting's range, default and reading (the Rust `KnobSpec`). */
interface KnobSpec {
  name: string
  min: number
  max: number
  default: number
  display: (v: number) => string
}

const knob = (name: string, min: number, max: number, def: number, display: (v: number) => string = (v) => `${v}`): KnobSpec => ({ name, min, max, default: def, display })
const ms = (v: number) => `${v} ms`
const EIGHTH = 2

const PHASER: KnobSpec[] = [knob('Depth', 0, 127, 64), knob('Rate', 5, 500, 50, (v) => `${(v / 100).toFixed(2)} Hz`), knob('Feedback', 0, 90, 40, (v) => `${v}%`)]

/** Each insert kind's name and settings (the Rust `InsertType::name`, `settings`). */
const INSERT_KINDS: Record<string, { name: string; settings: KnobSpec[] }> = {
  none: { name: 'None', settings: [] },
  distortion: { name: 'Distortion', settings: [knob('Drive', 0, 127, 64), knob('Tone', 0, 127, 64), knob('Output', 0, 127, 100)] },
  compressor: { name: 'Compressor', settings: [knob('Squeeze', 0, 127, 64), knob('Attack', 1, 80, 10, ms), knob('Release', 10, 1000, 200, ms), knob('Output', 0, 127, 100)] },
  autoWah: { name: 'Auto Wah', settings: [knob('Sensitivity', 0, 127, 64), knob('Resonance', 0, 127, 64), knob('Frequency', 0, 127, 32)] },
  tremolo: { name: 'Tremolo', settings: [knob('Depth', 0, 127, 64), knob('Note', 0, 7, EIGHTH, (v) => NOTES[v] ?? `${v}`), knob('Shape', 0, 127, 0)] },
  rotary: { name: 'Rotary', settings: [knob('Depth', 0, 127, 64), knob('Drive', 0, 127, 0), knob('Balance', 0, 127, 64)] },
  phaser: { name: 'Phaser', settings: PHASER },
}

/** Each send kind's name and the bus that plays it (the Rust `SendKind::name`, `bus`). */
const SEND_KINDS: Record<string, { name: string; bus: [number, FxType] | null }> = {
  hall: { name: 'Hall', bus: [0, 'hall'] },
  room: { name: 'Room', bus: [0, 'room'] },
  stage: { name: 'Stage', bus: [0, 'stage'] },
  plate: { name: 'Plate', bus: [0, 'plate'] },
  chorus: { name: 'Chorus', bus: [1, 'chorus'] },
  celeste: { name: 'Celeste', bus: [1, 'celeste'] },
  flanger: { name: 'Flanger', bus: [1, 'flanger'] },
  eighth: { name: 'Delay 1/8', bus: [2, 'eighth'] },
  dottedEighth: { name: 'Delay 1/8.', bus: [2, 'dottedEighth'] },
  quarter: { name: 'Delay 1/4', bus: [2, 'quarter'] },
  pingPong: { name: 'Ping-Pong', bus: [2, 'pingPong'] },
  phaser: { name: 'Phaser', bus: null },
}

const own = <T>(table: Record<string, T>, k: string): T | undefined => (Object.prototype.hasOwnProperty.call(table, k) ? table[k] : undefined)
const insertSettings = (k: InsertType) => own(INSERT_KINDS, k)?.settings ?? []
const insertName = (k: InsertType) => own(INSERT_KINDS, k)?.name ?? k
const sendBus = (k: SendKind) => own(SEND_KINDS, k)?.bus ?? null
const sendName = (k: SendKind) => own(SEND_KINDS, k)?.name ?? k

/** A send kind's parameters (`SendKind::params`): a bus kind's are its bus's at the type's
 * own values, a phaser's its own; none for an unknown kind. */
function sendParams(k: SendKind): KnobSpec[] {
  const bus = sendBus(k)
  if (bus) {
    const [block, type] = bus
    return blockParams(BLOCKS[block]).map((p) => {
      const s = FX_PARAMS[p]
      return knob(s.name, s.min, s.max, FX_TYPE_PARAMS[type][p] ?? s.min, s.display)
    })
  }
  return k === 'phaser' ? PHASER : []
}

/** `specs`' defaults, padded with 0 to `n`. */
const defaults = (specs: KnobSpec[], n: number) => Array.from({ length: n }, (_, i) => specs[i]?.default ?? 0)
const clampTo = (s: KnobSpec, v: number) => Math.max(s.min, Math.min(s.max, Math.round(v)))
const setting = (s: KnobSpec, v: number): SettingState => {
  const value = clampTo(s, v)
  return { name: s.name, value, min: s.min, max: s.max, default: s.default, display: s.display(value) }
}

interface InsertSlot {
  kind: InsertType
  on: boolean
  values: number[]
}

interface SendSlot {
  kind: SendKind
  params: number[]
  returnLevel: number
}

interface PartComp {
  on: boolean
  preset: CompPreset
  threshold: number
  ratio: number
  attack: number
  release: number
  makeup: number
}

interface StripSettings {
  eq: PartEq
  comp: PartComp
  inserts: InsertSlot[]
  sends: number[]
}

/** Each compressor type's threshold, ratio, attack, release and make-up (`preset_params`). */
const COMP_PARAMS: Record<CompPreset, [number, number, number, number, number]> = {
  natural: [-18, 25, 10, 200, 3],
  rich: [-20, 20, 30, 400, 3],
  punchy: [-24, 60, 5, 120, 6],
  electronic: [-22, 40, 3, 100, 5],
  loud: [-30, 80, 2, 150, 9],
}
/** Each compressor parameter's range (part_comp.rs). */
export const COMP_RANGES: Record<PartCompParam, [number, number]> = {
  threshold: [-48, 0],
  ratio: [10, 200],
  attack: [1, 100],
  release: [10, 1000],
  makeup: [0, 24],
}
const COMP_ORDER: PartCompParam[] = ['threshold', 'ratio', 'attack', 'release', 'makeup']

function compOf(on: boolean, preset: CompPreset): PartComp {
  const [threshold, ratio, attack, release, makeup] = COMP_PARAMS[preset]
  return { on, preset, threshold, ratio, attack, release, makeup }
}

function compState(c: PartComp): PartCompState {
  const own = COMP_PARAMS[c.preset]
  return { ...c, edited: COMP_ORDER.some((p, i) => c[p] !== own[i]) }
}

const emptySlot = (): InsertSlot => ({ kind: 'none', on: false, values: [0, 0, 0, 0] })
const sendSlot = (kind: SendKind): SendSlot => ({ kind, params: defaults(sendParams(kind), SEND_PARAMS), returnLevel: RETURN_UNITY })
const newStrip = (): StripSettings => ({ eq: { ...FLAT_EQ }, comp: compOf(false, 'natural'), inserts: [emptySlot(), emptySlot()], sends: [0, 0, 0, 0, 0, 0] })

/** A new kind: its values back to that kind's defaults; on/off unchanged. */
function setInsertKind(s: InsertSlot, kind: InsertType) {
  s.values = defaults(insertSettings(kind), INSERT_VALUES)
  s.kind = kind
}

function setSendKind(s: SendSlot, kind: SendKind) {
  s.params = defaults(sendParams(kind), SEND_PARAMS)
  s.kind = kind
}

/** The older single-slot shape as a slot (`InsertSlot::from_part_insert`): an off slot at its defaults is empty. */
function fromPartInsert(p: PartInsert): InsertSlot {
  if (p.effect === OFF_INSERT.effect && p.on === OFF_INSERT.on && p.amount === OFF_INSERT.amount) return emptySlot()
  const values = defaults(insertSettings(p.effect), INSERT_VALUES)
  values[0] = p.amount
  return { kind: p.effect, on: p.on, values }
}

function slotState(s: InsertSlot): InsertSlotState {
  return { kind: s.kind, name: insertName(s.kind), on: s.on, settings: insertSettings(s.kind).map((k, i) => setting(k, s.values[i])) }
}

function sendState(send: number, s: SendSlot, fromStyle: boolean, setByRack: boolean): SendState {
  return { send, kind: s.kind, name: sendName(s.kind), params: sendParams(s.kind).map((k, i) => setting(k, s.params[i])), returnLevel: s.returnLevel, fromStyle, setByRack }
}

const STRIP_TYPES = new Set<string>([
  'setStripEq', 'setStripCompressorOn', 'setStripCompressorPreset', 'setStripCompressorParam', 'setStripInsertKind', 'setStripInsertOn', 'setStripInsertSetting',
  'setStripSend', 'addSend', 'removeSend', 'setSendKind', 'setSendParam', 'setSendReturn', 'setRackSendOverride',
])

/** A strip or send command (`AppCmd::Strips`). */
export function isStripCmd(c: AppCmd): c is StripCmd {
  return STRIP_TYPES.has(c.type)
}

const PART_SENDS: PartSend[] = ['reverb', 'chorus', 'variation']

/** The older command a strip command is, when one covers it (`StripCmd::legacy`): the mock
 * sends that one too, and still keeps the strip command for what the older one doesn't carry. */
export function stripLegacy(c: StripCmd): AppCmd[] {
  // Emptying a keyboard strip's insert 1: the slot goes back to OFF_INSERT, which `fill`
  // shows as none (turning it off alone would keep the old kind).
  if (c.type === 'setStripInsertKind' && c.slot === 0 && c.kind === 'none' && Number.isInteger(c.strip) && c.strip >= 0 && c.strip < KEYBOARD_STRIPS) {
    const part = c.strip
    return [
      { type: 'setKeyboardInsertEffect', part, effect: OFF_INSERT.effect },
      { type: 'setKeyboardInsertOn', part, on: OFF_INSERT.on },
      { type: 'setKeyboardInsertAmount', part, amount: OFF_INSERT.amount },
    ]
  }
  const old = legacyOne(c)
  return old ? [old] : []
}

function legacyOne(c: StripCmd): AppCmd | null {
  const kb = (strip: number) => (Number.isInteger(strip) && strip >= 0 && strip < KEYBOARD_STRIPS ? strip : null)
  const style = (strip: number) => (Number.isInteger(strip) && strip >= KEYBOARD_STRIPS && strip < KEYBOARD_STRIPS + 8 ? strip - KEYBOARD_STRIPS : null)
  const styleSend = (send: number) => Number.isInteger(send) && send >= 0 && send < STYLE_SENDS
  switch (c.type) {
    case 'setStripEq': {
      const part = kb(c.strip)
      return part === null ? null : { type: 'setPartEq', part, eq: c.eq }
    }
    case 'setStripSend': {
      const send = styleSend(c.send) ? PART_SENDS[c.send] : null
      if (send === null) return null
      const part = kb(c.strip)
      if (part !== null) return { type: 'setPartSend', part, send, value: c.level }
      const sp = style(c.strip)
      return sp === null ? null : { type: 'setStylePartSend', part: sp, send, value: c.level }
    }
    case 'setStripInsertKind': {
      const part = kb(c.strip)
      if (c.slot !== 0 || part === null) return null
      return own(INSERT_KINDS, c.kind) ? { type: 'setKeyboardInsertEffect', part, effect: c.kind as InsertEffect } : null
    }
    case 'setStripInsertOn': {
      if (c.slot !== 0) return null
      const part = kb(c.strip)
      if (part !== null) return { type: 'setKeyboardInsertOn', part, on: c.on }
      const sp = style(c.strip)
      return sp === null ? null : { type: 'setPartInsertOn', part: sp, on: c.on }
    }
    case 'setStripInsertSetting': {
      if (c.slot !== 0 || c.setting !== 0) return null
      const amount = Math.min(127, c.value)
      const part = kb(c.strip)
      if (part !== null) return { type: 'setKeyboardInsertAmount', part, amount }
      const sp = style(c.strip)
      return sp === null ? null : { type: 'setPartInsertAmount', part: sp, amount }
    }
    case 'setSendKind': {
      if (!styleSend(c.send)) return null
      const bus = sendBus(c.kind)
      return bus && bus[0] === c.send ? { type: 'setEffectType', block: BLOCKS[c.send], effect: bus[1] } : null
    }
    case 'setSendParam': {
      if (!styleSend(c.send)) return null
      const block = BLOCKS[c.send]
      const param = blockParams(block)[c.param]
      return param === undefined ? null : { type: 'setEffectParam', block, param, value: c.value }
    }
    case 'setSendReturn':
      return styleSend(c.send) ? { type: 'setEffectReturn', block: BLOCKS[c.send], level: c.level } : null
    default:
      return null
  }
}

/** The strips and the added send effects (the Rust `api::Strips`). */
export class MockStrips {
  strips: StripSettings[] = Array.from({ length: STRIPS }, newStrip)
  /** Sends 3-5, as added. */
  added: SendSlot[] = []
  /** Sends 0-2: the rack overrides the style's kind. */
  overrides = [false, false, false]
  /** Sends 0-2 as last shown (`fill` brings them from the buses). */
  styleSends: SendSlot[] = [sendSlot('hall'), sendSlot('chorus'), sendSlot('dottedEighth')]

  /** How many send effects there are. */
  sends(): number {
    return STYLE_SENDS + this.added.length
  }

  private strip(strip: number): StripSettings | string {
    return (Number.isInteger(strip) && this.strips[strip]) || `no strip ${strip} (0-11)`
  }

  private slot(strip: number, slot: number): InsertSlot | string {
    const s = this.strip(strip)
    if (typeof s === 'string') return s
    return (Number.isInteger(slot) && slot >= 0 && slot < INSERT_SLOTS && s.inserts[slot]) || `no insert slot ${slot} (0-1)`
  }

  private sendSlot(send: number): SendSlot | string {
    if (!Number.isInteger(send) || send < 0) return `no send ${send + 1} (there are ${this.sends()})`
    if (send < STYLE_SENDS) return this.styleSends[send]
    return this.added[send - STYLE_SENDS] ?? `no send ${send + 1} (there are ${this.sends()})`
  }

  /** Play a command here. A string: why it was refused (nothing changed). */
  apply(c: StripCmd): string | null {
    const level = (v: number) => Math.max(0, Math.min(127, Math.round(v)))
    switch (c.type) {
      case 'setStripEq': {
        const s = this.strip(c.strip)
        if (typeof s === 'string') return s
        s.eq = clampEq(c.eq)
        return null
      }
      case 'setStripCompressorOn': {
        const s = this.strip(c.strip)
        if (typeof s === 'string') return s
        s.comp.on = c.on
        return null
      }
      case 'setStripCompressorPreset': {
        const s = this.strip(c.strip)
        if (typeof s === 'string') return s
        if (!own(COMP_PARAMS, c.preset)) return `no compressor type ${JSON.stringify(c.preset)}`
        s.comp = compOf(s.comp.on, c.preset)
        return null
      }
      case 'setStripCompressorParam': {
        const s = this.strip(c.strip)
        if (typeof s === 'string') return s
        const range = own(COMP_RANGES, c.param)
        if (!range) return `no compressor parameter ${JSON.stringify(c.param)}`
        s.comp[c.param] = Math.max(range[0], Math.min(range[1], Math.round(c.value)))
        return null
      }
      case 'setStripInsertKind': {
        if (!own(INSERT_KINDS, c.kind)) return `no insert kind ${JSON.stringify(c.kind)}`
        const s = this.slot(c.strip, c.slot)
        if (typeof s === 'string') return s
        setInsertKind(s, c.kind)
        return null
      }
      case 'setStripInsertOn': {
        const s = this.slot(c.strip, c.slot)
        if (typeof s === 'string') return s
        s.on = c.on
        return null
      }
      case 'setStripInsertSetting': {
        const s = this.slot(c.strip, c.slot)
        if (typeof s === 'string') return s
        const spec = insertSettings(s.kind)[c.setting]
        if (!spec) return `${insertName(s.kind)} has no setting ${c.setting + 1}`
        s.values[c.setting] = clampTo(spec, c.value)
        return null
      }
      case 'setStripSend': {
        const n = this.sends()
        if (!Number.isInteger(c.send) || c.send < 0 || c.send >= n) return `no send ${c.send + 1} (there are ${n})`
        const s = this.strip(c.strip)
        if (typeof s === 'string') return s
        s.sends[c.send] = level(c.level)
        return null
      }
      case 'addSend':
        if (!own(SEND_KINDS, c.kind)) return `no send kind ${JSON.stringify(c.kind)}`
        if (this.sends() >= SENDS) return `there are already ${SENDS} sends`
        this.added.push(sendSlot(c.kind))
        return null
      case 'removeSend': {
        const s = c.send
        if (!Number.isInteger(s) || s < STYLE_SENDS || s >= this.sends()) return `send ${s + 1} can't be removed (only added ones: 4-${this.sends()})`
        this.added.splice(s - STYLE_SENDS, 1)
        for (const st of this.strips) {
          st.sends.splice(s, 1)
          st.sends.push(0)
        }
        return null
      }
      case 'setSendKind': {
        if (!own(SEND_KINDS, c.kind)) return `no send kind ${JSON.stringify(c.kind)}`
        if (Number.isInteger(c.send) && c.send >= 0 && c.send < STYLE_SENDS && sendBus(c.kind)?.[0] !== c.send) {
          return `send ${c.send + 1} plays ${['Reverb', 'Chorus', 'Variation'][c.send]} types`
        }
        const s = this.sendSlot(c.send)
        if (typeof s === 'string') return s
        setSendKind(s, c.kind)
        return null
      }
      case 'setSendParam': {
        const s = this.sendSlot(c.send)
        if (typeof s === 'string') return s
        const spec = sendParams(s.kind)[c.param]
        if (!spec) return `${sendName(s.kind)} has no parameter ${c.param + 1}`
        s.params[c.param] = clampTo(spec, c.value)
        return null
      }
      case 'setSendReturn': {
        const s = this.sendSlot(c.send)
        if (typeof s === 'string') return s
        s.returnLevel = level(c.level)
        return null
      }
      case 'setRackSendOverride':
        if (!Number.isInteger(c.send) || c.send < 0 || c.send >= STYLE_SENDS) return `only sends 1-3 have an override (not ${c.send + 1})`
        this.overrides[c.send] = c.on
        return null
    }
  }

  /** Put the strips and the send effects into `st` (`Strips::fill`): every part's `strip`
   * and `effects.sends`, taking what older state already has from it. */
  fill(st: AppState) {
    st.effects.blocks.slice(0, STYLE_SENDS).forEach((b, i) => {
      const s = this.styleSends[i]
      const kind = BLOCKS[i] === b.block ? (Object.keys(SEND_KINDS).find((k) => SEND_KINDS[k].bus?.[0] === i && SEND_KINDS[k].bus?.[1] === b.effect) ?? null) : null
      if (kind !== null && s.kind !== kind) setSendKind(s, kind)
      b.params.forEach((p, j) => {
        if (j < SEND_PARAMS) s.params[j] = p.value
      })
      s.returnLevel = b.returnLevel
    })
    st.effects.sends = [
      ...this.styleSends.map((s, i) => sendState(i, s, true, this.overrides[i])),
      ...this.added.map((s, i) => sendState(STYLE_SENDS + i, s, false, true)),
    ]

    const n = this.sends()
    const state = (s: StripSettings): StripState => ({
      eq: { ...s.eq },
      comp: compState(s.comp),
      inserts: s.inserts.map(slotState),
      sends: s.sends.map((v, i) => (i < n ? v : 0)),
    })
    // Insert 1 from an older slot: its kind, on/off and amount; the other settings this
    // one's while the kind is the same.
    const slot1 = (mine: InsertSlot, kind: InsertType, on: boolean, amount: number) => {
      if (mine.kind !== kind) setInsertKind(mine, kind)
      mine.on = on
      if (insertSettings(mine.kind).length) mine.values[0] = amount
    }
    st.keyboardParts.slice(0, KEYBOARD_STRIPS).forEach((kp, p) => {
      const s = this.strips[p]
      s.eq = { ...kp.eq }
      s.sends.splice(0, STYLE_SENDS, kp.reverb, kp.chorus, kp.variation)
      const old = fromPartInsert(kp.insert)
      slot1(s.inserts[0], old.kind, old.on, kp.insert.amount)
      kp.strip = state(s)
    })
    st.mixer.styleParts.slice(0, 8).forEach((sp, p) => {
      const s = this.strips[KEYBOARD_STRIPS + p]
      s.sends.splice(0, STYLE_SENDS, sp.reverb, sp.chorus, sp.variation)
      const i = st.effects.inserts.find((x) => x.part === p)
      if (i) slot1(s.inserts[0], i.effect ?? 'none', i.on, i.amount)
      sp.strip = state(s)
    })
  }
}
