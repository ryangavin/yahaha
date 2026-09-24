// The Launchkey control pressed last, in words, for the help footer: "PAD 6 (page 2)" and
// what it does now ("KBD TR +"). The engine sends the raw message (`io.lastControl`,
// packed 0x00SSDDVV); what each pad, button and fader does comes from the same state the
// mirror draws (`pads`, `surface`), so nothing here knows the mapping itself.

import type { AppState, ControlId } from '../api/types'
import { tipFor } from '../../help/actions'
import { TIPS, type TipKey } from '../../help/tooltips'
import { BOTTOM, TOP } from '../leds'

export interface LastControl {
  /** The hardware control: "PAD 6 (page 2)", "FADER 3", "TRACK ▶". */
  where: string
  /** What it does now: "MAIN B", "Right 2 volume 87". Empty when nothing. */
  what: string
  /** Its catalog entry, when it has one. */
  tip: TipKey | null
}

const CONTROL_NAMES: Record<ControlId, string> = {
  padBankUp: 'PAD BANK ▲',
  padBankDown: 'PAD BANK ▼',
  trackPrev: 'TRACK ◀',
  trackNext: 'TRACK ▶',
  play: 'PLAY',
  stop: 'STOP',
  scene: 'SCENE',
  function: 'FUNCTION',
  faderButton1: 'FADER BUTTON 1',
  faderButton2: 'FADER BUTTON 2',
  faderButton3: 'FADER BUTTON 3',
  faderButton4: 'FADER BUTTON 4',
  faderButton5: 'FADER BUTTON 5',
  faderButton6: 'FADER BUTTON 6',
  faderButton7: 'FADER BUTTON 7',
  faderButton8: 'FADER BUTTON 8',
  masterButton: 'MASTER BUTTON',
}

/** Faders 1–8 and master (launchkey.rs FADER_CC), Shift (SHIFT_CC). */
const FADER_CC = [5, 6, 7, 8, 9, 10, 11, 12, 13]
const SHIFT_CC = 63

/** `io.lastControl` decoded against the state now; null for none, Shift on its own, or a
 * message that isn't a control (the Launchkey's mode reports on channel 7). Decode it when
 * it changes, while that state is current: a later state may have let go of Shift. */
export function lastControl(packed: number, s: AppState): LastControl | null {
  const status = (packed >> 16) & 0xff
  const d1 = (packed >> 8) & 0xff
  const d2 = packed & 0xff
  const kind = status & 0xf0
  if (kind === 0x80 || kind === 0x90) {
    const top = TOP.indexOf(d1)
    const bottom = BOTTOM.indexOf(d1)
    if (top < 0 && bottom < 0) return null
    const n = top >= 0 ? top + 1 : bottom + 9
    const pad = s.pads.pads.find((p) => p.note === d1)
    const tip = pad ? tipFor(pad.action) : null
    return { where: `PAD ${n} (page ${s.pads.pageNumber})`, what: pad?.label || (tip ? TIPS[tip].title : ''), tip }
  }
  // Channel 1 only: channel 7 carries mode reports whose CC numbers overlap the buttons.
  if (status !== 0xb0) return null
  // Shift is a modifier: its release would hide the button it was held for.
  if (d1 === SHIFT_CC) return null
  const f = FADER_CC.indexOf(d1)
  if (f >= 0) {
    const fader = s.surface.faders[f]
    const where = f === 8 ? 'MASTER FADER' : `FADER ${f + 1}`
    const tip = fader?.set ? tipFor(fader.set) : null
    const what = fader?.label ? `${fader.label} ${fader.value ?? d2}` : ''
    return { where, what, tip }
  }
  const c = s.surface.controls.find((c) => c.cc === d1)
  if (!c) return { where: `CC ${d1}`, what: s.io.unmapped ? 'not mapped' : '', tip: null }
  const shifted = s.surface.shift && c.shiftAction
  const action = shifted ? c.shiftAction : c.action
  const label = shifted ? c.shiftLabel : c.label
  const tip = action ? tipFor(action) : null
  return { where: `${s.surface.shift ? 'SHIFT + ' : ''}${CONTROL_NAMES[c.id]}`, what: label || (tip ? TIPS[tip].title : ''), tip }
}
