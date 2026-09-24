// The Settings drawer's pages, grouped the way the Genos menus group them, and which one
// is showing. App-only state: it survives closing and reopening the drawer. `?tab=<id>`
// opens a page (with `?open=settings`, for screenshots).

import type { TipKey } from '../../help/tooltips'

export type SettingsTab = 'chord' | 'split' | 'transpose' | 'style' | 'audio' | 'midi' | 'library'

export const TABS: { id: SettingsTab; label: string; tip: TipKey; genos: string | null }[] = [
  { id: 'chord', label: 'Chord', tip: 'settings.tab.chord', genos: 'Split & Fingering' },
  { id: 'split', label: 'Split', tip: 'settings.tab.split', genos: 'Split & Fingering' },
  { id: 'transpose', label: 'Transpose', tip: 'settings.tab.transpose', genos: 'Transpose' },
  { id: 'style', label: 'Style', tip: 'settings.tab.style', genos: 'Style Setting' },
  { id: 'audio', label: 'Audio', tip: 'settings.tab.audio', genos: null },
  { id: 'midi', label: 'MIDI', tip: 'settings.tab.midi', genos: 'MIDI' },
  { id: 'library', label: 'Library', tip: 'settings.tab.library', genos: null },
]

function fromUrl(): SettingsTab {
  try {
    const t = new URLSearchParams(location.search).get('tab')
    return TABS.find((x) => x.id === t)?.id ?? 'chord'
  } catch {
    return 'chord'
  }
}

class SettingsNav {
  tab = $state<SettingsTab>(fromUrl())
}

export const nav = new SettingsNav()
