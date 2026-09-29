// The rack switching guard's answers (docs/racks.md), shared by the Quick Racks bar and
// drawer. A rack switch the engine refused with unsaved changes (a Quick Rack press, a rack
// loaded from the drawer, Rack −/+) waits in `liveRack.prompt.then`:
//
// - Save first: `saveRack`, then the same switch once the save has gone through (the live
//   rack unmodified, no prompt). A save that asks for sound names waits for them.
// - Discard and switch: the same switch with `discard`.
// - Keep editing: `dismissRackPrompt`.
//
// The switch is sent as `loadRack` of the rack the prompt names: a Quick Rack press loads
// its button's rack exactly as `loadRack` does (src/session/quick_racks.rs `load_quick`),
// so this is the same press, wherever it came from (the bar, a key, the mirror's pads).

import type { AppCmd, AppState, RackSwitch } from '../../lib/api/types'
import { app } from '../../lib/store.svelte'

/** The command that makes switch `then`. */
export function switchCmd(then: RackSwitch, discard = false): AppCmd {
  const d = discard ? { discard } : {}
  return then.kind === 'load' ? { type: 'loadRack', id: then.id, ...d } : { type: 'newRack', ...d }
}

class Guard {
  /** Save first: the switch to send once the save has gone through. */
  afterSave = $state.raw<AppCmd | null>(null)

  saveFirst(then: RackSwitch) {
    this.afterSave = switchCmd(then)
    app.send({ type: 'saveRack' })
  }

  discard(then: RackSwitch) {
    this.afterSave = null
    app.send(switchCmd(then, true))
  }

  /** Keep editing (or cancel the sound names): nothing is saved or switched. */
  keepEditing() {
    this.afterSave = null
    app.send({ type: 'dismissRackPrompt' })
  }

  /** On every state: once a Save first has gone through, send the switch it was for. */
  settle(st: AppState) {
    const c = this.afterSave
    if (!c || st.liveRack.modified || st.liveRack.prompt !== null) return
    this.afterSave = null
    app.send(c)
  }
}

export const guard = new Guard()
