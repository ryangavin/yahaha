import type { AppCmd, AppState, LibraryList, Meters } from './types'

/**
 * The one seam between the UI and the engine. Components only see this interface (via
 * the stores in `lib/store.svelte.ts`), so the mock and the real engine behind Tauri are
 * interchangeable.
 */
export interface Session {
  readonly kind: 'mock' | 'tauri'
  /** Called with every new state (up to ~60 times a second while playing). Returns unsubscribe. */
  subscribe(fn: (s: AppState) => void): () => void
  /** Runs a command. Refusals show up in `state.message` too. */
  send(cmd: AppCmd): void
  /** The style library in display order (folder, then name). Re-fetch when
   * `state.library.revision` changes. */
  library(): Promise<LibraryList>
  /** Output levels since the last call (one reader: poll at display rate, apply your own
   * decay and peak hold). No channels without the synth. */
  meters(): Promise<Meters>
  /** Open (or focus) / close keyboard part `part`'s plugin editor window. The app shell
   * opens it on its main thread; the browser mock can't (it says so). */
  pluginEditor?(part: number, open: boolean): void
  dispose(): void
}

function inTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

/** Tauri when running inside the app shell, the mock in a plain browser (or with `?mock`). */
export async function connect(): Promise<Session> {
  const params = new URLSearchParams(typeof location !== 'undefined' ? location.search : '')
  if (inTauri() && !params.has('mock')) {
    const { TauriSession } = await import('./tauri')
    return TauriSession.connect()
  }
  const { MockSession } = await import('./mock')
  return new MockSession({ demo: params.get('demo') !== '0', styles: Number(params.get('styles')) || 0, chart: params.get('chart') === '1' })
}
