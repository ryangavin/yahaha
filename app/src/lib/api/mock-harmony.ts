// The mock's Keyboard Harmony / Arpeggio: the type lists the engine publishes in
// `LibraryList` (docs/fixtures/library.json) and the settings commands, as
// src/session/harmony_arp.rs runs them. The mock plays no notes.

import type { HarmonyArpCmd, HarmonyArpState, HarmonyTypeInfo } from './types'

const H = (name: string, category = 'Harmony'): HarmonyTypeInfo => ({ name, category })

/** The Keyboard Harmony types, Data List order (`harmony::ALL_TYPES`). */
export const HARMONY_TYPES: HarmonyTypeInfo[] = [
  H('Standard Duet 1'), H('Standard Duet 2'), H('Standard Trio'), H('Full Chord'), H('Rock Duet'),
  H('Country Duet 1'), H('Country Duet 2'), H('Country Trio'), H('Block'), H('4-Way Close 1'),
  H('4-Way Close 2'), H('4-Way Close 3'), H('4-Way Close 4'), H('4-Way Open 1'), H('4-Way Open 2'),
  H('4-Way Open 3'), H('1+5'), H('Octave'), H('Strum'), H('Multi Assign'),
  H('Echo', 'Echo'), H('Tremolo', 'Echo'), H('Trill', 'Echo'),
]

/** yahaha's own arpeggio patterns (`arp::library::PATTERNS`). */
export const ARP_PATTERNS: HarmonyTypeInfo[] = [
  ...['Climb 16', 'Fall 16', 'Peak 8', 'Valley Triplet', 'Sky Ladder 32'].map((n) => H(n, 'Up & Down')),
  ...['Dice 16', 'Scatter Octaves'].map((n) => H(n, 'Random')),
  ...['Echo Order 8', 'Shuffle Order 16'].map((n) => H(n, 'As Played')),
  ...['Four Stabs', 'Offbeat Pump', 'Syncopated Hits', 'Gated Pad 16'].map((n) => H(n, 'Chord Stab')),
  ...['Alberti 16', 'Waltz Broken', 'Rolling Eights', 'Thumb Pick'].map((n) => H(n, 'Broken Chord')),
  ...['Strum Quarters', 'Campfire Strum', 'Muted Sixteens'].map((n) => H(n, 'Guitar')),
  ...['Octave Pulse', 'Root Fifth Seq', 'Pluck Line'].map((n) => H(n, 'Sequence')),
]

/** Off, Standard Duet 1, the engine's defaults. */
export function initialHarmonyArp(): HarmonyArpState {
  const h: HarmonyArpState = {
    on: false, mode: 'harmony', harmonyType: 0, arpPattern: 0, typeName: '', category: '',
    volume: 100, speed: '1/8', assign: 'auto', chordNoteOnly: false, touchLimit: 1,
    arp: { quantize: 'off', hold: false, velocity: 'original', fixedVelocity: 100, keepKeyOn: false },
  }
  name(h)
  return h
}

function name(h: HarmonyArpState) {
  const t = h.mode === 'harmony' ? HARMONY_TYPES[h.harmonyType] : ARP_PATTERNS[h.arpPattern]
  h.typeName = t.name
  h.category = t.category
}

/** Run a Harmony/Arpeggio command on `h`; an error message when refused. */
export function harmonyArpCmd(h: HarmonyArpState, cmd: HarmonyArpCmd): string | null {
  const clamp = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, Math.round(v)))
  switch (cmd.type) {
    case 'toggleHarmonyArp': h.on = !h.on; break
    case 'setHarmonyArpOn': h.on = cmd.on; break
    case 'setHarmonyType':
      if (!(cmd.index >= 0 && cmd.index < HARMONY_TYPES.length)) return `no Harmony type ${cmd.index} (0-${HARMONY_TYPES.length - 1})`
      h.mode = 'harmony'
      h.harmonyType = cmd.index
      break
    case 'setArpPattern':
      if (!(cmd.index >= 0 && cmd.index < ARP_PATTERNS.length)) return `no arpeggio pattern ${cmd.index} (0-${ARP_PATTERNS.length - 1})`
      h.mode = 'arpeggio'
      h.arpPattern = cmd.index
      break
    case 'stepHarmonyArpType': {
      const n = HARMONY_TYPES.length + ARP_PATTERNS.length
      const cur = h.mode === 'harmony' ? h.harmonyType : HARMONY_TYPES.length + h.arpPattern
      const next = (((cur + cmd.delta) % n) + n) % n
      if (next < HARMONY_TYPES.length) {
        h.mode = 'harmony'
        h.harmonyType = next
      } else {
        h.mode = 'arpeggio'
        h.arpPattern = next - HARMONY_TYPES.length
      }
      break
    }
    case 'setHarmonyVolume': h.volume = clamp(cmd.volume, 0, 127); break
    case 'setHarmonySpeed': h.speed = cmd.speed; break
    case 'setHarmonyAssign': h.assign = cmd.assign; break
    case 'setChordNoteOnly': h.chordNoteOnly = cmd.on; break
    case 'setTouchLimit': h.touchLimit = clamp(cmd.velocity, 1, 127); break
    case 'setArpQuantize': h.arp.quantize = cmd.quantize; break
    case 'setArpHold': h.arp.hold = cmd.on; break
    case 'toggleArpHold': h.arp.hold = !h.arp.hold; break
    case 'setArpVelocity':
      h.arp.velocity = cmd.mode
      // As the engine: only Fixed keeps a velocity of its own.
      h.arp.fixedVelocity = cmd.mode === 'fixed' ? clamp(cmd.velocity, 1, 127) : 100
      break
    case 'setArpKeepKeyOn': h.arp.keepKeyOn = cmd.on; break
  }
  name(h)
  return null
}
