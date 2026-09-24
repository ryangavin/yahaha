// The browser's own memory: favourites, recently loaded styles, the Preview-on-select
// switch and the last category. Client-side for now (the engine has no place for them).
//
// Stored with `localStorage`. Inside the Tauri app that is the webview's storage, which
// lives in the app's data directory and survives restarts; in a private browser window
// it simply isn't remembered. Styles are remembered by file path, since library ids can
// change when the folder is rescanned.

import { app } from '../../lib/store.svelte'
import { setOf, type Category } from './model'

const KEY = 'yahaha.browser.v1'
const MAX_RECENTS = 50

interface Stored {
  favourites: string[]
  recents: string[]
  autoPreview: boolean
  category: Category
}

/** Where the preferences are kept; swap for a Tauri store later without touching the UI. */
export interface PrefsStorage {
  load(): Partial<Stored> | null
  save(s: Stored): void
}

export const localStore: PrefsStorage = {
  load() {
    try {
      const raw = localStorage.getItem(KEY)
      return raw ? (JSON.parse(raw) as Partial<Stored>) : null
    } catch {
      return null
    }
  },
  save(s) {
    try {
      localStorage.setItem(KEY, JSON.stringify(s))
    } catch {
      /* private window or full storage: not remembered */
    }
  },
}

function validCategory(c: unknown): c is Category {
  if (!c || typeof c !== 'object') return false
  const k = (c as { kind?: unknown }).kind
  return k === 'all' || k === 'favourites' || k === 'recents' || (k === 'folder' && typeof (c as { path?: unknown }).path === 'string')
}

export class BrowserPrefs {
  favourites = $state.raw<ReadonlySet<string>>(setOf([]))
  recents = $state.raw<readonly string[]>([])
  autoPreview = $state(false)
  category = $state.raw<Category>({ kind: 'all' })

  constructor(private storage: PrefsStorage = localStore) {
    const s = storage.load()
    if (!s) return
    if (Array.isArray(s.favourites)) this.favourites = setOf(s.favourites.filter((p) => typeof p === 'string'))
    if (Array.isArray(s.recents)) this.recents = s.recents.filter((p) => typeof p === 'string').slice(0, MAX_RECENTS)
    this.autoPreview = s.autoPreview === true
    if (validCategory(s.category)) this.category = s.category
  }

  private save() {
    this.storage.save({
      favourites: [...this.favourites],
      recents: [...this.recents],
      autoPreview: this.autoPreview,
      category: this.category,
    })
  }

  toggleFavourite(path: string) {
    const had = this.favourites.has(path)
    this.favourites = setOf(had ? [...this.favourites].filter((p) => p !== path) : [...this.favourites, path])
    this.save()
  }

  /** A style was loaded (from anywhere): it goes to the top of Recent. */
  noteLoaded(path: string) {
    if (!path || this.recents[0] === path) return
    this.recents = [path, ...this.recents.filter((p) => p !== path)].slice(0, MAX_RECENTS)
    this.save()
  }

  setAutoPreview(on: boolean) {
    this.autoPreview = on
    this.save()
  }

  setCategory(c: Category) {
    this.category = c
    this.save()
  }
}

export const prefs = new BrowserPrefs()

// Recent follows every style change, whether it came from the browser, < Track / Track >,
// or the Launchkey, so it's kept up while the browser is closed too.
$effect.root(() => {
  $effect(() => {
    const path = app.state.style.path
    // The mock's initial state is a placeholder until a session attaches.
    if (app.kind) prefs.noteLoaded(path)
  })
})
