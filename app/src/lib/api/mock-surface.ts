// The mock's `state.surface`: a port of the engine's `Session::surface` (src/session.rs)
// with the button colours of src/launchkey.rs (`button_colours`, `palette_colour`), so the
// browser mock sends what the engine sends. Mock only: the UI reads `state.surface`.

import type { AppCmd, AppState, ClockState, ControlId, Level, LibraryList, Neighbour, Rgb, SurfaceControl, SurfaceFader, SurfaceState } from './types'
import { PAD_PAGES, STYLE_PART_NAMES } from './types'

// Novation palette indices (src/launchkey.rs) and how they look.
const OFF = 0
const WHITE = 3
const CYAN = 37
const GREEN = 21
const DIM_GREEN = 23
const BLUE = 45
const DIM_BLUE = 47
const PINK = 57
const PURPLE = 53
const DIM_PURPLE = 55
const ORANGE = 9
const PALETTE: Record<number, [Rgb, Level]> = {
  [WHITE]: [[127, 127, 127], 'bright'],
  [CYAN]: [[0, 100, 127], 'bright'],
  [PINK]: [[127, 0, 70], 'bright'],
  [ORANGE]: [[127, 60, 0], 'bright'],
  [GREEN]: [[0, 127, 0], 'bright'],
  [DIM_GREEN]: [[0, 127, 0], 'dim'],
  [BLUE]: [[0, 0, 127], 'bright'],
  [DIM_BLUE]: [[0, 0, 127], 'dim'],
  [PURPLE]: [[90, 0, 127], 'bright'],
  [DIM_PURPLE]: [[90, 0, 127], 'dim'],
}
const PAGE_COLOUR = [WHITE, CYAN, PINK, ORANGE]

/** The Panel-page fader button (0-based) that is the HARMONY/ARPEGGIO switch (src/launchkey.rs). */
const HARM_ARP_FADER_BTN = 4
const PART_LABELS = ['RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT']
const SELECT_LABELS = ['EDIT R1', 'EDIT R2', 'EDIT R3', 'EDIT L']

/** The styles Track ◀/▶ load: library order, skipping unreadable files, wrapping. */
export function neighbours(lib: LibraryList, position: number): { prev: Neighbour | null; next: Neighbour | null } {
  const n = lib.entries.length
  const step = (d: number): Neighbour | null => {
    let i = position
    for (let k = 0; k < n; k++) {
      i = (((i + d) % n) + n) % n
      if (i === position) return null
      const e = lib.entries[i]
      if (e.status !== 'error') return { id: e.id, name: e.name, path: e.path }
    }
    return null
  }
  return n ? { prev: step(-1), next: step(1) } : { prev: null, next: null }
}

export interface MockHardware {
  /** Where the physical faders 1–8 and master are (0–127), null until moved. */
  faders: (number | null)[]
  clock: ClockState
}

/** The surface as the engine reports it (src/session.rs `surface`). */
export function mockSurface(s: AppState, lib: LibraryList, hw: MockHardware): SurfaceState {
  const page = PAD_PAGES.findIndex((p) => p.id === s.pads.page)
  const styles = lib.entries.filter((e) => e.status !== 'error').length > 1
  // Shift + Track: the Playlist's previous/next record, when it has any.
  const songs = s.playlist.records.length > 0
  const style = s.mixer.faderPage === 'style'
  const pageColour = PAGE_COLOUR[page]

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
    const to = Math.max(0, Math.min(PAD_PAGES.length - 1, page + d))
    return to === page ? null : { type: 'setPadPage', page: PAD_PAGES[to].id }
  }

  const controls: SurfaceControl[] = [
    control('padBankUp', 106, 'PAGE ▲', toPage(-1), page > 0 ? pageColour : OFF, { label: 'LEFT', action: { type: 'togglePart', part: 3 } }),
    control('padBankDown', 107, 'PAGE ▼', toPage(1), page < PAD_PAGES.length - 1 ? pageColour : OFF, { label: 'OTS LINK', action: { type: 'toggleOtsLink' } }),
    control('trackPrev', 103, '◀ STYLE', styles ? { type: 'stepStyle', delta: -1 } : null, styles ? WHITE : OFF,
      { label: '◀ SONG', action: songs ? { type: 'stepPlaylist', delta: -1 } : null }),
    control('trackNext', 102, 'STYLE ▶', styles ? { type: 'stepStyle', delta: 1 } : null, styles ? WHITE : OFF,
      { label: 'SONG ▶', action: songs ? { type: 'stepPlaylist', delta: 1 } : null }),
    control('play', 115, 'PLAY', { type: 'startStop' }, null),
    control('stop', 116, 'STOP', { type: 'stop' }, null),
    control('scene', 104, 'TEMPO +', { type: 'tempoUp' }, null),
    control('function', 105, 'TEMPO -', { type: 'tempoDown' }, null),
  ]
  for (let i = 0; i < 8; i++) {
    const id = `faderButton${i + 1}` as ControlId
    const cc = 37 + i
    if (style) {
      const p = s.mixer.styleParts[i]
      const on = p.on && !p.mutedByManualBass
      controls.push(control(id, cc, STYLE_PART_NAMES[i].toUpperCase(), { type: 'toggleStylePart', part: i }, on ? GREEN : DIM_GREEN))
    } else if (i < 4) {
      const on = s.keyboardParts[i].sounding
      controls.push(control(id, cc, PART_LABELS[i], { type: 'togglePart', part: i }, on ? BLUE : DIM_BLUE, {
        label: SELECT_LABELS[i], action: { type: 'selectPart', part: i },
      }))
    } else if (i === HARM_ARP_FADER_BTN) {
      controls.push(control(id, cc, 'HARM/ARP', { type: 'toggleHarmonyArp' }, s.harmonyArp.on ? PURPLE : DIM_PURPLE))
    } else controls.push(control(id, cc, '', null, OFF))
  }
  controls.push(control('masterButton', 45, style ? 'STYLE' : 'PANEL', { type: 'toggleFaderPage' }, style ? GREEN : BLUE))

  const faders: SurfaceFader[] = Array.from({ length: 8 }, (_, i): SurfaceFader => {
    const position = hw.faders[i] ?? null
    if (style) {
      const p = s.mixer.styleParts[i]
      return { label: STYLE_PART_NAMES[i].toUpperCase(), value: p.volume, waiting: p.waiting, position, set: { type: 'setStylePartVolume', part: i, volume: 0 } }
    }
    const p = s.keyboardParts[i]
    if (!p) return { label: '', value: null, waiting: false, position, set: null }
    return { label: PART_LABELS[i], value: p.volume, waiting: p.waiting, position, set: { type: 'setPartVolume', part: i, volume: 0 } }
  })
  const masterPos = hw.faders[8] ?? null
  faders.push(
    s.mixer.master === null
      ? { label: '', value: null, waiting: false, position: masterPos, set: null }
      : { label: 'MASTER', value: s.mixer.master, waiting: s.mixer.masterWaiting, position: masterPos, set: { type: 'setMasterVolume', volume: 0 } },
  )

  const near = neighbours(lib, s.library.position)
  return { shift: false, controls, faders, trackPrev: near.prev, trackNext: near.next, clock: hw.clock }
}

/** ClockState read at `t` (ms): bar, beat and phase moved on (the engine's `ClockState::at`). */
export function clockAt(c: ClockState, t: number): ClockState {
  const pos = c.running ? Math.max(0, c.sectionAnchorBeats + ((t - c.sectionAnchorMs) * c.tempo) / 60000) : 0
  const bpb = c.beatsPerBar > 0 ? c.beatsPerBar : 4
  return { ...c, atMs: t, bar: Math.floor(pos / bpb) + 1, beat: Math.floor(pos % bpb) + 1, phase: pos - Math.floor(pos) }
}
