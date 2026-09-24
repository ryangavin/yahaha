// URL parameters for development and screenshots (the app shell never sets them):
//   ?mock          use the mock session even inside Tauri
//   ?demo=0        mock without the scripted demo (stopped, Sync Start armed)
//   ?theme=light   start in the light theme (not remembered)
//   ?help=1        start in help mode
//   ?tip=<key>     show the help-footer entry of the first control with that catalog key
//   ?open=browser|settings|parts|mixer|charts   open an overlay or drawer
//   ?shift=1       latch the Launchkey mirror's Shift layer
//   ?styles=N      mock: add N synthetic styles to the library (e.g. 60000; read in api/session.ts)
//   ?chart=1       mock: import the demo chart playlist, chart mode on (read in api/session.ts)

import { isTipKey } from '../help/tooltips'
import { ui } from './store.svelte'
import { tips } from './tooltip/tip.svelte'

export function applyUrlParams(search = location.search) {
  const p = new URLSearchParams(search)
  const theme = p.get('theme')
  if (theme === 'light' || theme === 'dark') ui.theme = theme
  if (p.get('help') === '1') tips.help = true
  const open = p.get('open')
  if (open === 'browser') ui.browser = true
  if (open === 'settings' || open === 'parts' || open === 'mixer' || open === 'charts') ui.toggleDrawer(open)
  if (p.get('shift') === '1') ui.shiftLatched = true
  const key = p.get('tip')
  if (key && isTipKey(key)) {
    setTimeout(() => {
      const el = document.querySelector<HTMLElement>(`[data-tip="${key}"]`)
      if (el) tips.show(el, key, true)
    }, 50)
  }
}
