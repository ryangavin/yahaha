// The app's keyboard shortcuts: the terminal UI's keys (README "Terminal keys"), so your
// fingers don't have to relearn anything. Bindings are keyed in README notation, and a
// test checks them against the tooltip catalog both ways.

import type { AppCmd } from './api/types'

export type AppAction = 'browser' | 'help' | 'escape'
export type Binding = { cmd: AppCmd } | { app: AppAction }

const c = (cmd: AppCmd): Binding => ({ cmd })

export const BINDINGS: Record<string, Binding> = {
  space: c({ type: 'startStop' }),
  '1': c({ type: 'main', index: 0 }),
  '2': c({ type: 'main', index: 1 }),
  '3': c({ type: 'main', index: 2 }),
  '4': c({ type: 'main', index: 3 }),
  q: c({ type: 'intro', index: 0 }),
  w: c({ type: 'intro', index: 1 }),
  e: c({ type: 'intro', index: 2 }),
  i: c({ type: 'ending', index: 0 }),
  o: c({ type: 'ending', index: 1 }),
  p: c({ type: 'ending', index: 2 }),
  g: c({ type: 'break' }),
  t: c({ type: 'tapTempo' }),
  '-': c({ type: 'tempoDown' }),
  '=': c({ type: 'tempoUp' }),
  y: c({ type: 'toggleSyncStart' }),
  u: c({ type: 'toggleAutoFill' }),
  j: c({ type: 'toggleSyncStop' }),
  h: c({ type: 'toggleStopAcmp' }),
  f: c({ type: 'nextFingering' }),
  d: c({ type: 'toggleUpper' }),
  D: c({ type: 'toggleManualBass' }),
  '[': c({ type: 'moveSplit', delta: -1 }),
  ']': c({ type: 'moveSplit', delta: 1 }),
  ';': c({ type: 'stepTranspose', keyboard: -1, master: 0 }),
  "'": c({ type: 'stepTranspose', keyboard: 1, master: 0 }),
  ':': c({ type: 'stepTranspose', keyboard: 0, master: -1 }),
  '"': c({ type: 'stepTranspose', keyboard: 0, master: 1 }),
  '/': c({ type: 'resetTranspose' }),
  ...Object.fromEntries([...'zxcvbnm,'].map((k, i) => [k, c({ type: 'toggleStylePart', part: i })])),
  ...Object.fromEntries([0, 1, 2, 3].map((i) => [`shift+${i + 1}`, c({ type: 'recallOts', index: i })])),
  F10: c({ type: 'toggleOtsLink' }),
  '5': c({ type: 'togglePart', part: 0 }),
  '6': c({ type: 'togglePart', part: 1 }),
  '7': c({ type: 'togglePart', part: 2 }),
  '8': c({ type: 'togglePart', part: 3 }),
  l: c({ type: 'togglePart', part: 3 }),
  ...Object.fromEntries([0, 1, 2, 3].map((i) => [`F${i + 1}`, c({ type: 'selectPart', part: i })])),
  '9': c({ type: 'stepVoice', delta: -1 }),
  '0': c({ type: 'stepVoice', delta: 1 }),
  F9: c({ type: 'toggleFaderPage' }),
  J: c({ type: 'toggleHarmonyArp' }),
  L: c({ type: 'stepHarmonyArpType', delta: 1 }),
  '*': c({ type: 'toggleArpHold' }),
  // Registration Memory: Shift + the top letter row = buttons 1–10.
  ...Object.fromEntries([...'QWERTYUIOP'].map((k, i) => [k, c({ type: 'pressRegist', index: i })])),
  F5: c({ type: 'toggleRegistMemory' }),
  F6: c({ type: 'toggleFreeze' }),
  F7: c({ type: 'stepRegistSequence', delta: -1 }),
  F8: c({ type: 'stepRegistSequence', delta: 1 }),
  F11: c({ type: 'stepRegistBank', delta: -1 }),
  F12: c({ type: 'stepRegistBank', delta: 1 }),
  '<': c({ type: 'stepPlaylist', delta: -1 }),
  '>': c({ type: 'stepPlaylist', delta: 1 }),
  '←': c({ type: 'stepStyle', delta: -1 }),
  '→': c({ type: 'stepStyle', delta: 1 }),
  PgDn: c({ type: 'cyclePadPage', delta: 1 }),
  PgUp: c({ type: 'cyclePadPage', delta: -1 }),
  ...Object.fromEntries([...'ZXCV'].map((k, i) => [k, c({ type: 'triggerMultiPad', pad: i })])),
  B: c({ type: 'stopAllMultiPads' }),
  a: c({ type: 'nextAudioOutput' }),
  k: c({ type: 'toggleSynthMute' }),
  '\\': c({ type: 'panic' }),
  r: c({ type: 'looperRec' }),
  '^': c({ type: 'looperOnOff' }),
  '.': c({ type: 'toggleMetronome' }),
  enter: { app: 'browser' },
  esc: { app: 'escape' },
  '?': { app: 'help' },
}

const NAMED: Record<string, string> = {
  ' ': 'space', Enter: 'enter', Escape: 'esc', ArrowLeft: '←', ArrowRight: '→', PageDown: 'PgDn', PageUp: 'PgUp',
}

/** A key event in README notation, or null for keys we don't bind (with Ctrl/Alt/Cmd, say). */
export function keyName(e: Pick<KeyboardEvent, 'key' | 'code' | 'shiftKey' | 'ctrlKey' | 'altKey' | 'metaKey'>): string | null {
  if (e.ctrlKey || e.altKey || e.metaKey) return null
  if (NAMED[e.key]) return NAMED[e.key]
  // Shift+digit by physical key, so OTS works on any layout (`!` on US, `+` on Swiss …).
  const digit = /^Digit([1-4])$/.exec(e.code)
  if (e.shiftKey && digit) return `shift+${digit[1]}`
  if (/^F\d+$/.test(e.key)) return e.key
  if (e.key === '+') return '='
  return e.key.length === 1 ? e.key : null
}

export function binding(e: Parameters<typeof keyName>[0]): Binding | null {
  const k = keyName(e)
  return k ? (BINDINGS[k] ?? null) : null
}
