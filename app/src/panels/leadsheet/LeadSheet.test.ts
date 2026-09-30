import { cleanup, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import type { AppState } from '../../lib/api/types'
import { app, ui } from '../../lib/store.svelte'
import LeadSheet from './LeadSheet.svelte'

/** Renders the band on a fixed state (not the mock's), for values the mock doesn't reach. */
function renderState(edit: (st: AppState) => void) {
  const session = new MockSession({ manual: true, demo: true })
  const st = JSON.parse(JSON.stringify(session.state)) as AppState
  edit(st)
  app.attach({ kind: 'tauri', subscribe: (fn) => (fn(st), () => {}), send: () => {}, library: () => session.library(), meters: () => session.meters(), dispose: () => {} })
  flushSync()
  render(LeadSheet)
}

function setup(demo = true) {
  const session = new MockSession({ manual: true, demo })
  app.attach(session)
  flushSync()
  render(LeadSheet)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
})

const text = (sel: string) => document.querySelector(sel)!.textContent!.replace(/\s+/g, ' ').trim()

describe('status line', () => {
  afterEach(() => {
    ui.browser = false
  })

  it('shows the style; its name and Browse open the style browser', () => {
    const session = setup()
    const style = document.querySelector<HTMLButtonElement>('.status .style')!
    expect(style.getAttribute('data-tip')).toBe('browser.open')
    expect(text('.status .sname')).toBe(session.state.style.name)
    expect(ui.browser).toBe(false)
    document.querySelector<HTMLElement>('.status .sname')!.click()
    flushSync()
    expect(ui.browser).toBe(true)
    ui.browser = false
    document.querySelector<HTMLElement>('.status .browse')!.click()
    flushSync()
    expect(ui.browser).toBe(true)
  })

  it('shows the chord, fingering and transpose from the state, and follows them', () => {
    const session = setup()
    expect(text('.status .cname')).toBe('Am7')
    expect(document.querySelector('.status .chord')!.getAttribute('data-tip')).toBe('display.chord')
    expect(text('.status .fingering')).toBe(session.state.chord.fingeringName)
    expect(text('.status .transpose')).toBe('Transpose 0 · 0')
    expect(document.querySelector('.status .transpose')!.classList.contains('set')).toBe(false)
    session.send({ type: 'setFingering', fingering: 'multiFinger' })
    session.send({ type: 'setTranspose', keyboard: 2, master: -1 })
    flushSync()
    expect(text('.status .fingering')).toBe('Multi Finger')
    expect(text('.status .transpose')).toBe('Transpose +2 · −1')
    expect(document.querySelector('.status .transpose')!.classList.contains('set')).toBe(true)
  })

  it('shows the time signature, the chord as fingered when transposed, and Manual Bass', () => {
    renderState((st) => {
      st.style.timeSignature = [3, 4]
      st.chord.name = 'D7'
      st.chord.fingered = 'C7'
      st.chord.manualBassActive = true
      st.chord.fingeringName = 'Fingered'
    })
    expect(text('.status .timesig')).toBe('3/4')
    expect(document.querySelector('.status .timesig')!.getAttribute('data-tip')).toBe('display.timesig')
    expect(text('.status .cname')).toBe('D7')
    expect(text('.status .fingered')).toBe('played C7')
    expect(text('.status .fingering')).toBe('Fingered · Manual Bass')
  })

  it('no chord yet: a dash', () => {
    renderState((st) => {
      st.chord.name = null
      st.chord.fingered = null
    })
    expect(text('.status .cname')).toBe('–')
    expect(document.querySelector('.status .fingered')).toBeNull()
  })
})

describe('lead-sheet band', () => {
  it('shows the section playing, one cell per bar of it, and the current bar', () => {
    const session = setup()
    const t = session.state.transport
    expect(text('.now')).toContain('Main B')
    expect(document.querySelectorAll('.cell')).toHaveLength(t.sectionBars!)
    const current = [...document.querySelectorAll('.cell')].findIndex((c) => c.classList.contains('current'))
    expect(current).toBe((t.bar - 1) % t.sectionBars!)
    expect(document.querySelectorAll('.cell.current .beats i')).toHaveLength(t.beatsPerBar)
  })

  it('shows the queued section next, in amber', () => {
    const session = setup()
    expect(text('.next')).toContain('–')
    session.send({ type: 'main', index: 2 })
    flushSync()
    expect(text('.next')).toContain('→ Main C')
    expect(document.querySelector('.next')!.classList.contains('queued')).toBe(true)
  })

  it('stopped: what the band will start with', () => {
    setup(false)
    expect(text('.now')).toContain('Starts with')
    expect(text('.now')).toContain('Main A')
    expect(text('.now')).toContain('waiting for your chord')
    expect(document.querySelectorAll('.cell.current')).toHaveLength(0)
  })

  it('is the chart slot the M8 chord chart renders into', () => {
    setup()
    expect(document.querySelector('[data-slot="chart"]')).not.toBeNull()
  })

  it('in chart mode shows the chart, eight bars a line, with the bar playing ringed', () => {
    const session = setup(false)
    session.send({ type: 'importCharts', text: 'irealb://demo' })
    session.send({ type: 'setChartIntro', index: null })
    session.send({ type: 'setChartMode', on: true })
    flushSync()
    const lane = document.querySelector('[data-slot="chart"]')!
    expect(lane.getAttribute('data-tip')).toBe('lead.chart')
    expect(lane.querySelectorAll('.line')).toHaveLength(2)
    expect(lane.querySelectorAll('.cell')).toHaveLength(16)
    expect(lane.querySelector('.sec')!.textContent).toBe('A')
    expect(lane.querySelectorAll('.cell')[0].textContent).toContain('Bb6')
    session.send({ type: 'startStop' })
    const bar = (60000 / session.state.transport.tempo) * session.state.transport.beatsPerBar
    session.advance(bar * 9.1)
    flushSync()
    const n = session.state.chart.bar!
    expect(n).toBeGreaterThanOrEqual(8)
    // The line playing comes first: bars 9-16, the current one ringed.
    const cells = [...lane.querySelectorAll('.cell')]
    expect(cells[0].querySelector('.num')!.textContent).toBe('9')
    expect(cells.findIndex((c) => c.classList.contains('current'))).toBe(n - 8)
    expect(text('.now')).toContain(`bar ${n + 1} of 32`)
  })

  it('without the section length (an engine older than sectionBars): one cell, and the bar counted from the clock', () => {
    const session = new MockSession({ manual: true, demo: true })
    const st = { ...session.state, transport: { ...session.state.transport, sectionBars: undefined } } as unknown as AppState
    app.attach({ kind: 'tauri', subscribe: (fn) => (fn(st), () => {}), send: () => {}, library: () => session.library(), meters: () => session.meters(), dispose: () => {} })
    flushSync()
    render(LeadSheet)
    const c = st.surface.clock
    const pos = c.sectionAnchorBeats + ((c.atMs - c.sectionAnchorMs) * c.tempo) / 60000
    expect(document.querySelectorAll('.cell')).toHaveLength(1)
    expect(text('.now')).toContain(`bar ${Math.floor(pos / c.beatsPerBar) + 1}`)
    expect(text('.now')).not.toContain(' of ')
  })
})
