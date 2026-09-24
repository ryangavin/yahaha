// The Registration bar and panel on the mock session: the ten buttons light and press
// like pad page 4, Memory arms Memorize, the panel's pages send their commands.

import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { app, ui } from '../../lib/store.svelte'
import RegistBar from './RegistBar.svelte'
import Registration from './Registration.svelte'
import { buttonLook, buttonSummary, sequenceText } from './regist'

function setup() {
  const session = new MockSession({ manual: true })
  app.attach(session)
  return session
}

const q = <T extends Element = HTMLButtonElement>(sel: string) => document.querySelector<T>(sel)!
const all = (sel: string) => [...document.querySelectorAll<HTMLButtonElement>(sel)]

afterEach(() => {
  cleanup()
  ui.regist = false
  ui.registTab = 'bank'
})

describe('Registration bar', () => {
  it('starts on the demo bank with its stored buttons lit blue', () => {
    const s = setup()
    const r = s.state.registration
    expect(r.bank.name).toBe('Friday Gig')
    expect(buttonLook(r, 0)).toMatchObject({ level: 'bright', rgb: [0, 40, 127] })
    expect(buttonLook(r, 9).level).toBe('off')
    expect(sequenceText(r)).toBe('– / 6')
  })

  it('a button recalls; the lit one turns red', async () => {
    const s = setup()
    render(RegistBar)
    await fireEvent.click(q('[data-tip="regist.4"]'))
    const r = s.state.registration
    expect(r.selected).toBe(3)
    expect(buttonLook(r, 3)).toMatchObject({ rgb: [127, 0, 0], anim: 'solid' })
    expect(s.state.transport.tempo).toBe(132)
    expect(s.state.keyboardParts[0].program).toBe(26)
  })

  it('Memory arms Memorize: every button flashes, the next press stores the panel', async () => {
    const s = setup()
    render(RegistBar)
    s.send({ type: 'setPartVoice', part: 0, program: 71 })
    await fireEvent.click(q('[data-tip="regist.memory"]'))
    expect(buttonLook(s.state.registration, 9).anim).toBe('flash')
    await fireEvent.click(q('[data-tip="regist.10"]'))
    const r = s.state.registration
    expect(r.memory).toBe(false)
    expect(r.buttons[9].stored).toBe(true)
    expect(buttonSummary(r, 9)).toContain(s.state.style.name)
  })

  it('Regist + walks the sequence; the playlist steps songs', async () => {
    const s = setup()
    render(RegistBar)
    await fireEvent.click(q('[data-tip="regist.seq_next"]'))
    await fireEvent.click(q('[data-tip="regist.seq_next"]'))
    expect(s.state.registration.sequence.position).toBe(1)
    expect(s.state.registration.selected).toBe(1)
    await fireEvent.click(q('[data-tip="playlist.next"]'))
    await fireEvent.click(q('[data-tip="playlist.next"]'))
    expect(s.state.playlist.current).toBe(1)
    expect(s.state.registration.selected).toBe(3)
  })
})

describe('Registration panel', () => {
  it('Freeze keeps the ticked groups on recall', async () => {
    const s = setup()
    ui.registTab = 'groups'
    render(Registration)
    s.send({ type: 'tempoUp' })
    const tempo = s.state.transport.tempo
    const freezeTempo = all('[data-tip="regist.freeze_group"]').find((b) => b.textContent?.includes('Tempo'))!
    await fireEvent.click(freezeTempo)
    await fireEvent.click(q('[data-tip="regist.freeze"]'))
    s.send({ type: 'recallRegist', index: 3 })
    expect(s.state.transport.tempo).toBe(tempo)
    expect(s.state.keyboardParts[0].program).toBe(26)
  })

  it('the sequence page adds and removes steps and sets the end', async () => {
    const s = setup()
    ui.registTab = 'sequence'
    render(Registration)
    await fireEvent.click(all('[data-tip="regist.sequence_steps"]')[4])
    expect(s.state.registration.sequence.steps.at(-1)).toBe(4)
    flushSync()
    await fireEvent.click(all('[data-tip="regist.sequence_step"]')[0])
    expect(s.state.registration.sequence.steps).toEqual([1, 2, 1, 2, 3, 4])
    const top = all('[data-tip="regist.sequence_end"]').find((b) => b.textContent?.includes('Top'))!
    await fireEvent.click(top)
    expect(s.state.registration.sequence.end).toBe('top')
  })

  it('the playlist page loads records, and sorting turns off moving', async () => {
    const s = setup()
    ui.registTab = 'playlist'
    render(Registration)
    await fireEvent.click(all('[data-tip="playlist.record"]')[1])
    expect(s.state.playlist.current).toBe(1)
    expect(s.state.registration.selected).toBe(3)
    expect(all('[data-tip="playlist.up"]').length).toBe(4)
    const az = all('[data-tip="playlist.sort"]').find((b) => b.textContent?.includes('A → Z'))!
    await fireEvent.click(az)
    flushSync()
    expect(all('[data-tip="playlist.up"]').length).toBe(0)
    await fireEvent.click(q('[data-tip="playlist.add_style"]'))
    expect(s.state.playlist.records.length).toBe(5)
  })

  it('saving a new bank gives it a file', async () => {
    const s = setup()
    render(Registration)
    s.send({ type: 'newRegistBank' })
    s.send({ type: 'memorizeRegist', index: 0 })
    flushSync()
    expect(s.state.registration.bank.dirty).toBe(true)
    const name = q<HTMLInputElement>('[data-tip="regist.bank_name"]')
    await fireEvent.input(name, { target: { value: 'Sunday' } })
    await fireEvent.click(q('[data-tip="regist.save_bank"]'))
    const r = s.state.registration
    expect(r.bank.name).toBe('Sunday')
    expect(r.bank.dirty).toBe(false)
    expect(r.banks.map((b) => b.name)).toContain('Sunday')
  })

  it("saving under another bank's name needs Overwrite", async () => {
    const s = setup()
    render(Registration)
    const gig = s.state.registration.bank.name
    s.send({ type: 'newRegistBank' })
    s.send({ type: 'memorizeRegist', index: 0 })
    flushSync()
    const name = q<HTMLInputElement>('[data-tip="regist.bank_name"]')
    await fireEvent.input(name, { target: { value: gig } })
    flushSync()
    await fireEvent.click(q('[data-tip="regist.save_bank"]'))
    expect(s.state.registration.bank.path).toBe(null)
    expect(s.state.message?.error).toBe(true)
    await fireEvent.click(q('[data-tip="regist.overwrite_bank"]'))
    expect(s.state.registration.bank.name).toBe(gig)
    expect(s.state.registration.bank.dirty).toBe(false)
  })

  it('sequence on/off stays when the bank changes', () => {
    const s = setup()
    s.send({ type: 'setRegistSequenceOn', on: true })
    s.send({ type: 'stepRegistBank', delta: 1 })
    expect(s.state.registration.sequence.on).toBe(true)
  })
})
