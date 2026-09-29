// Whether the display shows the Channel view (ChannelView.svelte) for `ui.selectedPart`.
// Clicking a mixer strip's name opens it on that part; clicking the selected strip again,
// Esc (once nothing is open over the stage) or its × closes it, and the display shows what
// it showed before (the lead sheet).

import { app, ui } from '../../lib/store.svelte'

class ChannelNav {
  open = $state(false)

  /** Show part `part` (0–11); a keyboard part becomes the selected part too, as its strip's name does. */
  show(part: number) {
    ui.selectedPart = part
    if (part < 4) app.send({ type: 'selectPart', part })
    this.open = true
  }

  close() {
    this.open = false
  }

  /** A click anywhere in the mixer row, seen before the strip handles it: a strip's name
   * opens its channel, or closes it when that strip's channel is already shown. */
  stripClick(e: MouseEvent) {
    const target = e.target instanceof Element ? e.target : null
    const name = target?.closest('[data-tip="mixer.strip.select"]')
    const part = Number(name?.closest<HTMLElement>('[data-part]')?.dataset.part)
    if (!name || !Number.isInteger(part)) return
    this.open = !(this.open && ui.selectedPart === part)
  }

  /** Esc once nothing else took it: closes the channel. True when it did. */
  escape(): boolean {
    if (!this.open) return false
    this.open = false
    return true
  }
}

export const channelNav = new ChannelNav()
