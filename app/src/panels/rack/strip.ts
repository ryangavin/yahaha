// Panel-only helpers for the rack's channel strips and send effects (the mixer rework):
// the kind lists the selects offer and the strip's words. Everything here reads engine
// state and only words it for the screen.

import { INSERT_EFFECTS, type InsertType, type PartEq, type SendKind, type SendState, type SettingState } from '../../lib/api/types'

/** Send effects there can be: 1-3 the style's reverb, chorus and delay, 4-6 added. */
export const MAX_SENDS = 6
/** Sends 0-2 come from the style. */
export const STYLE_SENDS = 3

/** What an insert slot can play, in the order the select lists them ('none' empties it). */
export const INSERT_KIND_OPTIONS: { kind: InsertType; name: string }[] = [{ kind: 'none', name: 'None' }, ...INSERT_EFFECTS.map((e) => ({ kind: e.effect as InsertType, name: e.name }))]

/** What an added send (4-6) can play: any of the buses' types, and the phaser. */
export const SEND_KIND_OPTIONS: { kind: SendKind; name: string }[] = [
  { kind: 'hall', name: 'Hall' },
  { kind: 'room', name: 'Room' },
  { kind: 'stage', name: 'Stage' },
  { kind: 'plate', name: 'Plate' },
  { kind: 'chorus', name: 'Chorus' },
  { kind: 'celeste', name: 'Celeste' },
  { kind: 'flanger', name: 'Flanger' },
  { kind: 'eighth', name: 'Delay 1/8' },
  { kind: 'dottedEighth', name: 'Delay 1/8.' },
  { kind: 'quarter', name: 'Delay 1/4' },
  { kind: 'pingPong', name: 'Ping-Pong' },
  { kind: 'phaser', name: 'Phaser' },
]

/** Sends 1-3's roles, as the rack names them. */
export const STYLE_SEND_ROLES = ['Reverb', 'Chorus', 'Delay']

/** The sends the player added (4-6), as they are now. */
export function addedSends(sends: SendState[]): SendState[] {
  return sends.filter((s) => s.send >= STYLE_SENDS)
}

const db = (g: number) => (g > 0 ? `+${g}` : g < 0 ? `−${-g}` : '0')
const hz = (f: number) => (f >= 1000 ? `${Number((f / 1000).toFixed(1))}k` : `${f}`)

/** A strip's EQ in a few words: "EQ flat", "EQ Lo +3 @80 · Hi −2 @10k". */
export function eqSummary(eq: PartEq): string {
  const bands = [eq.lowGain ? `Lo ${db(eq.lowGain)} @${hz(eq.lowFreq)}` : '', eq.highGain ? `Hi ${db(eq.highGain)} @${hz(eq.highFreq)}` : ''].filter(Boolean)
  return bands.length ? `EQ ${bands.join(' · ')}` : 'EQ flat'
}

/** A setting's readout: the engine's own for the value it holds, else the number. */
export function settingFormat(s: SettingState): (v: number) => string {
  return (v) => (v === s.value ? s.display : String(v))
}

/** The insert-setting tooltips, by setting index 0-3. */
export const INSERT_SETTING_TIPS = ['mixer.strip.insert_setting_1', 'mixer.strip.insert_setting_2', 'mixer.strip.insert_setting_3', 'mixer.strip.insert_setting_4'] as const
