import { cleanup, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import type { AppState } from '../../lib/api/types'
import { app } from '../../lib/store.svelte'
import LeadSheet from './LeadSheet.svelte'

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
