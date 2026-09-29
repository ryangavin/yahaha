// The app bar's quick-nav strip: one button per panel and drawer, each with an Alt+letter
// shortcut (README's terminal keys use every plain letter). The buttons reuse the ui
// store's own open/close state; nothing here owns a panel.

import type { TipKey } from '../help/tooltips'
import { app, ui, type LibraryTab } from './store.svelte'

export interface NavItem {
  tip: TipKey
  label: string
  /** README notation, `alt+<letter>` (matched by physical key). */
  key: string
  open: () => boolean
  toggle: () => void
  /** A shortcut only, with no button on the strip (its tooltip is on the control it opens). */
  hidden?: boolean
}

/** The part Library loads into when it opens from here: the selected one (Right 1 if none). */
function selectedPart(): number {
  const i = app.state.keyboardParts.findIndex((p) => p.selected)
  return i < 0 ? 0 : i
}

/** Stage ↔ Library (docs/racks.md, "Screens"). From the stage it opens on the last tab
 * (Sounds at first), loading into the selected part; `tab` opens that tab instead. */
export function toggleLibrary(tab?: LibraryTab) {
  if (ui.view === 'library' && (!tab || ui.libraryTab === tab)) ui.view = 'stage'
  else ui.openLibrary(tab, ui.view === 'library' ? undefined : selectedPart())
}

export const NAV: NavItem[] = [
  { tip: 'nav.library', label: 'Library', key: 'alt+b', open: () => ui.view === 'library', toggle: () => toggleLibrary() },
  { tip: 'nav.styles', label: 'Styles', key: 'alt+s', open: () => ui.browser, toggle: () => (ui.browser = !ui.browser) },
  { tip: 'nav.regist', label: 'Registrations', key: 'alt+r', open: () => ui.regist, toggle: () => ui.toggleDrawer('regist') },
  { tip: 'nav.rack', label: 'Rack', key: 'alt+o', open: () => ui.rack, toggle: () => ui.toggleDrawer('rack') },
  { tip: 'nav.multipad', label: 'Multi Pads', key: 'alt+p', open: () => ui.multipad, toggle: () => ui.toggleDrawer('multipad') },
  { tip: 'nav.effects', label: 'Effects', key: 'alt+e', open: () => ui.mixer, toggle: () => ui.toggleDrawer('mixer') },
  { tip: 'nav.mixer', label: 'Mixer', key: 'alt+m', open: () => ui.mixer, toggle: () => ui.toggleDrawer('mixer') },
  { tip: 'nav.looper', label: 'Looper', key: 'alt+l', open: () => ui.looper, toggle: () => ui.toggleDrawer('looper') },
  { tip: 'nav.charts', label: 'Charts', key: 'alt+c', open: () => ui.charts, toggle: () => ui.toggleDrawer('charts') },
  { tip: 'nav.harmony', label: 'Harmony/Arp', key: 'alt+h', open: () => ui.harmony, toggle: () => ui.toggleDrawer('harmony') },
  // Was the Sound Library drawer; its GM map is Library's Style map tab now.
  { tip: 'library.tab_map', label: 'Style map', key: 'alt+y', open: () => ui.view === 'library' && ui.libraryTab === 'map', toggle: () => toggleLibrary('map'), hidden: true },
  { tip: 'nav.settings', label: 'Settings', key: 'alt+t', open: () => ui.settings, toggle: () => ui.toggleDrawer('settings') },
]
