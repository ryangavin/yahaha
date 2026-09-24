// The browser mock's iReal chart player (#89): a synthetic demo playlist (made-up
// progressions, no real songs) and the chart state the engine would send for it. The mock
// can't decode iReal links (the engine does: src/ireal); `importCharts` adds the demo
// playlist instead. `MockSession` plays it bar by bar (mock.ts).

import type { ChartBar, ChartSection, ChartSong, ChartSongInfo, ChartState } from './types'

interface DemoSong extends ChartSongInfo {
  time: [number, number]
  /** Sections in order: [label, bars], bars `|`-separated, chords in a bar space-separated. */
  form: [string, string][]
}

const A = 'Bb6 G7|Cm7 F7|Bb6 G7|Cm7 F7|Fm7 Bb7|Eb7 Ab7|Dm7 G7|Cm7 F7'

/** Made-up progressions in common forms, named as demos. */
export const DEMO_SONGS: DemoSong[] = [
  {
    title: 'Demo AABA', composer: 'Demo Composer', style: 'Medium Swing', key: 'Bb', tempo: 160, time: [4, 4],
    form: [['A', A], ['A', A], ['B', 'D7|D7|G7|G7|C7|C7|F7|F7'], ['A', A]],
  },
  {
    title: 'Demo Bossa', composer: 'Demo Composer', style: 'Bossa Nova', key: 'C', tempo: 140, time: [4, 4],
    form: [['A', 'Cmaj7|Cmaj7|Dm7|G7|Em7|A7|Dm7|G7'], ['B', 'Fmaj7|Fm7 Bb7|Em7|A7|Dm7|G7|Cmaj7|Cmaj7']],
  },
  {
    title: 'Demo Blues', composer: 'Demo Composer', style: 'Medium Up Swing', key: 'F', tempo: 132, time: [4, 4],
    form: [['A', 'F7|Bb7|F7|Cm7 F7|Bb7|Bdim7|F7|Am7 D7|Gm7|C7|F7 D7|Gm7 C7']],
  },
  {
    title: 'Demo Waltz', composer: 'Demo Composer', style: 'Jazz Waltz', key: 'G', tempo: 150, time: [3, 4],
    form: [['A', 'Gmaj7|Em7|Am7|D7|Bm7|E7|Am7|D7'], ['B', 'Cmaj7|Cm6|Bm7|E7|Am7|D7|Gmaj7|Gmaj7']],
  },
]

export const DEMO_PLAYLIST = 'Demo playlist (mock)'

export function info(s: DemoSong): ChartSongInfo {
  return { title: s.title, composer: s.composer, style: s.style, key: s.key, tempo: s.tempo }
}

/** The form played `choruses` times, as the engine expands it. */
export function songState(s: DemoSong, choruses: number): ChartSong {
  const bars: ChartBar[] = []
  for (let chorus = 1; chorus <= choruses; chorus++) {
    for (const [label, text] of s.form) {
      text.split('|').forEach((bar, i) => {
        const names = bar.trim().split(/\s+/).filter(Boolean)
        const step = Math.max(1, Math.floor(s.time[0] / Math.max(1, names.length)))
        bars.push({
          section: label,
          sectionStart: i === 0,
          main: Math.max(0, 'ABCD'.indexOf(label)),
          time: s.time,
          chorus,
          chords: names.map((name, k) => ({ beat: k * step, name })),
        })
      })
    }
  }
  const sections: ChartSection[] = []
  bars.forEach((b, i) => {
    const last = sections[sections.length - 1]
    if (last && !b.sectionStart) last.bars++
    else sections.push({ label: b.section ?? '', chorus: b.chorus, start: i, bars: 1 })
  })
  return { ...info(s), bars, sections }
}

export function emptyChart(): ChartState {
  return {
    on: false, playlists: [], selected: null, song: null, choruses: 1, intro: 0, ending: 0, loop: null,
    autoStyle: true, suggestedStyle: null, bar: null, overridden: false,
  }
}

/** The bar after `i` (following the loop), or null after the last. */
export function nextBar(c: ChartState, i: number): number | null {
  const n = c.song?.bars.length ?? 0
  if (c.loop && c.loop[0] < c.loop[1] && c.loop[1] <= n && i + 1 >= c.loop[1]) return c.loop[0]
  return i + 1 < n ? i + 1 : null
}

/** The chord in effect at `beat` of bar `i` (held from earlier bars when it has none). */
export function chordAt(song: ChartSong, i: number, beat: number): string | null {
  for (let b = i; b >= 0; b--) {
    const chords = song.bars[b].chords.filter((c) => b < i || c.beat <= beat)
    if (chords.length) return chords[chords.length - 1].name
  }
  return null
}

/** Words the mock matches a style label to library styles with (the engine has a fuller table). */
export function styleWords(label: string): string[] {
  const l = label.toLowerCase()
  const words: string[] = []
  for (const [k, w] of [['bossa', ['bossa', 'latin']], ['waltz', ['waltz']], ['swing', ['swing', 'jazz']], ['blues', ['blues']], ['latin', ['latin']]] as const) {
    if (l.includes(k)) words.push(...w)
  }
  return words
}
