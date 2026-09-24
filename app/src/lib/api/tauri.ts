import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { Session } from './session'
import type { AppCmd, AppState, CmdError, LibraryList, Meters, SessionEvent } from './types'

/**
 * The app shell's session, wired as docs/app-api.md describes: commands `send`, `state`,
 * `library` and `meters`, and `yahaha` events. On `stateChanged` it fetches the state, at most
 * once per animation frame (several changes merge into one fetch; the state is always
 * complete).
 */
export class TauriSession implements Session {
  readonly kind = 'tauri' as const
  private subs = new Set<(s: AppState) => void>()
  private last: AppState | null = null
  private unlisten: UnlistenFn | null = null
  private pending = false

  static async connect(): Promise<TauriSession> {
    const s = new TauriSession()
    s.unlisten = await listen<SessionEvent>('yahaha', (e) => {
      if (e.payload.type === 'stateChanged') s.schedule()
    })
    s.emit(await invoke<AppState>('state'))
    return s
  }

  private schedule() {
    if (this.pending) return
    this.pending = true
    requestAnimationFrame(async () => {
      this.pending = false
      const st = await invoke<AppState>('state')
      // Fetches can resolve out of order: only a newer snapshot goes out.
      if (!this.last || st.version > this.last.version) this.emit(st)
    })
  }

  private emit(state: AppState) {
    this.last = state
    for (const f of this.subs) f(state)
  }

  subscribe(fn: (s: AppState) => void) {
    this.subs.add(fn)
    if (this.last) fn(this.last)
    return () => this.subs.delete(fn)
  }

  send(cmd: AppCmd) {
    invoke('send', { cmd }).catch((e: CmdError) => {
      // Refusals are in state.message; "busy" means nothing changed: one retry.
      if (e?.kind === 'busy') setTimeout(() => invoke('send', { cmd }).catch(() => {}), 5)
    })
  }

  library() {
    return invoke<LibraryList>('library')
  }

  meters() {
    return invoke<Meters>('meters')
  }

  dispose() {
    this.unlisten?.()
    this.subs.clear()
  }
}
