// @vitest-environment node
import { describe, expect, it } from 'vitest'
import { MockSession, mockOtsEq, mockOtsInsert, noteName, transposeChord } from './mock'
import { FLAT_EQ } from './types'

/** Milliseconds per bar at the mock's current tempo. */
const bar = (m: MockSession) => (60000 / m.state.transport.tempo) * m.state.transport.beatsPerBar

describe('mock session', () => {
  it('meters: each track\'s CPU (#340), a plugin\'s on its own channel; plugin instances counted (#407)', async () => {
    const m = new MockSession({ manual: true })
    const cpu = async () => new Map((await m.meters()).channels.map((c) => [c.channel, c.cpu]))
    let c = await cpu()
    expect(c.size).toBe(16)
    expect(c.get(1)).toBeGreaterThan(0) // Right 1, on, SoundFont
    expect([...c.entries()].filter(([ch]) => ch >= 9).every(([, x]) => x === 0)).toBe(true) // the band stopped
    // AUSampler (the heavy one) on Right 2 (channel 3).
    m.send({ type: 'setPartPlugin', part: 1, id: 'aumu samp appl', state: null })
    expect(m.state.plugins.instances).toBe(0) // still loading
    m.advance(1000)
    expect(m.state.plugins.instances).toBe(1)
    m.send({ type: 'startStop' })
    c = await cpu()
    expect(c.get(3)).toBeGreaterThan(0.25)
    expect(c.get(9)).toBeGreaterThan(0) // Rhythm 1 plays
    expect(c.get(5)).toBe(0) // no Multi Pad
    const meters = await m.meters()
    expect(meters.channels.find((x) => x.channel === 3)!.cpuPeak).toBeGreaterThan(c.get(3)!)
    expect(meters.cpu.total).toBeCloseTo([...c.values()].reduce((a, b) => a + b, 0))
    m.send({ type: 'clearPartPlugin', part: 1 })
    expect(m.state.plugins.instances).toBe(0)
  })

  it('starts stopped with Sync Start armed; Start/Stop starts it on Main A', () => {
    const m = new MockSession({ manual: true })
    expect(m.state.transport.running).toBe(false)
    expect(m.state.transport.syncStart).toBe(true)
    m.send({ type: 'startStop' })
    expect(m.state.transport.running).toBe(true)
    expect(m.state.transport.section).toBe('Main A')
  })

  it('the live rack: a new rack, modified by a mix, split or Harmony/Arp change but not by the band', () => {
    for (const cmd of [
      { type: 'setPartVolume', part: 0, volume: 12 },
      { type: 'setSplit', note: 48 },
      { type: 'setHarmonyArpOn', on: true },
      { type: 'setRackControl', control: 'knob', index: 6, target: { kind: 'splitPoint' } },
    ] as const) {
      const m = new MockSession({ manual: true })
      expect(m.state.liveRack).toMatchObject({ name: 'New rack', id: null, modified: false, prompt: null })
      expect(m.state.liveRack.controls.faders).toEqual([0, 1, 2, 3].map((part) => ({ kind: 'partLevel', part })))
      expect(m.state.liveRack.controls.knobs.slice(4)).toEqual([{ kind: 'harmonyVolume' }, { kind: 'metronomeVolume' }, { kind: 'none' }, { kind: 'tempo' }])
      m.send({ type: 'startStop' })
      m.advance(bar(m) * 2)
      expect(m.state.liveRack.modified).toBe(false)
      m.send(cmd)
      expect(m.state.liveRack.modified).toBe(true)
    }
  })

  it('rack commands: save as, the unsaved-changes guard, load, rename, duplicate, delete', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setPartVolume', part: 0, volume: 30 })
    m.send({ type: 'saveRackAs', name: 'Ballad' })
    const id = m.state.racks[0].id
    expect(m.state.racks.map((r) => r.name)).toEqual(['Ballad'])
    expect(m.state.liveRack).toMatchObject({ name: 'Ballad', id, modified: false, prompt: null })

    m.send({ type: 'setPartVolume', part: 0, volume: 99 })
    m.send({ type: 'newRack' })
    expect(m.state.liveRack.prompt).toEqual({ kind: 'unsavedChanges', then: { kind: 'new' } })
    expect(m.state.keyboardParts[0].volume).toBe(99)
    m.send({ type: 'dismissRackPrompt' })
    expect(m.state.liveRack.prompt).toBeNull()
    m.send({ type: 'loadRack', id, discard: true })
    expect(m.state.keyboardParts[0].volume).toBe(30)
    expect(m.state.liveRack.modified).toBe(false)

    m.send({ type: 'duplicateRack', id })
    m.send({ type: 'renameRack', id, name: 'Slow' })
    expect(m.state.racks.map((r) => r.name)).toEqual(['Ballad copy', 'Slow'])
    expect(m.state.liveRack.name).toBe('Slow')
    m.send({ type: 'deleteRack', id })
    expect(m.state.racks).toHaveLength(2)
    expect(m.state.message?.error).toBe(true)
    m.send({ type: 'deleteRack', id: m.state.racks[0].id })
    expect(m.state.racks.map((r) => r.name)).toEqual(['Slow'])
  })

  it('a queued Main takes over at the next bar', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.send({ type: 'main', index: 2 })
    expect(m.state.transport.queued).toBe('Main C')
    m.advance(bar(m) * 0.5)
    expect(m.state.transport.section).toBe('Main A')
    m.advance(bar(m) * 0.6)
    expect(m.state.transport.section).toBe('Main C')
    expect(m.state.transport.queued).toBe(null)
    expect(m.state.transport.bar).toBe(1)
  })

  // #111: with OTS Link on, a queued style's OTS comes as it takes over in a Main, and
  // waits for the Main after a Break. A queued style waits for an Ending.
  const otherStyle = (m: MockSession) => {
    const styles = (m as unknown as { styles: { id: number; ots: number; sections: string[]; error?: string }[] }).styles
    return styles.find((s) => s.id !== m.state.style.id && s.ots > 0 && !s.error && s.sections.includes('Fill In BA') && s.sections.includes('Ending A') && s.sections.includes('Main A'))!
  }

  it('an OTS recall sets the part EQ as the engine does (#247)', () => {
    const m = new MockSession({ manual: true })
    const mine = { ...FLAT_EQ, lowGain: -4 }
    for (let p = 0; p < 4; p++) m.send({ type: 'setPartEq', part: p, eq: mine })
    m.send({ type: 'recallOts', index: 0 })
    expect(m.state.keyboardParts[0].eq).toEqual(mockOtsEq(0, 0))
    m.state.ots.settings[0].parts.slice(1).forEach((o, j) => {
      expect(m.state.keyboardParts[j + 1].eq).toEqual(o.program !== null ? FLAT_EQ : mine)
    })
  })

  it('the insert slot commands set it, and an OTS recall sets it as the engine does', () => {
    const m = new MockSession({ manual: true })
    for (let p = 0; p < 4; p++) {
      m.send({ type: 'setKeyboardInsertEffect', part: p, effect: 'tremolo' })
      m.send({ type: 'setKeyboardInsertOn', part: p, on: true })
      m.send({ type: 'setKeyboardInsertAmount', part: p, amount: 200 })
    }
    const mine = { effect: 'tremolo', on: true, amount: 127 }
    expect(m.state.keyboardParts[3].insert).toEqual(mine)
    m.send({ type: 'recallOts', index: 0 })
    expect(m.state.keyboardParts[0].insert).toEqual(mockOtsInsert(0, 0))
    m.state.ots.settings[0].parts.slice(1).forEach((o, j) => {
      expect(m.state.keyboardParts[j + 1].insert).toEqual(o.program !== null ? { ...mine, on: false } : mine)
    })
  })

  it('a queued style recalls its OTS as it takes over in a Main', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setOtsLink', on: true })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 0.3)
    const s = otherStyle(m)
    m.send({ type: 'queueStyle', id: s.id })
    expect(m.state.style.id).not.toBe(s.id)
    m.state.ots.applied = 0
    m.advance(bar(m) * 0.8)
    expect(m.state.style.id).toBe(s.id)
    expect(m.state.ots.applied).toBe(1)
  })

  it('a queued style taking over in a Break recalls its OTS when the Main starts', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setOtsLink', on: true })
    m.send({ type: 'setOtsLinkTiming', timing: 'immediate' })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 0.3)
    m.send({ type: 'break' })
    m.advance(bar(m) * 0.8)
    expect(m.state.transport.section).toBe('Fill In BA')
    const s = otherStyle(m)
    m.send({ type: 'queueStyle', id: s.id })
    m.state.ots.applied = 0
    for (let i = 0; i < 40 && m.state.style.id !== s.id; i++) {
      m.advance(bar(m) * 0.05)
      if (m.state.style.id !== s.id) expect(m.state.ots.applied).toBe(0)
    }
    // It took over at the Break's end, where Main A starts: the OTS comes with the Main,
    // under Immediate too.
    expect(m.state.style.id).toBe(s.id)
    expect(m.state.transport.section).toBe('Main A')
    expect(m.state.ots.applied).toBe(1)
  })

  it('a queued style waits for an Ending, and loads at the stop', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 0.3)
    m.send({ type: 'ending', index: 0 })
    const s = otherStyle(m)
    m.send({ type: 'queueStyle', id: s.id })
    for (let i = 0; i < 8 && m.state.transport.running; i++) {
      m.advance(bar(m) * 0.5)
      if (m.state.transport.running) expect(m.state.style.id).not.toBe(s.id)
    }
    expect(m.state.transport.running).toBe(false)
    expect(m.state.style.id).toBe(s.id)
  })

  it('pressing the Main that plays queues its fill, which plays from the next beat', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 0.1)
    m.send({ type: 'main', index: 0 })
    expect(m.state.transport.queued).toBe('Fill In AA')
    m.advance((60000 / m.state.transport.tempo) * 1.0)
    expect(m.state.transport.section).toBe('Fill In AA')
  })

  it('with a fill queued, a later press moves only where it lands (#282)', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 0.1)
    m.send({ type: 'main', index: 0 })
    m.send({ type: 'main', index: 2 })
    expect(m.state.transport.queued).toBe('Fill In AA')
    expect(m.state.transport.landing).toBe('Main C')
    const lamp = m.state.transport.lamps.find((p) => p.note === 114)!
    expect([lamp.level, lamp.anim]).toEqual(['bright', 'pulse'])
  })

  it('Tap while the band plays resets the section by default (the Genos default)', () => {
    const m = new MockSession({ manual: true })
    expect(m.state.styleSettings.sectionReset).toBe(true)
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 1.3)
    expect(m.state.transport.bar).toBe(2)
    const tempo = m.state.transport.tempo
    m.send({ type: 'tapTempo' })
    m.advance(400)
    m.send({ type: 'tapTempo' })
    expect(m.state.transport.tempo).toBe(tempo)
    expect(m.state.transport.bar).toBe(1)
    expect(m.state.transport.running).toBe(true)
  })

  it('Tap while the band plays sets the tempo with Section Reset off; Style Section Reset is a function of its own (#128)', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setSectionReset', on: false })
    m.send({ type: 'startStop' })
    m.advance(bar(m) * 1.3)
    expect(m.state.transport.bar).toBe(2)
    m.send({ type: 'tapTempo' })
    m.advance(400)
    m.send({ type: 'tapTempo' })
    expect(m.state.transport.tempo).toBeCloseTo(150, 1)
    expect(m.state.transport.bar).toBe(2)
    m.advance(bar(m) * 0.1)
    expect(m.state.transport.beat).not.toBe(1)
    m.send({ type: 'triggerFunction', function: 'sectionReset' })
    expect(m.state.transport.beat).toBe(1)
    expect(m.state.transport.running).toBe(true)
  })

  it('a bar of taps while stopped starts the band a beat after the last tap (#195)', () => {
    const m = new MockSession({ manual: true })
    const beats = m.state.transport.beatsPerBar
    for (let i = 0; i < beats; i++) {
      if (i > 0) m.advance(500)
      m.send({ type: 'tapTempo' })
    }
    expect(m.state.transport.tempo).toBe(120)
    m.advance(480)
    expect(m.state.transport.running).toBe(false)
    m.advance(40)
    expect(m.state.transport.running).toBe(true)
    // STOP during the count-in calls it off.
    m.send({ type: 'startStop' })
    m.advance(20000)
    for (let i = 0; i < beats; i++) {
      if (i > 0) m.advance(500)
      m.send({ type: 'tapTempo' })
    }
    m.send({ type: 'stop' })
    m.advance(2000)
    expect(m.state.transport.running).toBe(false)
  })

  it('an armed Intro plays first, then the Main', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'intro', index: 0 })
    expect(m.state.transport.pendingIntro).toBe(0)
    m.send({ type: 'startStop' })
    expect(m.state.transport.section).toBe('Intro A')
    m.advance(bar(m) * 2.1)
    expect(m.state.transport.section).toBe('Main A')
  })

  it('an Ending stops the band', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'startStop' })
    m.send({ type: 'ending', index: 0 })
    m.advance(bar(m) * 3.2)
    expect(m.state.transport.running).toBe(false)
  })

  it('switching fader page makes every level on the new page wait for its fader', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'toggleFaderPage' })
    expect(m.state.mixer.styleParts.every((p) => p.waiting)).toBe(true)
    m.send({ type: 'setStylePartVolume', part: 0, volume: 90 })
    expect(m.state.mixer.styleParts[0].waiting).toBe(false)
  })

  it('Upper turns Manual Bass on: Left plays the bass and the style Bass is muted', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'toggleUpper' })
    expect(m.state.chord.manualBassActive).toBe(true)
    expect(m.state.keyboardParts[3].sounding).toBe(true)
    expect(m.state.mixer.styleParts[2].mutedByManualBass).toBe(true)
  })

  it('publishes a fresh snapshot with a new version', () => {
    const m = new MockSession({ manual: true })
    const seen: { version: number }[] = []
    m.subscribe((s) => seen.push(s))
    m.advance(16)
    expect(seen).toHaveLength(2)
    expect(seen[0]).not.toBe(seen[1])
    expect(seen[1].version).toBeGreaterThan(seen[0].version)
  })

  it('a plugin sound exports as an .aupreset; a second export needs overwrite (D5, #307)', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'exportSoundPreset', id: 'keys-au' })
    expect(m.state.message?.text).toContain('no settings yet')
    m.send({ type: 'exportSoundPreset', id: 'stage-grand' })
    expect(m.state.message?.text).toContain('not a plugin sound')
    m.patchState('keys-au', 'c2FtcGxlciBkZWx1eGU=')
    m.send({ type: 'exportSoundPreset', id: 'keys-au' })
    expect(m.state.message).toMatchObject({ error: false, text: 'Keys (AU) exported to ~/Library/Audio/Presets' })
    m.send({ type: 'exportSoundPreset', id: 'keys-au' })
    expect(m.state.message?.text).toContain('already exists')
    m.send({ type: 'exportSoundPreset', id: 'keys-au', overwrite: true })
    expect(m.state.message?.error).toBe(false)
  })

  it('updatePatch without a plugin state keeps the stored one; the state shows only hasState', () => {
    const m = new MockSession({ manual: true })
    m.patchState('keys-au', 'c2FtcGxlciBkZWx1eGU=')
    const p = m.state.soundLibrary.patches.find((q) => q.id === 'keys-au')!
    expect(p.source).toEqual({ kind: 'plugin', componentId: 'aumu dls  appl', hasState: true })
    m.send({ type: 'updatePatch', id: 'keys-au', patch: { name: 'Renamed', category: 'ePiano', tags: [], favourite: false, source: { kind: 'plugin', componentId: 'aumu dls  appl' } } })
    const q = m.state.soundLibrary.patches.find((x) => x.id === 'keys-au')!
    expect(q.name).toBe('Renamed')
    expect(q.source).toMatchObject({ hasState: true })
    expect(m.patchState('keys-au')).toBe('c2FtcGxlciBkZWx1eGU=')
    // Another plugin: its own (empty) state.
    m.send({ type: 'updatePatch', id: 'keys-au', patch: { name: 'Renamed', category: 'ePiano', tags: [], favourite: false, source: { kind: 'plugin', componentId: 'aumu Smp7 Fake' } } })
    expect(m.state.soundLibrary.patches.find((x) => x.id === 'keys-au')!.source).toMatchObject({ hasState: false })
  })

  it('Chord Looper banks: Save As, a clash refused unless overwritten, Load (#201)', () => {
    const m = new MockSession({ manual: true })
    expect([m.state.looper.bankName, m.state.looper.bankPath]).toEqual(['New Bank', null])
    m.send({ type: 'saveLooperBank', name: null })
    expect(m.state.message?.error).toBe(true)
    m.send({ type: 'saveLooperBank', name: 'Songs' })
    expect(m.state.looper.bankName).toBe('Songs')
    const songs = m.state.looper.bankPath!
    m.send({ type: 'newLooperBank' })
    expect(m.state.looper.bankPath).toBe(null)
    m.send({ type: 'saveLooperBank', name: 'Songs' })
    expect(m.state.looper.bankPath).toBe(null)
    m.send({ type: 'saveLooperBank', name: 'Songs', overwrite: true })
    expect(m.state.looper.bankPath).toBe(songs)
    m.send({ type: 'saveLooperBank', name: 'Ballads' })
    expect(m.state.looper.banks.map((b) => b.name)).toEqual(['Ballads', 'Songs'])
    m.send({ type: 'loadLooperBank', path: songs })
    expect(m.state.looper.bankName).toBe('Songs')
  })

  it('Kbd Harmony/Arpeggio and Arpeggio Hold are control-side switches, as the engine keeps them', () => {
    const m = new MockSession({ manual: true })
    // Try: a press switches them; no pedal switch field is written.
    m.send({ type: 'triggerFunction', function: 'kbdHarmonyArp' })
    expect(m.state.harmonyArp.on).toBe(true)
    expect('kbdHarmonyArp' in m.state.controllers).toBe(false)
    m.send({ type: 'triggerFunction', function: 'arpHold' })
    expect(m.state.harmonyArp.arp.pedalHold).toBe(true)
    expect(m.state.harmonyArp.arp.hold).toBe(false)
    m.send({ type: 'triggerFunction', function: 'arpHold' })
    m.send({ type: 'triggerFunction', function: 'kbdHarmonyArp' })
    // A Hold B pedal picked with the pedal up turns the switch on; given another function
    // it lets go.
    const set = (fn: string, controlType: 'holdA' | 'holdB' | 'toggle') =>
      m.send({ type: 'setPedal', pedal: 1, cc: 66, function: fn, controlType, reverse: false, range: 'upper' })
    set('arpHold', 'holdB')
    expect(m.state.harmonyArp.arp.pedalHold).toBe(true)
    expect(m.state.harmonyArp.arp.hold).toBe(false)
    set('sostenuto', 'holdA')
    expect(m.state.harmonyArp.arp.pedalHold).toBe(false)
    set('kbdHarmonyArp', 'holdA')
    expect(m.state.harmonyArp.on).toBe(false)
    set('kbdHarmonyArp', 'holdB')
    expect(m.state.harmonyArp.on).toBe(true)
    // PANIC lets go of what a Hold pedal was keeping on (Hold B, up); the setting stays.
    set('arpHold', 'holdB')
    m.send({ type: 'setArpHold', on: true })
    m.send({ type: 'panic' })
    expect(m.state.harmonyArp.arp.pedalHold).toBe(false)
    expect(m.state.harmonyArp.arp.hold).toBe(true)
  })

  it('names notes Yamaha-style and transposes chords', () => {
    expect(noteName(60)).toBe('C3')
    expect(noteName(54)).toBe('F#2')
    expect(transposeChord('Am7/G', 2)).toBe('Bm7/A')
    expect(transposeChord('C', -1)).toBe('B')
  })
})

describe('mock knobs (#197)', () => {
  it('a knob turn runs its function from the value in effect, as the session', () => {
    const m = new MockSession({ manual: true })
    expect(m.state.knobs.knobs.map((k) => k.short)).toEqual(['DynCtrl', 'RtgRate', 'RtgOnOff', 'StyMuteA', 'StyMuteB', 'Swing', '---', 'Tempo'])
    m.send({ type: 'turnKnob', knob: 0, delta: -4 })
    expect(m.state.dynamics.level).toBe(119)
    expect(m.state.knobs.knobs[0]).toMatchObject({ value: '119', level: 119 })
    // Track Mute A fully left: only Rhythm 2 plays.
    m.send({ type: 'turnKnob', knob: 3, delta: -40 })
    expect(m.state.mixer.styleParts.map((p) => p.on)).toEqual([false, true, false, false, false, false, false, false])
    expect(m.state.knobs.knobs[3].value).toBe('1 of 8')
    // Retrigger on/off switches every 3 steps.
    m.send({ type: 'turnKnob', knob: 2, delta: 2 })
    expect(m.state.transport.retrigger).toBe(false)
    m.send({ type: 'turnKnob', knob: 2, delta: 1 })
    expect(m.state.transport.retrigger).toBe(true)
    m.send({ type: 'stepKnobPage', delta: 1 })
    expect(m.state.knobs).toMatchObject({ page: 'rack', pageNumber: 2, pageCount: 6 })
    const v = m.state.keyboardParts[1].volume
    m.send({ type: 'turnKnob', knob: 1, delta: -1 })
    expect(m.state.keyboardParts[1].volume).toBe(Math.max(0, v - 2))
    // Pan and the effect sends (#198).
    m.send({ type: 'setKnobPage', page: 'pan' })
    const pan = m.state.keyboardParts[0].pan
    m.send({ type: 'turnKnob', knob: 0, delta: 1 })
    expect(m.state.keyboardParts[0].pan).toBe(Math.min(127, pan + 2))
    // One page per effect: the parts' sends to it, its parameters, its return on knob 8.
    m.send({ type: 'setKnobPage', page: 'reverb' })
    expect(m.state.knobs.knobs.map((k) => k.short)).toEqual(['RevR1', 'RevR2', 'RevR3', 'RevL', 'RevTime', 'PreDly', 'RevTone', 'RevRtn'])
    const rev = m.state.keyboardParts[3].reverb
    m.send({ type: 'turnKnob', knob: 3, delta: 3 })
    expect(m.state.keyboardParts[3].reverb).toBe(Math.min(127, rev + 6))
    expect(m.state.knobs.knobs[3].value).toBe(String(m.state.keyboardParts[3].reverb))
    // A parameter knob (#236) moves in its own step and pins the block to the player's own.
    expect(m.state.effects.blocks[0].followStyle).toBe(true)
    m.send({ type: 'turnKnob', knob: 4, delta: 1 })
    expect(m.state.effects.blocks[0].params[0].display).toBe('2.5 s')
    expect(m.state.effects.blocks[0].followStyle).toBe(false)
    m.send({ type: 'stepKnobPage', delta: 1 })
    expect(m.state.knobs.knobs.map((k) => k.short)).toEqual(['ChoR1', 'ChoR2', 'ChoR3', 'ChoL', 'ChoRate', 'ChoDepth', '---', 'ChoRtn'])
    m.send({ type: 'stepKnobPage', delta: 1 })
    expect(m.state.knobs).toMatchObject({ page: 'delay', pageName: 'Delay', pageNumber: 6 })
    expect(m.state.knobs.knobs.map((k) => k.short)).toEqual(['DlyR1', 'DlyR2', 'DlyR3', 'DlyL', 'DlyTime', 'DlyFdbk', 'DlyTone', 'DlyRtn'])
    m.send({ type: 'turnKnob', knob: 0, delta: 5 })
    expect(m.state.keyboardParts[0].variation).toBe(10)
    expect(m.state.knobs.knobs[4].value).toBe('1/8.')
    m.send({ type: 'turnKnob', knob: 4, delta: 3 })
    expect(m.state.knobs.knobs[4].value).toBe('1/4')
    const ret = m.state.effects.blocks[2].returnLevel
    m.send({ type: 'turnKnob', knob: 7, delta: -1 })
    expect(m.state.effects.blocks[2].returnLevel).toBe(ret - 2)
  })
})
