// Style racks in the mock (docs/racks.md "Styles and OTS"), as src/session/style_racks.rs
// keeps them: per style file name, which of OTS 1–4 load one of the user's racks instead of
// the style's own. Kept in memory.

import type { AppCmd, AppState, OtsRack, RackCmd } from './types'

type StyleRackCmd = Extract<AppCmd, { type: 'setOtsRack' | 'clearOtsRack' }>

export interface StyleRacksCtx {
  state: AppState
  message: (text: string, error?: boolean) => void
}

/** The loaded style's key: its file name. */
const styleKey = (st: AppState) => st.style.path.split('/').pop() ?? st.style.path

export class MockStyleRacks {
  private styles = new Map<string, (string | null)[]>()

  handles(cmd: AppCmd): cmd is StyleRackCmd {
    return cmd.type === 'setOtsRack' || cmd.type === 'clearOtsRack'
  }

  cmd(cmd: StyleRackCmd, ctx: StyleRacksCtx) {
    const st = ctx.state
    if (cmd.index < 0 || cmd.index >= Math.min(4, st.ots.settings.length)) return ctx.message(`${st.style.name} has no OTS ${cmd.index + 1}`, true)
    const id = cmd.type === 'setOtsRack' ? cmd.id : null
    const rack = id === null ? undefined : st.racks.find((r) => r.id === id)
    if (id !== null && !rack) return ctx.message(`no rack ${id}`, true)
    const key = styleKey(st)
    const slots = this.styles.get(key) ?? [null, null, null, null]
    slots[cmd.index] = id
    if (slots.every((s) => s === null)) this.styles.delete(key)
    else this.styles.set(key, slots)
    ctx.message(rack ? `With ${st.style.name} loaded, OTS ${cmd.index + 1} now loads ${rack.name}` : `OTS ${cmd.index + 1} is back to ${st.style.name}'s own`)
  }

  /** The rack OTS `index` of the loaded style loads, if one is chosen and still exists. */
  rackFor(st: AppState, index: number): string | null {
    const id = this.styles.get(styleKey(st))?.[index] ?? null
    return id !== null && st.racks.some((r) => r.id === id) ? id : null
  }

  /** Deleting a rack gives every OTS that loaded it back to its style. */
  afterRack(cmd: RackCmd, ok: boolean) {
    if (cmd.type !== 'deleteRack' || !ok) return
    for (const [key, slots] of this.styles) {
      slots.forEach((id, i) => id === cmd.id && (slots[i] = null))
      if (slots.every((s) => s === null)) this.styles.delete(key)
    }
  }

  /** `ots.racks`: per OTS of the loaded style. */
  racks(st: AppState): OtsRack[] {
    const slots = this.styles.get(styleKey(st))
    return st.ots.settings.slice(0, 4).map((_, i) => {
      const id = slots?.[i] ?? null
      const found = id === null ? undefined : st.racks.find((r) => r.id === id)
      return { rack: id, name: found?.name ?? '', missing: id !== null && !found }
    })
  }
}
