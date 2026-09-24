// The Chord Looper drawer's view logic, kept out of the component so it can be tested.

import type { LoopChord, LooperMode, LooperState, Pad } from '../../lib/api/types'

type Led = Pick<Pad, 'rgb' | 'level' | 'anim'>

const RED: [number, number, number] = [127, 8, 8]
const ORANGE: [number, number, number] = [127, 52, 0]
const BLUE: [number, number, number] = [10, 40, 127]

/** The REC/STOP and ON/OFF lamps as the Genos lights them (RM p.14–19): flashing while
 *  armed for the next bar line, solid while recording or looping; ON/OFF blue when there
 *  is a sequence that isn't looping. */
export function looperLeds(lp: Pick<LooperState, 'mode' | 'hasData'>): { rec: Led | null; onOff: Led | null } {
  const lit = (rgb: [number, number, number], anim: Led['anim']): Led => ({ rgb, level: 'bright', anim })
  return {
    rec: lp.mode === 'recArmed' ? lit(RED, 'flash') : lp.mode === 'recording' ? lit(RED, 'solid') : null,
    onOff:
      lp.mode === 'loopArmed'
        ? lit(ORANGE, 'flash')
        : lp.mode === 'looping'
          ? lit(ORANGE, 'solid')
          : lp.hasData
            ? { rgb: BLUE, level: 'dim', anim: 'solid' }
            : null,
  }
}

/** What the looper is doing, in words. */
export function modeText(mode: LooperMode, running: boolean): string {
  switch (mode) {
    case 'recArmed':
      return running ? 'Recording starts at the next bar' : 'Play a chord: the band and the recording start together'
    case 'recording':
      return 'Recording'
    case 'loopArmed':
      return running ? 'The loop starts at the next bar' : 'The loop starts with the band'
    case 'looping':
      return 'Looping: your chords are ignored, both hands are free'
    case 'off':
      return 'Off'
  }
}

/** The sequence as bars 1…bars, each with its chord changes (empty: the chord holds). */
export function barsOf(lp: Pick<LooperState, 'bars' | 'chords'>): { bar: number; chords: LoopChord[] }[] {
  return Array.from({ length: lp.bars }, (_, i) => ({ bar: i + 1, chords: lp.chords.filter((c) => c.bar === i + 1) }))
}

/** A chord's place in its bar, when it isn't the downbeat: "b3", "b2½". */
export function beatText(beat: number): string {
  const whole = Math.floor(beat)
  const frac = beat - whole
  const part = frac === 0 ? '' : frac === 0.5 ? '½' : frac === 0.25 ? '¼' : frac === 0.75 ? '¾' : `.${Math.round(frac * 100)}`
  return `b${whole}${part}`
}
