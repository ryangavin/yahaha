// The app bar's quick-nav strip: one button per panel and drawer, each with an Alt+letter
// shortcut (README's terminal keys use every plain letter). The buttons reuse the ui
// store's own open/close state; nothing here owns a panel.

import type { TipKey } from '../help/tooltips'
import { app, ui } from './store.svelte'

export interface NavItem {
  tip: TipKey
  label: string
  /** README notation, `alt+<letter>` (matched by physical key). */
  key: string
  open: () => boolean
  toggle: () => void
}

/** The Sound Browser without going through a part: it plays on the selected part (Right 1 if none). */
function toggleSounds() {
  if (ui.soundBrowser !== null) ui.soundBrowser = null
  else {
    const i = app.state.keyboardParts.findIndex((p) => p.selected)
    ui.soundBrowser = i < 0 ? 0 : i
  }
}

export const NAV: NavItem[] = [
  { tip: 'nav.sounds', label: 'Sounds', key: 'alt+b', open: () => ui.soundBrowser !== null, toggle: toggleSounds },
  { tip: 'nav.styles', label: 'Styles', key: 'alt+s', open: () => ui.browser, toggle: () => (ui.browser = !ui.browser) },
  { tip: 'nav.regist', label: 'Registrations', key: 'alt+r', open: () => ui.regist, toggle: () => ui.toggleDrawer('regist') },
  { tip: 'nav.rack', label: 'Rack', key: 'alt+o', open: () => ui.rack, toggle: () => ui.toggleDrawer('rack') },
  { tip: 'nav.multipad', label: 'Multi Pads', key: 'alt+p', open: () => ui.multipad, toggle: () => ui.toggleDrawer('multipad') },
  { tip: 'nav.effects', label: 'Effects', key: 'alt+e', open: () => ui.mixer, toggle: () => ui.toggleDrawer('mixer') },
  { tip: 'nav.mixer', label: 'Mixer', key: 'alt+m', open: () => ui.mixer, toggle: () => ui.toggleDrawer('mixer') },
  { tip: 'nav.looper', label: 'Looper', key: 'alt+l', open: () => ui.looper, toggle: () => ui.toggleDrawer('looper') },
  { tip: 'nav.charts', label: 'Charts', key: 'alt+c', open: () => ui.charts, toggle: () => ui.toggleDrawer('charts') },
  { tip: 'nav.harmony', label: 'Harmony/Arp', key: 'alt+h', open: () => ui.harmony, toggle: () => ui.toggleDrawer('harmony') },
  { tip: 'nav.library', label: 'Sound Library', key: 'alt+y', open: () => ui.sound, toggle: () => ui.toggleDrawer('sound') },
  { tip: 'nav.settings', label: 'Settings', key: 'alt+t', open: () => ui.settings, toggle: () => ui.toggleDrawer('settings') },
]
