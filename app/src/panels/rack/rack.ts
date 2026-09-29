// Panel-only helpers for the Rack panel (docs/racks.md "Screens") and the sound names
// under the Launchkey mirror's part faders. Everything here reads engine state and only
// words it for the screen.

import type { ControlTarget, KeyboardPart, LiveRackState } from '../../lib/api/types'

export const PART_NAMES = ['Right 1', 'Right 2', 'Right 3', 'Left']

/** The rack's name as the head shows it: a rack that came from none is "Untitled rack". */
export function rackName(r: LiveRackState): string {
  return r.id === null && (r.name === '' || r.name === 'New rack') ? 'Untitled rack' : r.name
}

/** The sound a part plays, by name: its Sound, else its voice. */
export function soundName(p: KeyboardPart): string {
  return p.sound?.name ?? p.voiceName
}

/** Its plugin isn't installed: the part is silent until it's replaced or reinstalled. */
export const isMissing = (p: KeyboardPart) => !!p.plugin?.missing

/** The sound's name with its state marks, as the fader shows it: "⚠ Pad", "Pad ●". */
export function soundLabel(p: KeyboardPart): string {
  return `${isMissing(p) ? '⚠ ' : ''}${soundName(p)}${p.soundEdited ? ' ●' : ''}`
}

/** Where a part's sound comes from: one of your sounds, a plugin's own, or a SoundFont preset. */
export type SoundBadge = 'Mine' | 'Factory' | 'SoundFont'

/** The badge for the sound a part plays (`playing` is its catalog id, from `playingId`). */
export function soundBadge(p: KeyboardPart, playing: string | null): SoundBadge {
  if (playing?.startsWith('saved:') || (!p.plugin && p.patch)) return 'Mine'
  if (p.plugin || playing?.startsWith('au:')) return 'Factory'
  return 'SoundFont'
}

/** What a controller-map target does, in words: "Right 2 level", "Split point", "—". */
export function targetLabel(t: ControlTarget | undefined): string {
  if (!t) return '—'
  switch (t.kind) {
    case 'none':
      return '—'
    case 'partLevel':
      return `${PART_NAMES[t.part] ?? '?'} level`
    case 'partPan':
      return `${PART_NAMES[t.part] ?? '?'} pan`
    case 'partReverb':
      return `${PART_NAMES[t.part] ?? '?'} reverb`
    case 'partChorus':
      return `${PART_NAMES[t.part] ?? '?'} chorus`
    case 'partDelay':
      return `${PART_NAMES[t.part] ?? '?'} delay`
    case 'partInsertOn':
      return `${PART_NAMES[t.part] ?? '?'} insert ${t.slot + 1} on/off`
    case 'partInsertSetting':
      return `${PART_NAMES[t.part] ?? '?'} insert ${t.slot + 1} setting ${t.setting + 1}`
    case 'partSend':
      return `${PART_NAMES[t.part] ?? '?'} send ${t.send + 1}`
    case 'rotaryFast':
      return 'Rotary fast/slow'
    case 'harmonyArp':
      return 'Harmony/Arp on/off'
    case 'splitPoint':
      return 'Split point'
    case 'harmonyVolume':
      return 'Harmony volume'
    case 'metronomeVolume':
      return 'Metronome volume'
    case 'tempo':
      return 'Tempo'
    default:
      return 'Set by a newer yahaha'
  }
}

/** A target as a select's option value: "partPan:2", "partInsertSetting:0:1:2", "splitPoint". */
export function targetKey(t: ControlTarget | undefined): string {
  if (!t) return 'none'
  switch (t.kind) {
    case 'partInsertOn':
      return `${t.kind}:${t.part}:${t.slot}`
    case 'partInsertSetting':
      return `${t.kind}:${t.part}:${t.slot}:${t.setting}`
    case 'partSend':
      return `${t.kind}:${t.part}:${t.send}`
  }
  return 'part' in t ? `${t.kind}:${t.part}` : t.kind
}

/** The targets a fader or knob can have, in the order the Controller map lists them (the
 * wireframe's `mapTable`): none; each part's level, pan, reverb, chorus and delay, its
 * inserts' on/off and settings 1-4, and its sends 4-6 (sends 1-3 are its reverb, chorus
 * and delay); then Harmony/Arp, the rotary speed, the split, Harmony and Metronome
 * volume, and (knobs only) Tempo. */
export function targetOptions(control: 'fader' | 'knob'): { key: string; label: string; target: ControlTarget }[] {
  const targets: ControlTarget[] = [
    { kind: 'none' },
    ...[0, 1, 2, 3].flatMap((part): ControlTarget[] => [
      { kind: 'partLevel', part },
      { kind: 'partPan', part },
      { kind: 'partReverb', part },
      { kind: 'partChorus', part },
      { kind: 'partDelay', part },
      ...[0, 1].flatMap((slot): ControlTarget[] => [
        { kind: 'partInsertOn', part, slot },
        ...[0, 1, 2, 3].map((setting): ControlTarget => ({ kind: 'partInsertSetting', part, slot, setting })),
      ]),
      ...[3, 4, 5].map((send): ControlTarget => ({ kind: 'partSend', part, send })),
    ]),
    { kind: 'harmonyArp' },
    { kind: 'rotaryFast' },
    { kind: 'splitPoint' },
    { kind: 'harmonyVolume' },
    { kind: 'metronomeVolume' },
    ...(control === 'knob' ? [{ kind: 'tempo' } as ControlTarget] : []),
  ]
  return targets.map((target) => ({ key: targetKey(target), label: targetLabel(target), target }))
}

/** A pan position as the slot shows it: "C", "L20", "R12". */
export function panLabel(v: number): string {
  return v === 64 ? 'C' : v < 64 ? `L${64 - v}` : `R${v - 64}`
}

/** A transpose as a signed label: "0", "+2", "−3". */
export function signed(n: number): string {
  return n > 0 ? `+${n}` : n < 0 ? `−${-n}` : '0'
}
