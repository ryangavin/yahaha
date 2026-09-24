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
const REGIST: TipKey[] = ['regist.1', 'regist.2', 'regist.3', 'regist.4', 'regist.5', 'regist.6', 'regist.7', 'regist.8', 'regist.9', 'regist.10']
const PAGE: Record<string, TipKey> = { sections: 'padpage.sections', chordSetup: 'padpage.chord_setup', otsParts: 'padpage.ots_parts', registration: 'padpage.registration' }

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
    case 'setStopAcmp': return cmd.mode === 'off' ? 'settings.stop_acmp_off' : cmd.mode === 'style' ? 'settings.stop_acmp_style' : 'settings.stop_acmp_fixed'
    case 'fillUp': return 'transport.fill_up'
    case 'fillDown': return 'transport.fill_down'
    case 'fillSelf': return 'transport.fill_self'
    case 'fillBreak': return 'transport.fill_break'
    case 'setHalfBarFill':
    case 'toggleHalfBarFill': return 'transport.half_bar_fill'
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
    case 'setPadPage': return PAGE[cmd.page]
    case 'cyclePadPage': return cmd.delta < 0 ? 'padpage.prev' : 'padpage.next'
    case 'setMasterVolume': return 'mixer.master'
    case 'recallOts': return OTS[cmd.index]
    case 'setOtsLink':
    case 'toggleOtsLink': return 'ots.link'
    case 'setOtsLinkTiming': return 'settings.ots_link_timing'
    case 'setTempoChange':
    case 'toggleStyleTempoLock':
    case 'toggleStyleTempoHold': return 'settings.tempo_change'
    case 'setPartsChange': return 'settings.parts_change'
    case 'setSectionSet': return 'settings.section_set'
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
    case 'setAudioBuffer': return 'audio.buffer'
    case 'rescanLibrary': return 'settings.rescan'
    case 'importCharts': return 'chart.import_link'
    case 'importChartFile': return 'chart.import_file'
    case 'selectChart': return 'chart.song'
    case 'stepChart': return cmd.delta < 0 ? 'chart.prev' : 'chart.next'
    case 'removeChartPlaylist': return 'chart.remove_playlist'
    case 'setChartMode':
    case 'toggleChartMode': return 'chart.mode'
    case 'setChartChoruses': return 'chart.choruses_up'
    case 'setChartLoop': return 'chart.loop'
    case 'setChartIntro': return 'chart.intro'
    case 'setChartEnding': return 'chart.ending'
    case 'setChartAutoStyle': return 'chart.auto_style'
    case 'toggleFade': return 'transport.fade'
    case 'sectionReset': return 'transport.section_reset'
    case 'toggleRetrigger': return 'transport.retrigger'
    case 'stepRetriggerRate': return cmd.delta < 0 ? 'transport.retrigger_longer' : 'transport.retrigger_shorter'
    case 'setRetriggerRate': return 'settings.retrigger_rate'
    case 'setMainTiming': return 'settings.section_timing'
    case 'setIntroEndingTiming': return 'settings.intro_ending_timing'
    case 'setSyncStopWindow': return 'settings.synchro_stop_window'
    case 'setFadeInTime': return 'settings.fade_in'
    case 'setFadeOutTime': return 'settings.fade_out'
    case 'setFadeHoldTime': return 'settings.fade_hold'
    case 'setSectionReset': return 'settings.section_reset'
    // Registration Memory
    case 'pressRegist':
    case 'recallRegist': return REGIST[cmd.index]
    case 'memorizeRegist':
    case 'toggleRegistMemory': return 'regist.memory'
    case 'setMemorizeGroup': return 'regist.memorize_group'
    case 'clearRegist': return 'regist.clear'
    case 'renameRegist': return 'regist.rename'
    case 'stepRegistBank': return cmd.delta < 0 ? 'regist.bank_prev' : 'regist.bank_next'
    case 'selectRegistBank': return 'regist.bank'
    case 'newRegistBank': return 'regist.new_bank'
    case 'saveRegistBank': return 'regist.save_bank'
    case 'setFreeze':
    case 'toggleFreeze': return 'regist.freeze'
    case 'setFreezeGroup': return 'regist.freeze_group'
    case 'setRegistSequence': return 'regist.sequence_steps'
    case 'setRegistSequenceOn':
    case 'toggleRegistSequence': return 'regist.sequence_on'
    case 'stepRegistSequence': return cmd.delta < 0 ? 'regist.seq_prev' : 'regist.seq_next'
    // Playlist
    case 'newPlaylist': return 'playlist.new'
    case 'loadPlaylist': return 'playlist.file'
    case 'savePlaylist': return 'playlist.save'
    case 'addPlaylistRecord':
    case 'addCurrentBank': return 'playlist.add_bank'
    case 'addCurrentStyle': return 'playlist.add_style'
    case 'appendPlaylist': return 'playlist.append'
    case 'setPlaylistRecord': return 'playlist.edit'
    case 'movePlaylistRecord': return cmd.delta < 0 ? 'playlist.up' : 'playlist.down'
    case 'deletePlaylistRecord': return 'playlist.delete'
    case 'setPlaylistSort': return 'playlist.sort'
    case 'loadPlaylistRecord': return 'playlist.record'
    case 'stepPlaylist': return cmd.delta < 0 ? 'playlist.prev' : 'playlist.next'
    case 'setTempo': return 'display.tempo'
    case 'setStyleSolo':
    case 'setPartSolo': return 'mixer.solo'
    case 'styleTrackMute': return 'mixer.track_mute'
    case 'looperRec': return 'looper.rec'
    case 'looperOnOff': return 'looper.on_off'
    case 'selectLooperMemory': return 'looper.memory'
    case 'storeLooperMemory': return 'looper.store'
    case 'clearLooperMemory': return 'looper.clear'
    case 'newLooperBank': return 'looper.new_bank'
    case 'toggleMetronome':
    case 'setMetronome': return 'metronome.on'
    case 'setMetronomeVolume': return 'metronome.volume'
    case 'setMetronomeBell': return 'metronome.bell'
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
    case 'setPartPlugin':
    case 'clearPartPlugin':
    case 'savePartPluginState': return 'part.plugin'
    case 'rescanPlugins': return 'part.plugin_rescan'
    case 'toggleHarmonyArp':
    case 'setHarmonyArpOn': return 'harmony.switch'
    case 'setHarmonyType': return 'harmony.type'
    case 'setArpPattern': return 'harmony.pattern'
    case 'stepHarmonyArpType': return 'harmony.next_type'
    case 'setHarmonyVolume': return 'harmony.volume'
    case 'setHarmonySpeed': return 'harmony.speed'
    case 'setHarmonyAssign': return 'harmony.assign'
    case 'setChordNoteOnly': return 'harmony.chord_note_only'
    case 'setTouchLimit': return 'harmony.touch_limit'
    case 'setArpQuantize': return 'harmony.arp_quantize'
    case 'setArpHold':
    case 'toggleArpHold':
    case 'setArpPedalHold':
    case 'toggleArpPedalHold': return 'harmony.arp_hold'
    case 'setArpVelocity': return 'harmony.arp_velocity'
    case 'setArpKeepKeyOn': return 'harmony.arp_keep_key_on'
    case 'createPatch':
    case 'addPresetAsPatch': return 'sound.preset_add'
    case 'savePartAsPatch': return 'sound.save_part'
    case 'updatePatch': return 'sound.name'
    case 'deletePatch': return 'sound.delete'
    case 'duplicatePatch': return 'sound.duplicate'
    case 'movePatch': return 'sound.move_up'
    case 'setPatchFavourite': return 'sound.favourite'
    case 'auditionPatch':
    case 'auditionPreset': return 'sound.audition'
    case 'stopPatchAudition': return 'sound.audition_stop'
    case 'setFamilyRule': return 'sound.family'
    case 'setProgramOverride': return 'sound.override_patch'
    case 'setDrumRule': return 'sound.drums'
    case 'clearStyleMap': return 'sound.clear_style_map'
    case 'setPartPatch': return 'part.library'
    case 'setPortSendsMapped': return 'sound.port_mapped'
    case 'browseSoundFont': return 'sound.soundfont'
    case 'importSoundLibrary': return 'sound.import'
    case 'exportSoundLibrary': return 'sound.export'
  }
}
