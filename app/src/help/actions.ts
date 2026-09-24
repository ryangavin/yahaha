// Which catalog entry explains a command. Pads come from the engine with their `action`,
// so a pad's tooltip is the tooltip of what it does.

import type { AppCmd, Fingering } from '../lib/api/types'
import type { TipKey } from './tooltips'

const FINGERING: Record<Fingering, TipKey> = {
  singleFinger: 'fingering.single_finger',
  fingered: 'fingering.fingered',
  fingeredOnBass: 'fingering.fingered_on_bass',
  multiFinger: 'fingering.multi_finger',
  aiFingered: 'fingering.ai_fingered',
  fullKeyboard: 'fingering.full_keyboard',
  aiFullKeyboard: 'fingering.ai_full_keyboard',
}
const INTRO: TipKey[] = ['section.intro1', 'section.intro2', 'section.intro3']
const MAIN: TipKey[] = ['section.main_a', 'section.main_b', 'section.main_c', 'section.main_d']
const ENDING: TipKey[] = ['section.ending1', 'section.ending2', 'section.ending3']
const PART_ON: TipKey[] = ['part.right1.on', 'part.right2.on', 'part.right3.on', 'part.left.on']
const PART_SELECT: TipKey[] = ['part.right1.select', 'part.right2.select', 'part.right3.select', 'part.left.select']
const PART_VOLUME: TipKey[] = ['mixer.panel.right1', 'mixer.panel.right2', 'mixer.panel.right3', 'mixer.panel.left']
const OTS: TipKey[] = ['ots.1', 'ots.2', 'ots.3', 'ots.4']

/** The catalog entry for a command; an unused pad (null) has its own. */
export function tipFor(cmd: AppCmd | null): TipKey {
  if (!cmd) return 'launchkey.unused'
  switch (cmd.type) {
    case 'intro': return INTRO[cmd.index]
    case 'main': return MAIN[cmd.index]
    case 'break': return 'section.break'
    case 'ending': return ENDING[cmd.index]
    case 'startStop': return 'transport.start_stop'
    case 'stop': return 'transport.stop'
    case 'toggleSyncStart': return 'transport.sync_start'
    case 'toggleSyncStop': return 'transport.sync_stop'
    case 'toggleAutoFill': return 'transport.auto_fill'
    case 'toggleStopAcmp': return 'transport.stop_acmp'
    case 'tapTempo': return 'tempo.tap'
    case 'tempoUp': return 'tempo.up'
    case 'tempoDown': return 'tempo.down'
    case 'toggleStylePart': return 'mixer.style.mute'
    case 'setStylePartVolume': return 'mixer.style.volume'
    case 'setFingering': return FINGERING[cmd.fingering]
    case 'nextFingering': return 'fingering.next'
    case 'setUpper':
    case 'toggleUpper': return 'detection.upper'
    case 'setManualBass':
    case 'toggleManualBass': return 'detection.manual_bass'
    case 'setSplit': return 'split.display'
    case 'moveSplit': return cmd.delta < 0 ? 'split.down' : 'split.up'
    case 'setTranspose': return 'transpose.display'
    case 'stepTranspose':
      if (cmd.keyboard) return cmd.keyboard < 0 ? 'transpose.keyboard_down' : 'transpose.keyboard_up'
      return cmd.master < 0 ? 'transpose.master_down' : 'transpose.master_up'
    case 'resetTranspose': return 'transpose.reset'
    case 'setChordSettle': return 'settings.chord_settle'
    case 'setPartOn':
    case 'togglePart': return PART_ON[cmd.part]
    case 'selectPart': return PART_SELECT[cmd.part]
    case 'setPartVoice': return 'part.voice_up'
    case 'stepVoice': return cmd.delta < 0 ? 'part.voice_down' : 'part.voice_up'
    case 'setPartVolume': return PART_VOLUME[cmd.part]
    case 'setPartOctave': return 'part.octave_up'
    case 'setFaderPage':
    case 'toggleFaderPage': return 'mixer.page'
    case 'setPadPage': return cmd.page === 'sections' ? 'padpage.sections' : cmd.page === 'chordSetup' ? 'padpage.chord_setup' : 'padpage.ots_parts'
    case 'cyclePadPage': return cmd.delta < 0 ? 'padpage.prev' : 'padpage.next'
    case 'setMasterVolume': return 'mixer.master'
    case 'recallOts': return OTS[cmd.index]
    case 'setOtsLink':
    case 'toggleOtsLink': return 'ots.link'
    case 'loadStyle':
    case 'loadStylePath': return 'browser.row'
    case 'stepStyle': return cmd.delta < 0 ? 'style.prev' : 'style.next'
    case 'setSynthMuted':
    case 'toggleSynthMute': return 'audio.synth_mute'
    case 'setAudioOutput':
    case 'nextAudioOutput': return 'audio.output'
    case 'panic': return 'transport.panic'
    case 'clearMessage': return 'display.status'
    case 'auditionStyle': return 'browser.preview'
    case 'stopAudition': return 'browser.preview_stop'
    case 'queueStyle': return 'browser.queue'
    case 'setSoundFont': return 'audio.soundfont'
    case 'setMidiInputs': return cmd.all ? 'midi.merge_all' : 'midi.input'
    case 'setPaletteLeds': return 'midi.palette_leds'
    case 'rescanLibrary': return 'settings.rescan'
    case 'loadMultiPad':
    case 'loadMultiPadPath': return 'multipad.bank'
    case 'clearMultiPad': return 'multipad.clear'
    case 'triggerMultiPad': return 'multipad.pad'
    case 'stopMultiPad': return 'multipad.stop'
    case 'stopAllMultiPads': return 'multipad.stop_all'
    case 'armMultiPad': return 'multipad.arm'
    case 'setMultiPadRepeat': return 'multipad.repeat'
    case 'setMultiPadChordMatch': return 'multipad.chord_match'
    case 'setMultiPadSynchroStop': return 'multipad.synchro_style_stop'
    // Fill Up/Down/Self are pedal functions (no pad has them).
    case 'fill':
    case 'setPedal': return 'pedal.function'
    case 'learnPedal': return 'pedal.learn'
    case 'triggerFunction': return 'pedal.try'
    case 'setPartControllers': return 'pedal.part_sustain'
    case 'setBendRange': return 'pedal.bend_up'
  }
}
