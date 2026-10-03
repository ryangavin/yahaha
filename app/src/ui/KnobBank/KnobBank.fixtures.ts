import type { KnobItem } from './types'

/** The board's knob page 1 of 6, Style (Stage-Dark.dc.html). */
export const styleKnobs: KnobItem[] = [
  { label: 'Dynamics', code: 'DynCtrl', value: '127', fraction: 1 },
  { label: 'Retrig rate', code: 'RtgRate', value: '1/8', fraction: 0.4 },
  { label: 'Retrigger', code: 'RtgOnOff', value: 'Off', fraction: 0 },
  { label: 'Mute A', code: 'StyMuteA', value: 'Off', fraction: 0 },
  { label: 'Mute B', code: 'StyMuteB', value: 'Off', fraction: 0 },
  { label: 'Swing', code: 'Swing', value: '0', unit: '%', fraction: 0 },
  { label: '---', code: '', value: '', fraction: 0, unused: true },
  { label: 'Tempo', code: 'Tempo', value: '104', fraction: 0.27 },
]

/** The board's knob page block and counter. */
export const styleKnobPage = { label: 'Style', count: '1/6' }
