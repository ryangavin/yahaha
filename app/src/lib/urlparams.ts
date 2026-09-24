// URL parameters for development and screenshots (the app shell never sets them):
//   ?mock          use the mock session even inside Tauri
//   ?demo=0        mock without the scripted demo (stopped, Sync Start armed)
//   ?theme=light   start in the light theme (not remembered)
//   ?help=1        start in help mode
//   ?tip=<key>     show the tooltip of the first control with that catalog key
//   ?open=browser|settings   open an overlay

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
  if (open === 'settings') ui.settings = true
  const key = p.get('tip')
  if (key && isTipKey(key)) {
    setTimeout(() => {
      const el = document.querySelector<HTMLElement>(`[data-tip="${key}"]`)
      if (el) tips.show(el, key, true)
    }, 50)
  }
}
