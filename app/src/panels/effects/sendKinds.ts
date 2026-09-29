// The send effect kinds the Effects screen offers for an added send (`SendKind`), in the
// order the picker lists them, with their names. A kind the engine adds later still shows
// on its card by the state's own `name`.

import type { SendKind } from '../../lib/api/types'

export const SEND_KINDS: { kind: SendKind; name: string }[] = [
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

/** How many send effects there can be: the style's 3 and 3 the player adds. */
export const MAX_SENDS = 6
/** Sends 0-2 are the style's buses; later ones are added. */
export const STYLE_SENDS = 3

/** A return level as the Genos shows it: 64 = 0 dB, 127 = +6 dB, 0 = off. */
export const returnText = (v: number) => (v === 0 ? 'Off' : `${v >= 64 ? '+' : ''}${(20 * Math.log10(v / 64)).toFixed(1)} dB`)
