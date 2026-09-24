import { cleanup, fireEvent, render } from '@testing-library/svelte'
import { flushSync } from 'svelte'
import { afterEach, describe, expect, it } from 'vitest'
import { MockSession } from '../../lib/api/mock'
import { DEMO_SONGS } from '../../lib/api/mock-chart'
import { app } from '../../lib/store.svelte'
import Charts from './Charts.svelte'

function setup() {
  const session = new MockSession({ manual: true, demo: false })
  app.attach(session)
  flushSync()
  render(Charts)
  return session
}

afterEach(() => {
  cleanup()
  app.detach()
})

const tipped = (key: string) => [...document.querySelectorAll<HTMLElement>(`[data-tip="${key}"]`)]
const click = async (el: Element) => {
  await fireEvent.click(el)
  flushSync()
}

describe('charts drawer', () => {
  it('imports a pasted link and lists the playlist and its songs', async () => {
    const session = setup()
    expect(document.body.textContent).toContain('None imported')
    const link = tipped('chart.link')[0] as HTMLInputElement
    await fireEvent.input(link, { target: { value: 'irealb://demo' } })
    await click(tipped('chart.import_link')[0])
    expect(session.state.chart.playlists).toHaveLength(1)
    expect(tipped('chart.song')).toHaveLength(DEMO_SONGS.length)
    // The first song is chosen and marked.
    expect(document.querySelector('.song.chosen')!.textContent).toContain(DEMO_SONGS[0].title)
    expect(link.value).toBe('')
  })

  it('chooses songs, chart mode, choruses, intro, ending and loop', async () => {
    const session = setup()
    session.send({ type: 'importCharts', text: 'irealb://demo' })
    flushSync()
    await click(tipped('chart.song')[1])
    expect(session.state.chart.selected).toEqual([0, 1])
    expect(document.querySelector('.song-now')!.textContent).toContain(DEMO_SONGS[1].title)
    await click(tipped('chart.mode')[0])
    expect(session.state.chart.on).toBe(true)
    await click(tipped('chart.choruses_up')[0])
    expect(session.state.chart.choruses).toBe(2)
    expect(session.state.chart.song!.bars).toHaveLength(32)
    await click(tipped('chart.intro').find((b) => b.textContent === 'None')!)
    expect(session.state.chart.intro).toBeNull()
    await click(tipped('chart.ending').find((b) => b.textContent === 'B')!)
    expect(session.state.chart.ending).toBe(1)
    // Loop the B section: its bars of the first chorus.
    await click(tipped('chart.loop').find((b) => b.textContent?.trim() === 'B 9–16')!)
    expect(session.state.chart.loop).toEqual([8, 16])
    await click(tipped('chart.loop').find((b) => b.textContent === 'Off')!)
    expect(session.state.chart.loop).toBeNull()
    await click(tipped('chart.next')[0])
    expect(session.state.chart.selected).toEqual([0, 2])
    await click(tipped('chart.remove_playlist')[0])
    expect(session.state.chart.playlists).toHaveLength(0)
    expect(session.state.chart.on).toBe(false)
  })
})
