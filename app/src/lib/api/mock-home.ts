// The browser mock's Home screen data (`state.home`, as src/session/home.rs builds it).
// The mock has no patterns, so each Main present gets a made-up 4-bar groove; the rest
// comes from the state. app/src-tauri/src/mock_home.rs does the same.

import { snapshotLabel } from './registration'
import type { AppState, HomeState } from './types'

const LETTERS = ['A', 'B', 'C', 'D']
const lane = (steps: number[], vel: number) => Array.from({ length: 16 }, (_, i) => (steps.includes(i) ? vel : 0))

export function mockHome(st: AppState): HomeState {
  const t = st.transport
  const has = (n: string) => st.style.sections.includes(n)
  const mains = LETTERS.map((l, i) => {
    const name = `Main ${l}`
    const fill = `Fill In ${l}${l}`
    const present = has(name)
    const bars = present ? 4 : 0
    return {
      name,
      present,
      bars,
      stepsPerBar: 16,
      density: present ? Array.from({ length: bars * 16 }, (_, s) => [3, 0, 1, 0, 2, 0, 2, 0][s % 8] + (i & 1)) : [],
      lanes: present
        ? { kick: lane([0, 8], 110), snare: lane([4, 12], 100), hats: lane([0, 2, 4, 6, 8, 10, 12, 14], 70), bass: lane([0, 6, 8, 14], 96) }
        : { kick: [], snare: [], hats: [], bass: [] },
      fill: { name: fill, present: has(fill), bars: has(fill) ? 1 : 0, active: t.section === fill || t.queued === fill },
      current: t.main === i,
    }
  })
  const n = t.sectionBars
  const b = Math.max(1, t.beatsPerBar)
  const fraction = t.running && n ? Math.min(1, Math.max(0, ((Math.max(1, t.bar) - 1) * b + Math.max(1, t.beat) - 1) / (n * b))) : 0
  const r = st.registration
  const sel = r.selected === null ? undefined : r.buttons[r.selected]
  const snapshot =
    r.selected !== null && sel?.stored
      ? { index: r.selected, label: snapshotLabel(r.selected), name: sel.name || sel.style || `Snapshot ${snapshotLabel(r.selected)}`, bank: r.bank.name }
      : null
  const o = st.ots.settings[st.ots.applied - 1]
  return {
    mains,
    progress: { running: t.running, bar: t.bar, beat: t.beat, bars: n, beatsPerBar: t.beatsPerBar, fraction },
    snapshot,
    ots: st.ots.applied > 0 && o ? { index: st.ots.applied - 1, name: o.name } : null,
    bandSends: st.effects.blocks.map((x) => ({ block: x.block, name: x.name, effectName: x.effectName, level: x.bandSend })),
  }
}
