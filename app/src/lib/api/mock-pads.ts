// The mock's pad lights: a port of `looks()` in `src/launchkey.rs` (same colours, labels,
// keys and lamp rules), producing the `Pad`s the engine puts in `AppState`. Only the mock
// uses this; with the real engine the pads come in the state.

import type { AppCmd, AppState, Anim, Level, Pad, PadPage, Rgb } from './types'
import { BREAK, ENDINGS, FILLS, FINGERINGS, INTROS, MAINS } from './types'

const C_INTRO: Rgb = [127, 95, 0]
const C_MAIN: Rgb = [0, 127, 16]
const C_ENDING: Rgb = [127, 0, 0]
const C_BREAK: Rgb = [90, 0, 127]
const C_SYNC: Rgb = [127, 45, 0]
const C_FILL: Rgb = [0, 45, 127]
const C_TAP: Rgb = [100, 100, 100]
const C_STOPSYNC: Rgb = [0, 110, 110]
const C_RUN: Rgb = [0, 127, 0]
const C_IDLE: Rgb = [127, 0, 0]
export const PAGE_RGB: Record<PadPage, Rgb> = { sections: C_TAP, chordSetup: [0, 100, 127], otsParts: [127, 0, 70] }

type Look = { rgb: Rgb; level: Level; anim: Anim }
const look = (rgb: Rgb, level: Level, anim: Anim = 'solid'): Look => ({ rgb, level, anim })
const toggle = (on: boolean, rgb: Rgb) => look(rgb, on ? 'bright' : 'dim')
const pad = (note: number, label: string, key: string, action: AppCmd | null, l: Look): Pad => ({ note, label, key, action, ...l, palette: null })

function sectionPads(s: AppState): Pad[] {
  const t = s.transport
  const has = (id: string) => s.style.sections.includes(id)
  const sec = (id: string, rgb: Rgb): Look =>
    !has(id) ? look(rgb, 'off') : t.queued === id ? look(rgb, 'bright', 'flash') : t.section === id ? look(rgb, 'bright') : look(rgb, 'dim')
  const main = (i: number): Look => {
    const id = MAINS[i]
    const fill = FILLS[i]
    if (!has(id)) return look(C_MAIN, 'off')
    if (t.queued === id || t.queued === fill || t.section === fill) return look(C_MAIN, 'bright', 'flash')
    if (t.section === id || (t.main === i && !(t.section && MAINS.includes(t.section)))) return look(C_MAIN, 'bright')
    return look(C_MAIN, 'dim')
  }
  const intro = (i: number): Look => (t.pendingIntro === i && has(INTROS[i]) ? look(C_INTRO, 'bright', 'pulse') : sec(INTROS[i], C_INTRO))
  return [
    pad(96, 'INTRO 1', 'q', { type: 'intro', index: 0 }, intro(0)),
    pad(97, 'INTRO 2', 'w', { type: 'intro', index: 1 }, intro(1)),
    pad(98, 'INTRO 3', 'e', { type: 'intro', index: 2 }, intro(2)),
    pad(99, 'SYNC ST', 'y', { type: 'toggleSyncStart' }, t.syncStart ? look(C_SYNC, 'bright', 'pulse') : look(C_SYNC, 'dim')),
    pad(100, 'ENDING 1', 'i', { type: 'ending', index: 0 }, sec(ENDINGS[0], C_ENDING)),
    pad(101, 'ENDING 2', 'o', { type: 'ending', index: 1 }, sec(ENDINGS[1], C_ENDING)),
    pad(102, 'ENDING 3', 'p', { type: 'ending', index: 2 }, sec(ENDINGS[2], C_ENDING)),
    pad(103, 'AUTOFILL', 'u', { type: 'toggleAutoFill' }, toggle(t.autoFill, C_FILL)),
    pad(112, 'MAIN A', '1', { type: 'main', index: 0 }, main(0)),
    pad(113, 'MAIN B', '2', { type: 'main', index: 1 }, main(1)),
    pad(114, 'MAIN C', '3', { type: 'main', index: 2 }, main(2)),
    pad(115, 'MAIN D', '4', { type: 'main', index: 3 }, main(3)),
    pad(116, 'BREAK', 'g', { type: 'break' }, sec(BREAK, C_BREAK)),
    pad(117, 'TAP', 't', { type: 'tapTempo' }, toggle(t.running && t.beat === 1, C_TAP)),
    pad(118, 'SYNC STP', 'j', { type: 'toggleSyncStop' }, toggle(t.syncStop, C_STOPSYNC)),
    pad(119, t.running ? 'START' : 'STOP', 'spc', { type: 'startStop' }, look(t.running ? C_RUN : C_IDLE, 'bright')),
  ]
}

/** A page 2/3 pad in the page's colour: bright when on, dim when off, dark when unavailable. */
function pagePad(page: PadPage, note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean): Pad {
  return pad(note, label, key, action, look(PAGE_RGB[page], !available ? 'off' : on ? 'bright' : 'dim'))
}

const FINGERING_LABELS = ['SINGLE', 'FINGERED', 'ON BASS', 'MULTI', 'AI FING', 'FULL KBD', 'AI FULL']

function chordPads(s: AppState): Pad[] {
  const p = (note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean) =>
    pagePad('chordSetup', note, label, key, action, available, on)
  const c = s.chord
  return [
    ...FINGERINGS.map((f, i) => p(96 + i, FINGERING_LABELS[i], 'pad', { type: 'setFingering', fingering: f.id }, true, c.fingering === f.id)),
    p(103, 'UPPER', 'd', { type: 'toggleUpper' }, true, c.upper),
    p(112, 'MAN BASS', 'D', { type: 'toggleManualBass' }, c.upper, c.manualBass),
    p(113, 'STOP ACMP', 'h', { type: 'toggleStopAcmp' }, true, s.transport.stopAcmp),
    p(114, 'SPLIT -', '[', { type: 'moveSplit', delta: -1 }, true, false),
    p(115, 'SPLIT +', ']', { type: 'moveSplit', delta: 1 }, true, false),
    p(116, 'KBD TR -', ';', { type: 'stepTranspose', keyboard: -1, master: 0 }, true, c.transposeKeyboard < 0),
    p(117, 'KBD TR +', "'", { type: 'stepTranspose', keyboard: 1, master: 0 }, true, c.transposeKeyboard > 0),
    p(118, 'TR RESET', '/', { type: 'resetTranspose' }, true, c.transposeKeyboard !== 0 || c.transposeMaster !== 0),
    p(119, '', '', null, false, false),
  ]
}

function otsPads(s: AppState): Pad[] {
  const p = (note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean) =>
    pagePad('otsParts', note, label, key, action, available, on)
  const n = s.ots.settings.length
  return [
    ...[0, 1, 2, 3].map((i) => p(96 + i, `OTS ${i + 1}`, `⇧${i + 1}`, { type: 'recallOts', index: i }, i < n, s.ots.applied === i + 1)),
    p(100, 'OTS LINK', 'F10', { type: 'toggleOtsLink' }, true, s.ots.link),
    p(101, '', '', null, false, false),
    p(102, 'VOICE -', '9', { type: 'stepVoice', delta: -1 }, true, false),
    p(103, 'VOICE +', '0', { type: 'stepVoice', delta: 1 }, true, false),
    ...['RIGHT 1', 'RIGHT 2', 'RIGHT 3', 'LEFT'].map((l, i) => p(112 + i, l, ['5', '6', '7', '8/l'][i], { type: 'togglePart', part: i }, true, s.keyboardParts[i].on)),
    ...['EDIT R1', 'EDIT R2', 'EDIT R3', 'EDIT L'].map((l, i) => p(116 + i, l, `F${i + 1}`, { type: 'selectPart', part: i }, true, s.keyboardParts[i].selected)),
  ]
}

/** The 16 pads of a page, top row then bottom row. */
export function padsFor(s: AppState, page: PadPage): Pad[] {
  if (page === 'chordSetup') return chordPads(s)
  if (page === 'otsParts') return otsPads(s)
  return sectionPads(s)
}
