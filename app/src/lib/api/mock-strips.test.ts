// @vitest-environment node
// The mock's channel strips and send effects: the same refusals, clamping and older-state
// bookkeeping as the Rust `api::Strips` (src/api/strips.rs).
import { describe, expect, it } from 'vitest'
import { MockSession } from './mock'
import { MockStrips, stripLegacy } from './mock-strips'

describe('mock strips', () => {
  it('every part has a strip and the effects have sends 1-3 from the buses', () => {
    const m = new MockSession({ manual: true })
    expect(m.state.keyboardParts.every((p) => p.strip.inserts.length === 2 && p.strip.sends.length === 6)).toBe(true)
    expect(m.state.mixer.styleParts.every((p) => p.strip.inserts.length === 2 && p.strip.sends.length === 6)).toBe(true)
    expect(m.state.effects.sends.map((s) => [s.kind, s.fromStyle, s.setByRack, s.returnLevel])).toEqual([
      ['hall', true, false, 64], ['chorus', true, false, 64], ['dottedEighth', true, false, 64],
    ])
    expect(m.state.effects.sends[0].params.map((p) => [p.name, p.value, p.display])).toEqual([['Time', 24, '2.4 s'], ['Pre-delay', 22, '22 ms'], ['Tone', 45, '4.5 kHz']])
    expect(m.state.effects.sends[2].params).toHaveLength(6)
    // The style part's sends 1-3 are its reverb, chorus and variation.
    const sp = m.state.mixer.styleParts[6]
    expect(sp.strip.sends.slice(0, 3)).toEqual([sp.reverb, sp.chorus, sp.variation])
    // The mock style's insert (Chord 1, a distortion) is Chord 1's insert 1.
    expect(m.state.mixer.styleParts[3].strip.inserts[0]).toMatchObject({ kind: 'distortion', name: 'Distortion', on: true })
    expect(m.state.mixer.styleParts[3].strip.inserts[0].settings[0].value).toBe(64)
  })

  it('older commands cover what they did, both ways', () => {
    expect(stripLegacy({ type: 'setStripSend', strip: 1, send: 0, level: 90 })).toEqual([{ type: 'setPartSend', part: 1, send: 'reverb', value: 90 }])
    expect(stripLegacy({ type: 'setStripInsertOn', strip: 6, slot: 0, on: false })).toEqual([{ type: 'setPartInsertOn', part: 2, on: false }])
    expect(stripLegacy({ type: 'setSendKind', send: 0, kind: 'plate' })).toEqual([{ type: 'setEffectType', block: 'reverb', effect: 'plate' }])
    expect(stripLegacy({ type: 'setStripInsertOn', strip: 6, slot: 1, on: true })).toEqual([])
    expect(stripLegacy({ type: 'setStripSend', strip: 1, send: 3, level: 9 })).toEqual([])

    const m = new MockSession({ manual: true })
    m.send({ type: 'setStripSend', strip: 1, send: 1, level: 70 })
    expect(m.state.keyboardParts[1].chorus).toBe(70)
    expect(m.state.keyboardParts[1].strip.sends[1]).toBe(70)
    m.send({ type: 'setStripSend', strip: 5, send: 2, level: 33 })
    expect(m.state.mixer.styleParts[1].variation).toBe(33)
    expect(m.state.mixer.styleParts[1].strip.sends[2]).toBe(33)
    // The older way shows in the strip too.
    m.send({ type: 'setPartEq', part: 2, eq: { lowGain: 5, lowFreq: 100, highGain: -3, highFreq: 8000 } })
    expect(m.state.keyboardParts[2].strip.eq).toEqual({ lowGain: 5, lowFreq: 100, highGain: -3, highFreq: 8000 })
    m.send({ type: 'setStripEq', strip: 0, eq: { lowGain: 40, lowFreq: 80, highGain: 0, highFreq: 10000 } })
    expect(m.state.keyboardParts[0].eq.lowGain).toBe(12)
    expect(m.state.keyboardParts[0].strip.eq.lowGain).toBe(12)
    // Insert 1 of a keyboard strip is its insert slot; its first setting the amount.
    m.send({ type: 'setStripInsertKind', strip: 0, slot: 0, kind: 'rotary' })
    m.send({ type: 'setStripInsertOn', strip: 0, slot: 0, on: true })
    m.send({ type: 'setStripInsertSetting', strip: 0, slot: 0, setting: 0, value: 99 })
    expect(m.state.keyboardParts[0].insert).toEqual({ effect: 'rotary', on: true, amount: 99 })
    expect(m.state.keyboardParts[0].strip.inserts[0]).toMatchObject({ kind: 'rotary', name: 'Rotary', on: true })
    expect(m.state.keyboardParts[0].strip.inserts[0].settings.map((s) => s.value)).toEqual([99, 0, 64])
    // Sends 1-3's kind, parameters and return are the buses'.
    m.send({ type: 'setSendKind', send: 0, kind: 'plate' })
    expect(m.state.effects.blocks[0].effect).toBe('plate')
    expect(m.state.effects.sends[0]).toMatchObject({ kind: 'plate', name: 'Plate' })
    expect(m.state.effects.sends[0].params[0].value).toBe(18)
    m.send({ type: 'setSendParam', send: 1, param: 0, value: 9999 })
    expect(m.state.effects.blocks[1].params[0].value).toBe(500)
    expect(m.state.effects.sends[1].params[0]).toMatchObject({ value: 500, display: '5.00 Hz' })
    m.send({ type: 'setSendReturn', send: 2, level: 100 })
    expect(m.state.effects.blocks[2].returnLevel).toBe(100)
    expect(m.state.effects.sends[2].returnLevel).toBe(100)
  })

  it('sends are added and removed with their levels, six at most', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setStripSend', strip: 0, send: 3, level: 10 })
    expect(m.state.message).toMatchObject({ error: true, text: 'no send 4 (there are 3)' })
    m.send({ type: 'addSend', kind: 'phaser' })
    m.send({ type: 'addSend', kind: 'room' })
    expect(m.state.effects.sends.map((s) => [s.send, s.kind, s.fromStyle, s.setByRack])).toEqual([
      [0, 'hall', true, false], [1, 'chorus', true, false], [2, 'dottedEighth', true, false], [3, 'phaser', false, true], [4, 'room', false, true],
    ])
    expect(m.state.effects.sends[3].params.map((p) => p.display)).toEqual(['64', '0.50 Hz', '40%'])
    m.send({ type: 'setStripSend', strip: 0, send: 4, level: 77 })
    m.send({ type: 'removeSend', send: 3 })
    expect(m.state.effects.sends.map((s) => s.kind)).toEqual(['hall', 'chorus', 'dottedEighth', 'room'])
    expect(m.state.keyboardParts[0].strip.sends[3]).toBe(77)
    expect(m.state.keyboardParts[0].strip.sends[4]).toBe(0)
    m.send({ type: 'removeSend', send: 0 })
    expect(m.state.message?.error).toBe(true)
    m.send({ type: 'addSend', kind: 'shimmer' })
    expect(m.state.message).toMatchObject({ error: true, text: 'no send kind "shimmer"' })
    m.send({ type: 'addSend', kind: 'hall' })
    m.send({ type: 'addSend', kind: 'hall' })
    expect(m.state.effects.sends).toHaveLength(6)
    m.send({ type: 'addSend', kind: 'hall' })
    expect(m.state.message).toMatchObject({ error: true, text: 'there are already 6 sends' })
  })

  it('refuses what the Rust model refuses and clamps to the ranges', () => {
    const s = new MockStrips()
    expect(s.apply({ type: 'setStripInsertKind', strip: 4, slot: 0, kind: 'ringModulator' })).toBe('no insert kind "ringModulator"')
    expect(s.apply({ type: 'setStripInsertKind', strip: 12, slot: 0, kind: 'phaser' })).toBe('no strip 12 (0-11)')
    expect(s.apply({ type: 'setStripInsertKind', strip: 4, slot: 2, kind: 'phaser' })).toBe('no insert slot 2 (0-1)')
    // A new kind starts at its defaults; on/off unchanged.
    expect(s.apply({ type: 'setStripInsertKind', strip: 11, slot: 1, kind: 'distortion' })).toBeNull()
    expect(s.apply({ type: 'setStripInsertSetting', strip: 11, slot: 1, setting: 0, value: 500 })).toBeNull()
    expect(s.strips[11].inserts[1]).toEqual({ kind: 'distortion', on: false, values: [127, 64, 100, 0] })
    expect(s.apply({ type: 'setStripInsertSetting', strip: 11, slot: 1, setting: 3, value: 1 })).toBe('Distortion has no setting 4')
    expect(s.apply({ type: 'setStripInsertKind', strip: 11, slot: 1, kind: 'compressor' })).toBeNull()
    expect(s.strips[11].inserts[1].values).toEqual([64, 3, 150, 100])
    // Sends 1-3 play only their own bus's kinds.
    expect(s.apply({ type: 'setSendKind', send: 0, kind: 'chorus' })).toBe('send 1 plays Reverb types')
    expect(s.apply({ type: 'setSendKind', send: 2, kind: 'phaser' })).toBe('send 3 plays Variation types')
    expect(s.apply({ type: 'setSendKind', send: 2, kind: 'pingPong' })).toBeNull()
    expect(s.styleSends[2].params).toEqual([1, 2, 375, 38, 50, 1])
    expect(s.apply({ type: 'setSendParam', send: 1, param: 2, value: 1 })).toBe('Chorus has no parameter 3')
    expect(s.apply({ type: 'setRackSendOverride', send: 3, on: true })).toBe('only sends 1-3 have an override (not 4)')
    // The compressor: a type brings its parameters; a parameter is clamped and marks it edited.
    expect(s.apply({ type: 'setStripCompressorPreset', strip: 2, preset: 'punchy' })).toBeNull()
    expect(s.apply({ type: 'setStripCompressorParam', strip: 2, param: 'ratio', value: 999 })).toBeNull()
    expect(s.apply({ type: 'setStripCompressorParam', strip: 2, param: 'threshold', value: -100 })).toBeNull()
    const m = new MockSession({ manual: true })
    m.send({ type: 'setStripCompressorOn', strip: 2, on: true })
    m.send({ type: 'setStripCompressorPreset', strip: 2, preset: 'loud' })
    expect(m.state.keyboardParts[2].strip.comp).toEqual({ on: true, preset: 'loud', threshold: -30, ratio: 80, attack: 2, release: 150, makeup: 9, edited: false })
    m.send({ type: 'setStripCompressorParam', strip: 2, param: 'release', value: 5000 })
    expect(m.state.keyboardParts[2].strip.comp).toMatchObject({ release: 1000, edited: true })
    m.send({ type: 'setRackSendOverride', send: 1, on: true })
    expect(m.state.effects.sends[1].setByRack).toBe(true)
    m.send({ type: 'setStripInsertKind', strip: 11, slot: 1, kind: 'tremolo' })
    expect(m.state.mixer.styleParts[7].strip.inserts[1]).toMatchObject({ kind: 'tremolo', name: 'Tremolo' })
    expect(m.state.mixer.styleParts[7].strip.inserts[1].settings.map((x) => x.display)).toEqual(['64', '1/8', '0'])
  })

  it('none empties a keyboard insert 1, and a setting on an empty one is refused', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setStripInsertKind', strip: 0, slot: 0, kind: 'rotary' })
    m.send({ type: 'setStripInsertKind', strip: 0, slot: 0, kind: 'none' })
    expect(m.state.keyboardParts[0].strip.inserts[0].kind).toBe('none')
    m.send({ type: 'setStripInsertSetting', strip: 1, slot: 0, setting: 0, value: 10 })
    expect(m.state.message?.error).toBe(true)
  })

  it('an insert 2 change marks the live rack modified', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setPartOn', part: 0, on: true })
    m.state.liveRack.modified = false
    m.send({ type: 'setStripInsertKind', strip: 0, slot: 1, kind: 'phaser' })
    expect(m.state.liveRack.modified).toBe(true)
  })

  it('a keyboard strip keeps its voice settings; a Style strip refuses them', () => {
    const m = new MockSession({ manual: true })
    const kp = () => m.state.keyboardParts[1].strip
    expect([kp().tone.cutoff, kp().tone.vibratoDelay, kp().mono, kp().portamento]).toEqual([64, 64, false, { on: false, time: 0 }])
    m.send({ type: 'setStripTone', strip: 1, control: 'cutoff', value: 200 })
    m.send({ type: 'setStripTone', strip: 1, control: 'vibratoDepth', value: 90 })
    m.send({ type: 'setStripMono', strip: 1, on: true })
    m.send({ type: 'setStripPortamento', strip: 1, on: true, time: 30 })
    expect([kp().tone.cutoff, kp().tone.vibratoDepth, kp().tone.release, kp().mono, kp().portamento]).toEqual([127, 90, 64, true, { on: true, time: 30 }])
    expect(m.state.keyboardParts[0].strip.mono).toBe(false)
    for (const cmd of [
      { type: 'setStripMono', strip: 4, on: true },
      { type: 'setStripTone', strip: 11, control: 'attack', value: 1 },
      { type: 'setStripPortamento', strip: 7, on: true, time: 5 },
    ] as const) {
      m.state.message = null
      m.send(cmd)
      expect(m.state.message).toMatchObject({ error: true, text: `strip ${cmd.strip} has no voice settings (only the keyboard strips, 0-3)` })
    }
    expect(m.state.mixer.styleParts.every((p) => !p.strip.mono && p.strip.tone.attack === 64 && !p.strip.portamento.on)).toBe(true)
    m.state.message = null
    m.send({ type: 'setStripMono', strip: 12, on: true })
    expect(m.state.message).toMatchObject({ error: true })
  })

  it('a Compressor insert starts at attack 3 ms and release 150 ms', () => {
    const m = new MockSession({ manual: true })
    m.send({ type: 'setStripInsertKind', strip: 0, slot: 1, kind: 'compressor' })
    const s = m.state.keyboardParts[0].strip.inserts[1].settings
    expect(s.map((x) => [x.name, x.value, x.min, x.max])).toEqual([['Squeeze', 64, 0, 127], ['Attack', 3, 1, 80], ['Release', 150, 10, 1000], ['Output', 100, 0, 127]])
  })
})
