// The Sound Browser's tab and the instruments open in its Instruments tab. App-only state:
// it survives closing and reopening the browser.

import { SvelteSet } from 'svelte/reactivity'

export type BrowserTab = 'sounds' | 'instruments'

class BrowserNav {
  tab = $state<BrowserTab>('sounds')
  /** Instruments (`font:<file>`, `au:<component>`) whose presets are open. */
  open = new SvelteSet<string>()
}

export const browserNav = new BrowserNav()
