// The mock's `state.surface`: a port of the engine's `Session::surface` (src/session.rs)
// with the button colours of src/launchkey.rs (`button_colours`, `palette_colour`), so the
// browser mock sends what the engine sends. Mock only: the UI reads `state.surface`.

import type { AppCmd, AppState, ClockState, ControlId, FaderLayer, Level, LibraryList, PartSend, Rgb, SurfaceControl, SurfaceFader, SurfaceState } from './types'
import type { Layer, PadPage } from './types'
import { STYLE_PART_NAMES } from './types'
import { neighbours } from './constants'
import { MockKnobs, faderRoute, rackFn } from './mock-knobs'

/** Reads a controller map target for a fader's label and level (no knob state of its own). */
const RACK_READER = new MockKnobs()

// Novation palette indices (src/launchkey.rs) and how they look.
const OFF = 0
const WHITE = 3
const DIM_WHITE = 1
const CYAN = 37
const DIM_CYAN = 39
const YELLOW = 13
const DIM_PINK = 59
const GREEN = 21
const DIM_GREEN = 23
const BLUE = 45
const DIM_BLUE = 47
const PINK = 57
const PURPLE = 53
const DIM_PURPLE = 55
const ORANGE = 9
const DIM_ORANGE = 11
const RED = 5
const DIM_RED = 7
const DIM_YELLOW = 15
const PALETTE: Record<number, [Rgb, Level]> = {
  [WHITE]: [[127, 127, 127], 'bright'],
  [DIM_WHITE]: [[127, 127, 127], 'dim'],
  [CYAN]: [[0, 100, 127], 'bright'],
  [DIM_CYAN]: [[0, 100, 127], 'dim'],
  [YELLOW]: [[127, 127, 0], 'bright'],
  [PINK]: [[127, 0, 70], 'bright'],
  [DIM_PINK]: [[127, 0, 70], 'dim'],
  [ORANGE]: [[127, 60, 0], 'bright'],
  [DIM_ORANGE]: [[127, 60, 0], 'dim'],
  [GREEN]: [[0, 127, 0], 'bright'],
  [DIM_GREEN]: [[0, 127, 0], 'dim'],
  [BLUE]: [[0, 0, 127], 'bright'],
  [DIM_BLUE]: [[0, 0, 127], 'dim'],
  [PURPLE]: [[90, 0, 127], 'bright'],
  [RED]: [[127, 0, 0], 'bright'],
  [DIM_PURPLE]: [[90, 0, 127], 'dim'],
  [DIM_RED]: [[127, 0, 0], 'dim'],
  [DIM_YELLOW]: [[127, 127, 0], 'dim'],
}
/** Each pad page's colour (src/launchkey.rs `Page::colour`). */
const PAGE_COLOUR: Record<PadPage, number> = { sections: WHITE, racks: ORANGE, chord: CYAN, multiPads: YELLOW, setup: PINK }
/** The Panel fader page's colour (bright, dim) in each fader layer (src/launchkey.rs `layer_colour`). */
const LAYER_COLOUR: Record<FaderLayer, [number, number]> = {
  volume: [BLUE, DIM_BLUE],
  pan: [YELLOW, DIM_YELLOW],
  reverb: [CYAN, DIM_CYAN],
  chorus: [PINK, DIM_PINK],
  delay: [WHITE, DIM_WHITE],
}

/** The effect send each fader layer moves (none for VOL and PAN). */
const LAYER_SEND: Record<FaderLayer, PartSend | null> = { volume: null, pan: null, reverb: 'reverb', chorus: 'chorus', delay: 'variation' }

/** The Panel-page fader button (0-based) that is the HARMONY/ARPEGGIO switch (src/launchkey.rs). */
const HARM_ARP_FADER_BTN = 4
/** The fader button that is Sound, on both fader pages (src/launchkey.rs `SOUND_FADER_BTN`):
 * hold it and the pads act and light as the Racks page. */
const SOUND_FADER_BTN = 5
/** The Panel-page fader button that is the CHORD LOOPER: ON/OFF, Shift REC/STOP (src/launchkey.rs). */
const LOOPER_FADER_BTN = 7
/** The Panel-page fader button that is LEFT HOLD (src/launchkey.rs). */
const LEFT_HOLD_FADER_BTN = 6
const PART_LABELS = ['RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT']
const SELECT_LABELS = ['EDIT R1', 'EDIT R2', 'EDIT R3', 'EDIT L']

// The styles Track ◀/▶ load (moved to ./constants; re-exported for mock code and tests).
export { neighbours }

export interface MockHardware {
  /** Where the physical faders 1–8 and master are (0–127), null until moved. */
  faders: (number | null)[]
  clock: ClockState
}

/** The surface as the engine reports it (src/session.rs `surface`). */
export function mockSurface(s: AppState, lib: LibraryList, hw: MockHardware): SurfaceState {
  // The page order Pad Bank ▲/▼ walk (`PageOrder::step`: stops at the ends; from a page
  // left out, steps from Sections).
  const order: PadPage[] = ['sections', ...s.settings.padPages]
  const page = Math.max(0, order.indexOf(s.pads.page))
  // The held control's layer: the mock has no hardware to hold, so it stays as it is.
  const held: Layer = s.surface?.layer ?? { type: 'none' }
  const soundHeld = held.type === 'sound'
  const styles = lib.entries.filter((e) => e.status !== 'error').length > 1
  const racks = s.quickRacks.buttons.some((b) => !!b.rack)
  const style = s.mixer.faderPage === 'style'
  const pageColour = PAGE_COLOUR[s.pads.page]
  const [layerOn, layerOff] = LAYER_COLOUR[s.mixer.faderLayer]

  const control = (
    id: ControlId, cc: number, label: string, action: AppCmd | null, colour: number | null,
    shift?: { label: string; action: AppCmd | null },
  ): SurfaceControl => {
    const [rgb, level]: [Rgb, Level] = colour === null ? [[0, 0, 0], 'off'] : (PALETTE[colour] ?? [[0, 0, 0], 'off'])
    const shown = action ? label : ''
    return {
      id, cc, label: shown, action,
      shiftLabel: shift ? (shift.action ? shift.label : '') : shown,
      shiftAction: shift ? shift.action : action,
      rgb, level, anim: 'solid', colour,
    }
  }
  const toPage = (d: number): AppCmd | null => {
    const to = Math.max(0, Math.min(order.length - 1, page + d))
    return to === page ? null : { type: 'setPadPage', page: order[to] }
  }

  const controls: SurfaceControl[] = [
    control('padBankUp', 106, 'PAGE ▲', toPage(-1), page > 0 ? pageColour : OFF, { label: 'LEFT', action: { type: 'togglePart', part: 3 } }),
    control('padBankDown', 107, 'PAGE ▼', toPage(1), page < order.length - 1 ? pageColour : OFF, { label: 'OTS LINK', action: { type: 'toggleOtsLink' } }),
    control('trackPrev', 103, '◀ STYLE', styles ? { type: 'stepStyle', delta: -1 } : null, styles ? WHITE : OFF,
      // Shift + Track: the previous/next Quick Rack in the bank on view, when it holds any.
      { label: '◀ RACK', action: racks ? { type: 'stepQuickRack', delta: -1 } : null }),
    control('trackNext', 102, 'STYLE ▶', styles ? { type: 'stepStyle', delta: 1 } : null, styles ? WHITE : OFF,
      { label: 'RACK ▶', action: racks ? { type: 'stepQuickRack', delta: 1 } : null }),
    control('play', 115, 'PLAY', { type: 'startStop' }, null, { label: 'RESET', action: { type: 'sectionReset' } }),
    control('stop', 116, 'STOP', { type: 'stop' }, null, { label: 'FADE', action: { type: 'toggleFade' } }),
    control('scene', 104, 'TEMPO +', { type: 'tempoUp' }, null, { label: 'RTG SHORT', action: { type: 'stepRetriggerRate', delta: 1 } }),
    control('function', 105, 'TEMPO -', { type: 'tempoDown' }, null, { label: 'RTG LONG', action: { type: 'stepRetriggerRate', delta: -1 } }),
  ]
  for (let i = 0; i < 8; i++) {
    const id = `faderButton${i + 1}` as ControlId
    const cc = 37 + i
    if (i === SOUND_FADER_BTN) {
      // Sound, on both fader pages: a hold (the input thread reads it), so no action; the
      // app shows the Racks page with setPadPage. White while held. With Shift on the Style
      // page it mutes Style part 6 (Pad), as the plain press did before.
      const shift = style
        ? { label: 'PAD', action: { type: 'toggleStylePart', part: i } as AppCmd }
        : { label: '', action: null }
      const b = control(id, cc, 'SOUND', null, soundHeld ? WHITE : DIM_WHITE, shift)
      controls.push({ ...b, label: 'SOUND' })
    } else if (style) {
      const p = s.mixer.styleParts[i]
      const on = p.on && !p.mutedByManualBass
      controls.push(control(id, cc, STYLE_PART_NAMES[i].toUpperCase(), { type: 'toggleStylePart', part: i }, on ? GREEN : DIM_GREEN))
    } else if (i < 4) {
      const on = s.keyboardParts[i].sounding
      controls.push(control(id, cc, PART_LABELS[i], { type: 'togglePart', part: i }, on ? layerOn : layerOff, {
        label: SELECT_LABELS[i], action: { type: 'selectPart', part: i },
      }))
    } else if (i === HARM_ARP_FADER_BTN) {
      controls.push(control(id, cc, 'HARM/ARP', { type: 'toggleHarmonyArp' }, s.harmonyArp.on ? PURPLE : DIM_PURPLE))
    } else if (i === LEFT_HOLD_FADER_BTN) {
      controls.push(control(id, cc, 'L HOLD', { type: 'toggleLeftHold' }, s.chord.leftHold ? ORANGE : DIM_ORANGE))
    } else if (i === LOOPER_FADER_BTN) {
      const l = s.looper
      const colour = { off: l.hasData ? DIM_GREEN : OFF, recArmed: DIM_RED, recording: RED, loopArmed: DIM_YELLOW, looping: GREEN }[l.mode]
      controls.push(control(id, cc, 'LOOPER', { type: 'looperOnOff' }, colour, { label: 'LOOP REC', action: { type: 'looperRec' } }))
    } else controls.push(control(id, cc, '', null, OFF))
  }
  const layer = { volume: '', pan: ' PAN', reverb: ' REV', chorus: ' CHO', delay: ' DLY' }[s.mixer.faderLayer]
  controls.push(control('masterButton', 45, (style ? 'STYLE' : 'PANEL') + layer, { type: 'toggleFaderPage' }, style ? GREEN : layerOn, { label: 'LAYER', action: { type: 'stepFaderLayer', delta: 1 } }))

  /** Panel fader 5 (0-based 4): the Style volume. */
  const STYLE_FADER = 4
  /** Panel fader 6: the Multi Pad volume. */
  const PAD_FADER = 5
  // A send layer: the faders show and set what the hardware moves there (src/session/surface.rs).
  const send = LAYER_SEND[s.mixer.faderLayer]
  const faders: SurfaceFader[] = Array.from({ length: 8 }, (_, i): SurfaceFader => {
    const position = hw.faders[i] ?? null
    if (style && s.mixer.faderLayer !== 'volume') {
      const p = s.mixer.styleParts[i]
      const label = STYLE_PART_NAMES[i].toUpperCase()
      // The Style parts have no pan control: the fader does nothing in PAN.
      if (!send) return { label, value: null, waiting: false, position, set: null }
      return { label, value: p[send], waiting: (s.mixer.styleSendWaiting & (1 << i)) !== 0, position, set: { type: 'setStylePartSend', part: i, send, value: 0 } }
    }
    if (!style && i < 4 && s.keyboardParts[i] && s.mixer.faderLayer !== 'volume') {
      const p = s.keyboardParts[i]
      const waiting = (s.mixer.sendWaiting & (1 << i)) !== 0
      if (!send) return { label: PART_LABELS[i], value: p.pan, waiting, position, set: { type: 'setPartPan', part: i, pan: 0 } }
      return { label: PART_LABELS[i], value: p[send], waiting, position, set: { type: 'setPartSend', part: i, send, value: 0 } }
    }
    if (style) {
      const p = s.mixer.styleParts[i]
      return { label: STYLE_PART_NAMES[i].toUpperCase(), value: p.volume, waiting: p.waiting, position, set: { type: 'setStylePartVolume', part: i, volume: 0 } }
    }
    if (i === STYLE_FADER) {
      return { label: 'STYLE', value: s.mixer.styleVolume, waiting: s.mixer.styleVolumeWaiting, position, set: { type: 'setStyleVolume', volume: 0 } }
    }
    if (i === PAD_FADER) {
      return { label: 'M.PAD', value: s.mixer.multiPadVolume, waiting: s.mixer.multiPadVolumeWaiting, position, set: { type: 'setMultiPadVolume', volume: 0 } }
    }
    const p = s.keyboardParts[i]
    if (!p) return { label: '', value: null, waiting: false, position, set: null }
    // Panel faders 1-4 in the Volume layer follow the live rack's controller map.
    const target = s.liveRack.controls.faders[i]
    const route = target && s.mixer.faderLayer === 'volume' ? faderRoute(target, i) : 'own'
    if (route === 'off') return { label: '', value: null, waiting: false, position, set: null }
    if (route === 'control') {
      const k = RACK_READER.read(rackFn(target), s)
      return { label: k.short.toUpperCase(), value: k.level, waiting: false, position, set: { type: 'moveRackFader', fader: i, volume: 0 } }
    }
    return { label: PART_LABELS[i], value: p.volume, waiting: p.waiting, position, set: { type: 'setPartVolume', part: i, volume: 0 } }
  })
  const masterPos = hw.faders[8] ?? null
  faders.push(
    s.mixer.master === null
      ? { label: '', value: null, waiting: false, position: masterPos, set: null }
      : { label: 'MASTER', value: s.mixer.master, waiting: s.mixer.masterWaiting, position: masterPos, set: { type: 'setMasterVolume', volume: 0 } },
  )

  const near = neighbours(lib, s.library.position)
  return { shift: false, layer: held, controls, faders, trackPrev: near.prev, trackNext: near.next, clock: hw.clock }
}

/** ClockState read at `t` (ms): bar, beat and phase moved on (the engine's `ClockState::at`). */
export function clockAt(c: ClockState, t: number): ClockState {
  const pos = c.running ? Math.max(0, c.sectionAnchorBeats + ((t - c.sectionAnchorMs) * c.tempo) / 60000) : 0
  const bpb = c.beatsPerBar > 0 ? c.beatsPerBar : 4
  return { ...c, atMs: t, bar: Math.floor(pos / bpb) + 1, beat: Math.floor(pos % bpb) + 1, phase: pos - Math.floor(pos) }
}
