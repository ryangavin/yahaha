// The mock's pad lights: a port of `looks()` in `src/launchkey.rs` (same colours, labels,
// keys and lamp rules), producing the `Pad`s the engine puts in `AppState`. Only the mock
// uses this; with the real engine the pads come in the state.

import { QUICK_BANKS, QUICK_LOADED, QUICK_STORED } from './quick-racks'
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
export const PAGE_RGB: Record<PadPage, Rgb> = { sections: C_TAP, racks: [127, 60, 0], chord: [0, 100, 127], multiPads: [127, 127, 0], setup: [127, 0, 70] }

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
    // Where a fill queued or playing lands, when that's another Main (#282).
    if (t.landing === id) return look(C_MAIN, 'bright', 'pulse')
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

/** A pad in the page's colour: bright when on, dim when off, dark when unavailable. */
function pagePad(page: PadPage, note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean): Pad {
  return pad(note, label, key, action, look(PAGE_RGB[page], !available ? 'off' : on ? 'bright' : 'dim'))
}

const FINGERING_LABELS = ['SINGLE', 'FINGERED', 'ON BASS', 'MULTI', 'AI FING', 'FULL KBD', 'AI FULL']

/** A pad that does nothing on its page: dark. */
const darkPad = (page: PadPage, note: number) => pagePad(page, note, '', '', null, false, false)

/** Page Chord (launchkey/pages/chord.rs): the mid-song chord switches on the bottom row;
 * the top row dark. */
function chordPads(s: AppState): Pad[] {
  const p = (note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean) =>
    pagePad('chord', note, label, key, action, available, on)
  const c = s.chord
  return [
    ...Array.from({ length: 8 }, (_, i) => darkPad('chord', 96 + i)),
    p(112, 'MAN BASS', 'D', { type: 'toggleManualBass' }, c.upper, c.manualBass),
    p(113, 'STOP ACMP', 'h', { type: 'toggleStopAcmp' }, true, s.transport.stopAcmp),
    p(114, 'SPLIT -', '[', { type: 'moveSplit', delta: -1 }, true, false),
    p(115, 'SPLIT +', ']', { type: 'moveSplit', delta: 1 }, true, false),
    p(116, 'KBD TR -', ';', { type: 'stepTranspose', keyboard: -1, master: 0 }, true, c.transposeKeyboard < 0),
    p(117, 'KBD TR +', "'", { type: 'stepTranspose', keyboard: 1, master: 0 }, true, c.transposeKeyboard > 0),
    p(118, 'TR RESET', '/', { type: 'resetTranspose' }, true, c.transposeKeyboard !== 0 || c.transposeMaster !== 0),
    p(119, 'RETRIG', 'R', { type: 'toggleRetrigger' }, true, s.transport.retrigger),
  ]
}

/** Page Setup (launchkey/pages/setup.rs): the set-and-forget switches, each saved in
 * settings. Fingering types 1–7 and Upper on the top row; OTS Link and the Stop ACMP mode
 * on the bottom row, the rest dark. */
function setupPads(s: AppState): Pad[] {
  const p = (note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean) =>
    pagePad('setup', note, label, key, action, available, on)
  const c = s.chord
  const mode = s.transport.stopAcmpMode
  return [
    ...FINGERINGS.map((f, i) => p(96 + i, FINGERING_LABELS[i], 'pad', { type: 'setFingering', fingering: f.id }, true, c.fingering === f.id)),
    p(103, 'UPPER', 'd', { type: 'toggleUpper' }, true, c.upper),
    p(112, 'OTS LINK', 'F10', { type: 'toggleOtsLink' }, true, s.ots.link),
    p(113, 'ACMP STYLE', 'pad', { type: 'setStopAcmp', mode: 'style' }, true, mode === 'style'),
    p(114, 'ACMP FIXED', 'pad', { type: 'setStopAcmp', mode: 'fixed' }, true, mode === 'fixed'),
    ...[115, 116, 117, 118, 119].map((n) => darkPad('setup', n)),
  ]
}

const QUICK_KEYS = ['⇧Q', '⇧W', '⇧E', '⇧R', '⇧T', '⇧Y', '⇧U', '⇧I']

/** Page Racks (`racks_looks`, launchkey/pages/racks.rs): Quick Racks 1–8 of the bank on
 * view on the top row; OTS 1–4, Bank −/+ and Store on the bottom row, the last pad dark.
 * Hold Sound shows it from any page. */
function racksPads(s: AppState): Pad[] {
  const q = s.quickRacks
  const p = (note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean) =>
    pagePad('racks', note, label, key, action, available, on)
  const n = s.ots.settings.length
  const button = (i: number): Pad => {
    const b = q.buttons[i]
    const stored = !!b?.rack
    const l = q.store
      ? look(QUICK_LOADED, 'bright', 'flash')
      : stored && b.loaded
        ? look(QUICK_LOADED, 'bright')
        : look(QUICK_STORED, stored ? 'bright' : 'off')
    return pad(96 + i, `QUICK ${i + 1}`, QUICK_KEYS[i], { type: 'pressQuickRack', slot: i }, l)
  }
  return [
    ...Array.from({ length: 8 }, (_, i) => button(i)),
    ...[0, 1, 2, 3].map((i) => p(112 + i, `OTS ${i + 1}`, `⇧${i + 1}`, { type: 'recallOts', index: i }, i < n, s.ots.applied === i + 1)),
    p(116, 'BANK -', '⇧O', { type: 'stepQuickRackBank', delta: -1 }, q.bank > 0, false),
    p(117, 'BANK +', '⇧P', { type: 'stepQuickRackBank', delta: 1 }, q.bank < QUICK_BANKS - 1, false),
    q.store
      ? pad(118, 'STORE', 'F5', { type: 'toggleQuickRackStore' }, look(QUICK_LOADED, 'bright', 'flash'))
      : p(118, 'STORE', 'F5', { type: 'toggleQuickRackStore' }, true, false),
    darkPad('racks', 119),
  ]
}

/** Multi Pad lamps (OM p.75): blue = data, red = playing, amber = waiting for the bar. */
const C_PAD_READY: Rgb = [0, 40, 127]
const C_PAD_PLAYING: Rgb = [127, 0, 0]
const C_PAD_QUEUED: Rgb = [127, 60, 0]

function multiPadPads(s: AppState): Pad[] {
  const lamps = s.multiPad.pads.map((x) => x.lamp)
  const has = (i: number) => lamps[i] !== 'empty'
  const sounding = (i: number) => lamps[i] === 'playing' || lamps[i] === 'queued'
  const p = (note: number, label: string, key: string, action: AppCmd | null, available: boolean, on: boolean) =>
    pagePad('multiPads', note, label, key, action, available, on)
  const lamp = (i: number): Look => {
    switch (lamps[i]) {
      case 'ready': return look(C_PAD_READY, 'bright')
      case 'armed': return look(C_PAD_PLAYING, 'bright', 'flash')
      case 'queued': return look(C_PAD_QUEUED, 'bright', 'flash')
      case 'playing': return look(C_PAD_PLAYING, 'bright')
      default: return look(C_PAD_READY, 'off')
    }
  }
  const busy = lamps.some((l, i) => sounding(i) || l === 'armed')
  return [
    ...[0, 1, 2, 3].map((i) => pad(96 + i, `PAD ${i + 1}`, ['Z', 'X', 'C', 'V'][i], { type: 'triggerMultiPad', pad: i }, lamp(i))),
    p(100, 'STOP', 'B', { type: 'stopAllMultiPads' }, lamps.some((_, i) => has(i)), busy),
    p(101, '', '', null, false, false),
    p(102, '', '', null, false, false),
    p(103, '', '', null, false, false),
    ...[0, 1, 2, 3].map((i) => {
      const armed = lamps[i] === 'armed'
      const x = p(112 + i, `SELECT ${i + 1}`, 'pad', { type: 'armMultiPad', pad: i }, has(i), armed)
      return armed ? { ...x, anim: 'flash' as const } : x
    }),
    ...[0, 1, 2, 3].map((i) => p(116 + i, `STOP ${i + 1}`, 'pad', { type: 'stopMultiPad', pad: i }, has(i), sounding(i))),
  ]
}

/** The 16 pads of a page, top row then bottom row. */
export function padsFor(s: AppState, page: PadPage): Pad[] {
  if (page === 'multiPads') return multiPadPads(s)
  if (page === 'racks') return racksPads(s)
  if (page === 'chord') return chordPads(s)
  if (page === 'setup') return setupPads(s)
  return sectionPads(s)
}
