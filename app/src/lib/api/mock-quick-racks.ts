// Quick Racks in the mock (docs/racks.md), as src/session/quick_racks.rs runs them from
// the app: banks A–H of eight buttons naming the user's racks by id, the bank on view,
// Store, and a button waiting for the live rack to be saved. Kept in memory.

import { QUICK_BANKS, QUICK_SLOTS, bankLetter, quickLabel } from './quick-racks'
import type { AppCmd, AppState, QuickRackCmd, QuickRacksState, RackCmd } from './types'

export interface QuickCtx {
  state: AppState
  /** Runs a rack command (the guard included); true when it went through. */
  rack: (c: RackCmd) => boolean
  message: (text: string, error?: boolean) => void
}

const TYPES = new Set<string>(['pressQuickRack', 'stepQuickRackBank', 'toggleQuickRackStore', 'storeRack', 'clearQuickRack', 'stepQuickRack'])

export class MockQuickRacks {
  private banks: (string | null)[][] = Array.from({ length: QUICK_BANKS }, () => Array<string | null>(QUICK_SLOTS).fill(null))
  private bank = 0
  private store = false
  /** A button (bank, slot) waiting for the live rack to be saved. */
  private waiting: [number, number] | null = null

  handles(cmd: AppCmd): cmd is QuickRackCmd {
    return TYPES.has(cmd.type)
  }

  cmd(cmd: QuickRackCmd, ctx: QuickCtx) {
    switch (cmd.type) {
      case 'pressQuickRack': {
        // Slots 8 and 9 run on into the next bank's 1 and 2.
        const i = this.bank * QUICK_SLOTS + cmd.slot
        if (cmd.slot < 0 || cmd.slot >= 10 || i >= QUICK_BANKS * QUICK_SLOTS) return ctx.message(`no Quick Rack ${cmd.slot + 1}`, true)
        const [bank, slot] = [Math.floor(i / QUICK_SLOTS), i % QUICK_SLOTS]
        if (this.store) return this.storeOn(bank, slot, ctx)
        return this.load(bank, slot, !!cmd.discard, ctx)
      }
      case 'stepQuickRackBank':
        this.bank = Math.max(0, Math.min(QUICK_BANKS - 1, this.bank + Math.sign(cmd.delta)))
        return
      case 'toggleQuickRackStore':
        this.store = !this.store
        this.waiting = null
        return
      case 'storeRack':
        // One step, no arming (`capture_quick`); Store armed is cleared.
        if (cmd.slot < 0 || cmd.slot >= QUICK_SLOTS) return ctx.message(`no Quick Rack ${cmd.slot + 1}`, true)
        this.store = false
        this.waiting = null
        return this.capture(this.bank, cmd.slot, ctx)
      case 'clearQuickRack':
        if (cmd.bank < 0 || cmd.bank >= QUICK_BANKS || cmd.slot < 0 || cmd.slot >= QUICK_SLOTS) return ctx.message(`no Quick Rack ${cmd.bank}:${cmd.slot}`, true)
        this.banks[cmd.bank][cmd.slot] = null
        return
      case 'stepQuickRack': {
        const bank = this.bank
        const stored = [...Array(QUICK_SLOTS).keys()].filter((s) => this.banks[bank][s] !== null)
        const live = ctx.state.liveRack.id
        const lit = live === null ? -1 : stored.findIndex((s) => this.banks[bank][s] === live)
        const d = Math.sign(cmd.delta)
        const to = d === 0 ? undefined : lit < 0 ? (d > 0 ? stored[0] : stored[stored.length - 1]) : stored[lit + d]
        if (to !== undefined) return this.load(bank, to, !!cmd.discard, ctx)
        if (!stored.length) ctx.message(`Bank ${bankLetter(bank)} has no racks`, true)
        return
      }
    }
  }

  /** After a rack command (`quick_after_rack_cmd`): a waiting button takes the saved rack,
   * deleting a rack empties its buttons, loading another rack or dismissing the prompt lets
   * a waiting Store go. */
  afterRack(cmd: RackCmd, ok: boolean, ctx: QuickCtx) {
    const live = ctx.state.liveRack
    if ((cmd.type === 'saveRack' || cmd.type === 'saveRackAs') && ok) {
      if (this.waiting && live.id) this.put(this.waiting[0], this.waiting[1], live.id, ctx)
    } else if (cmd.type === 'deleteRack' && ok) {
      for (const b of this.banks) b.forEach((id, s) => id === cmd.id && (b[s] = null))
    } else if ((cmd.type === 'loadRack' || cmd.type === 'newRack' || cmd.type === 'revertRack') && ok) {
      this.waiting = null
    } else if (cmd.type === 'dismissRackPrompt') {
      this.waiting = null
    }
  }

  state(st: AppState): QuickRacksState {
    const live = st.liveRack.id
    const w = this.waiting
    return {
      bank: this.bank,
      buttons: this.banks[this.bank].map((id) => {
        const found = id === null ? undefined : st.racks.find((r) => r.id === id)
        return { rack: id, name: found?.name ?? '', missing: id !== null && !found, loaded: id !== null && id === live }
      }),
      store: this.store,
      storeWaiting: w && w[0] === this.bank ? w[1] : null,
      readOnly: false,
    }
  }

  private load(bank: number, slot: number, discard: boolean, ctx: QuickCtx) {
    const label = quickLabel(bank, slot)
    const id = this.banks[bank][slot]
    if (id === null) return ctx.message(`Quick Rack ${label} is empty`, true)
    if (!ctx.state.racks.some((r) => r.id === id)) return ctx.message(`Quick Rack ${label}'s rack is gone`, true)
    this.waiting = null
    ctx.rack({ type: 'loadRack', id, ...(discard ? { discard } : {}) })
  }

  private storeOn(bank: number, slot: number, ctx: QuickCtx) {
    const live = ctx.state.liveRack
    const saved = live.id !== null && !live.modified && ctx.state.racks.some((r) => r.id === live.id)
    if (saved) return this.put(bank, slot, live.id!, ctx)
    this.waiting = [bank, slot]
    ctx.message(`Save the rack first; then it goes on Quick Rack ${quickLabel(bank, slot)}`)
  }

  /** `storeRack`: the live rack, saved as it goes, on button (`bank`, `slot`)
   * (`capture_quick`). The lit button's rack takes the live rack's changes; elsewhere a
   * saved rack with no changes goes on as it is, and anything else is saved as a new rack
   * named from the sounds of the parts that are on. */
  private capture(bank: number, slot: number, ctx: QuickCtx) {
    const st = ctx.state
    const live = st.liveRack
    const own = live.id !== null && st.racks.some((r) => r.id === live.id) ? live.id : null
    const lit = own !== null && this.banks[bank][slot] === own
    if (own !== null && !live.modified) return this.put(bank, slot, own, ctx)
    if (own !== null && lit) {
      if (!this.saveLive(null, ctx)) return
      return this.put(bank, slot, own, ctx)
    }
    const seen = new Set<string>()
    const names = st.keyboardParts
      .filter((p) => p.on)
      .map((p) => p.voiceName.trim())
      .filter((n) => n && !seen.has(n.toLowerCase()) && seen.add(n.toLowerCase()))
    const base = names.length ? names.join(' + ') : 'New rack'
    const taken = (n: string) => st.racks.some((r) => r.name.toLowerCase() === n.toLowerCase())
    let name = base
    for (let n = 2; taken(name); n++) name = `${base} ${n}`
    if (!this.saveLive(name, ctx) || st.liveRack.id === null) return
    this.put(bank, slot, st.liveRack.id, ctx)
  }

  /** Save the live rack (`saveRack`, or `saveRackAs` with a name) with no dialog: edited
   * sounds take their suggested names and a prompt up is dismissed (`save_live`). */
  private saveLive(saveAs: string | null, ctx: QuickCtx): boolean {
    const st = ctx.state
    st.liveRack.prompt = null
    const soundNames: Record<number, string> = {}
    st.keyboardParts.forEach((p, part) => {
      if (p.soundEdited) soundNames[part] = p.sound?.name ?? p.voiceName
    })
    return ctx.rack(saveAs === null ? { type: 'saveRack', soundNames } : { type: 'saveRackAs', name: saveAs, soundNames })
  }

  private put(bank: number, slot: number, id: string, ctx: QuickCtx) {
    this.store = false
    this.waiting = null
    this.banks[bank][slot] = id
    ctx.message(`Stored ${ctx.state.liveRack.name} on Quick Rack ${quickLabel(bank, slot)}`)
  }
}
