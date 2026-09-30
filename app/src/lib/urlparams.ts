// URL parameters for development and screenshots (the app shell never sets them):
//   ?mock          use the mock session even inside Tauri
//   ?demo=0        mock without the scripted demo (stopped, Sync Start armed)
//   ?theme=light   start in the light theme (not remembered)
//   ?help=1        start in help mode
//   ?tip=<key>     show the help-footer entry of the first control with that catalog key
//   ?open=browser|settings|rack|mixer|charts|looper|multipad|harmony   open an overlay or drawer
//   ?open=library&tab=racks|sounds|instruments|map   Library on that tab, loading into Right 1
//                  (open=sounds: Library › Sounds; open=sound: Library › Style map; open=quick: Library › Racks)
//   ?shift=1       latch the Launchkey mirror's Shift layer
//   ?styles=N      mock: add N synthetic styles to the library (e.g. 60000; read in api/session.ts)
//   ?chart=1       mock: import the demo chart playlist, chart mode on (read in api/session.ts)
//   ?dropouts=N    mock: N audio dropouts after half a second, raising the dropout notice (api/session.ts)

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
  // Library (the Sounds modal, the Sound Library drawer and the Quick Racks drawer moved into it).
  const tab = p.get('tab')
  if (open === 'library') ui.openLibrary(tab === 'racks' || tab === 'sounds' || tab === 'instruments' || tab === 'map' ? tab : undefined, 0)
  if (open === 'sounds') ui.openLibrary('sounds', 0)
  if (open === 'sound') ui.openLibrary('map')
  if (open === 'quick') ui.openLibrary('racks')
  if (open === 'parts') ui.toggleDrawer('rack') // the Rack panel's old name
  if (open === 'mixer') ui.mixer = true // the mixer row's Details layer, not a drawer
  if (open === 'settings' || open === 'rack' || open === 'effects' || open === 'charts' || open === 'looper' || open === 'multipad' || open === 'harmony') ui.toggleDrawer(open)
  if (p.get('shift') === '1') ui.shiftLatched = true
  const key = p.get('tip')
  if (key && isTipKey(key)) {
    setTimeout(() => {
      const el = document.querySelector<HTMLElement>(`[data-tip="${key}"]`)
      if (el) tips.show(el, key, true)
    }, 50)
  }
}
