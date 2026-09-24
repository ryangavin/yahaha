// The Launchkey surface beyond the pads: what every button and fader does on the current
// page and Shift layer, how it's lit, and the beat clock. The engine will send this as
// `state.surface` (the API follow-up to #71); until then `deriveSurface` computes it from
// the rest of the state with the rules in src/launchkey.rs. The mirror only ever reads
// `surfaceOf(state)`, so when the engine sends it, nothing in the UI changes.

import { tipFor } from '../help/actions'
import type { TipKey } from '../help/tooltips'
import type { AppCmd, AppState, ControlId, LibraryEntry, LibraryList, Rgb, SurfaceControl, SurfaceFader, SurfaceState } from './api/types'
import { PAD_PAGES } from './api/types'

const PAGE_RGB: Record<string, Rgb> = { sections: [100, 100, 100], chordSetup: [0, 100, 127], otsParts: [127, 0, 70] }
const WHITE: Rgb = [100, 100, 100]
const BLUE: Rgb = [10, 50, 127]
const GREEN: Rgb = [0, 127, 16]
const PART_SHORT = ['R1', 'R2', 'R3', 'Left']

type SetCmd = NonNullable<SurfaceFader['set']>

/** The styles < Track and Track > would load: library order, skipping unreadable files, wrapping. */
export function neighbours(lib: LibraryList, position: number): { prev: LibraryEntry | null; next: LibraryEntry | null } {
  const n = lib.entries.length
  const step = (d: number): LibraryEntry | null => {
    let i = position
    for (let k = 0; k < n; k++) {
      i = (((i + d) % n) + n) % n
      if (i === position) return null
      if (lib.entries[i].status !== 'error') return lib.entries[i]
    }
    return null
  }
  return n ? { prev: step(-1), next: step(1) } : { prev: null, next: null }
}

function control(
  id: ControlId, label: string, action: AppCmd | null, rgb: Rgb, on: boolean | null,
  shift?: { label: string; action: AppCmd | null },
): SurfaceControl {
  return {
    id, label, action, rgb,
    shiftLabel: shift?.label ?? label,
    shiftAction: shift ? shift.action : action,
    level: on === null ? 'off' : on ? 'bright' : 'dim',
    anim: 'solid',
  }
}

/** The surface as the engine would report it, from the rest of the state. */
export function deriveSurface(s: AppState, lib: LibraryList): SurfaceState {
  const page = PAD_PAGES.findIndex((p) => p.id === s.pads.page)
  const pageRgb = PAGE_RGB[s.pads.page]
  const near = neighbours(lib, s.library.position)
  const style = s.mixer.faderPage === 'style'
  const parts = s.keyboardParts

  const faderButtons = Array.from({ length: 8 }, (_, i): SurfaceControl => {
    const id = `faderButton${i + 1}` as ControlId
    if (style) {
      const p = s.mixer.styleParts[i]
      return control(id, p.on ? p.name : `${p.name} off`, { type: 'toggleStylePart', part: i }, GREEN, p.on)
    }
    const p = parts[i]
    if (!p) return control(id, '', null, BLUE, null)
    return control(id, PART_SHORT[i], { type: 'togglePart', part: i }, BLUE, p.sounding, { label: `Edit ${PART_SHORT[i]}`, action: { type: 'selectPart', part: i } })
  })

  const controls: SurfaceControl[] = [
    control('padBankUp', '▲', page > 0 ? { type: 'setPadPage', page: PAD_PAGES[page - 1].id } : null, pageRgb, page > 0 ? true : null, {
      label: 'Left', action: { type: 'togglePart', part: 3 },
    }),
    control('padBankDown', '▼', page < PAD_PAGES.length - 1 ? { type: 'setPadPage', page: PAD_PAGES[page + 1].id } : null, pageRgb, page < PAD_PAGES.length - 1 ? true : null, {
      label: 'OTS Link', action: { type: 'toggleOtsLink' },
    }),
    control('trackPrev', '◀', { type: 'stepStyle', delta: -1 }, WHITE, near.prev ? true : null),
    control('trackNext', '▶', { type: 'stepStyle', delta: 1 }, WHITE, near.next ? true : null),
    control('play', 'Play', { type: 'startStop' }, [0, 127, 0], s.transport.running ? true : null),
    control('stop', 'Stop', { type: 'stop' }, WHITE, null),
    control('scene', 'Tempo +', { type: 'tempoUp' }, WHITE, null),
    control('function', 'Tempo −', { type: 'tempoDown' }, WHITE, null),
    ...faderButtons,
    control('masterButton', style ? 'Style' : 'Panel', { type: 'toggleFaderPage' }, style ? GREEN : BLUE, true),
  ]

  const fader = (label: string, value: number | null, waiting: boolean, set: SetCmd | null): SurfaceFader => ({ label, value, waiting, position: null, set })
  const faders: SurfaceFader[] = [
    ...Array.from({ length: 8 }, (_, i) => {
      if (style) {
        const p = s.mixer.styleParts[i]
        return fader(p.name, p.volume, p.waiting, { type: 'setStylePartVolume', part: i, volume: 0 })
      }
      const p = parts[i]
      return p ? fader(PART_SHORT[i], p.volume, p.waiting, { type: 'setPartVolume', part: i, volume: 0 }) : fader('', null, false, null)
    }),
    fader('Master', s.mixer.master, s.mixer.masterWaiting, s.mixer.master === null ? null : { type: 'setMasterVolume', volume: 0 }),
  ]

  return {
    shift: false,
    controls,
    faders,
    trackPrev: near.prev ? { id: near.prev.id, name: near.prev.name, path: near.prev.path } : null,
    trackNext: near.next ? { id: near.next.id, name: near.next.name, path: near.next.path } : null,
    clock: { bar: s.transport.bar, beat: s.transport.beat, phase: 0, tempo: s.transport.tempo, atMs: 0 },
  }
}

/** The surface: the engine's when it sends one, else derived. */
export function surfaceOf(s: AppState, lib: LibraryList): SurfaceState {
  return s.surface ?? deriveSurface(s, lib)
}

/** A control's label and action on the layer showing. */
export function layer(c: SurfaceControl, shift: boolean): { label: string; action: AppCmd | null } {
  return shift ? { label: c.shiftLabel, action: c.shiftAction } : { label: c.label, action: c.action }
}

/** The catalog entry for a control on the layer showing. */
export function controlTip(c: SurfaceControl, shift: boolean): TipKey {
  const { action } = layer(c, shift)
  if (!shift && c.id === 'padBankUp') return 'padpage.prev'
  if (!shift && c.id === 'padBankDown') return 'padpage.next'
  if (c.id === 'trackPrev') return 'style.prev'
  if (c.id === 'trackNext') return 'style.next'
  if (!action) return c.id.startsWith('faderButton') ? 'launchkey.fader_unused' : 'launchkey.unused'
  return tipFor(action)
}

/** The catalog entry for a fader. */
export function faderTip(f: SurfaceFader): TipKey {
  if (!f.set) return 'launchkey.fader_unused'
  if (f.set.type === 'setPartVolume') return tipFor(f.set)
  if (f.set.type === 'setStylePartVolume') return 'mixer.style.volume'
  return 'mixer.master'
}

/** The command moving a fader to `v` sends. */
export function faderCmd(f: SurfaceFader, v: number): AppCmd | null {
  return f.set ? ({ ...f.set, volume: v } as AppCmd) : null
}
