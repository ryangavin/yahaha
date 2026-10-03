import type { TabItem } from './types'

/** The app bar's display-page tabs, Stage … Harm/Arp. */
export const displayPageTabs: TabItem[] = [
  { id: 'stage', label: 'Stage', tip: 'view.stage' },
  { id: 'channel', label: 'Channel', tip: 'nav.channel' },
  { id: 'effects', label: 'Effects', tip: 'nav.effects' },
  { id: 'quickRacks', label: 'Quick Racks', tip: 'nav.quick' },
  { id: 'multiPads', label: 'Multi Pads', tip: 'nav.multipad' },
  { id: 'looper', label: 'Looper', tip: 'nav.looper' },
  { id: 'harmArp', label: 'Harm/Arp', tip: 'nav.harmony' },
]

/** The app bar's full-page tabs, after the separator. */
export const fullPageTabs: TabItem[] = [
  { id: 'library', label: 'Library', tip: 'view.library' },
  { id: 'settings', label: 'Settings', tip: 'nav.settings' },
]

/** The band header's fader page tabs (the master button). */
export const faderPageTabs: TabItem[] = [
  { id: 'panel', label: 'Panel', tip: 'mixer.page' },
  { id: 'style', label: 'Style', tip: 'mixer.page' },
]

/** The band header's fader layer tabs. */
export const layerTabs: TabItem[] = [
  { id: 'volume', label: 'Vol', name: 'Volume', tip: 'mixer.layer' },
  { id: 'pan', label: 'Pan', tip: 'mixer.layer' },
  { id: 'reverb', label: 'Reverb', name: 'Reverb send', tip: 'mixer.layer' },
  { id: 'chorus', label: 'Chorus', name: 'Chorus send', tip: 'mixer.layer' },
  { id: 'delay', label: 'Delay', name: 'Delay send', tip: 'mixer.layer' },
]
