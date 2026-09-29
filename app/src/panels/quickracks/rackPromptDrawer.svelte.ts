// The rack prompts (`liveRack.prompt`: unsaved changes, sound names) are asked in one
// place, the Rack panel. Whatever shows the Quick Racks (the stage's knob and rack panel,
// Library's bar) calls this while it's mounted: a prompt that appears on Stage opens the
// Rack drawer so it's visible (in Library the Rack panel is docked already). Only on its
// appearance, so closing the drawer sticks.

import { untrack } from 'svelte'
import { app, ui } from '../../lib/store.svelte'

export function openRackDrawerOnPrompt() {
  let asked = false
  $effect(() => {
    const now = app.state.liveRack.prompt !== null
    if (now && !asked && ui.view !== 'library' && !ui.rack) untrack(() => ui.toggleDrawer('rack'))
    asked = now
  })
}
