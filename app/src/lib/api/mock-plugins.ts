// Instrument plugins as the engine runs them (src/session/plugins.rs): a part's plugin
// loads in the background (`loading`, with its stage), then plays; the list comes from the
// cached scan. The mock lists Apple's built-in instruments (every Mac has them), one
// made-up third-party synth that always fails to load, to show the error path, and one
// the system refuses to host out of process, so it falls back to loading in process.

import type { AppState, KeyboardPart, PluginCmd, PluginEntry, PluginsState } from './types'

export const MOCK_PLUGINS: PluginEntry[] = [
  { id: 'aumu dls  appl', name: 'DLSMusicDevice', manufacturer: 'Apple', version: '1.0.0', format: 'AUv2', lastError: null, inProcess: false, canRunInProcess: true },
  { id: 'aumu samp appl', name: 'AUSampler', manufacturer: 'Apple', version: '1.0.0', format: 'AUv2', lastError: null, inProcess: false, canRunInProcess: true },
  { id: 'aumu Mock Demo', name: 'Broken Synth', manufacturer: 'Example Audio', version: '0.9.0', format: 'AUv3', lastError: 'timed out after 20.0 s', inProcess: false, canRunInProcess: false },
  { id: 'aumu Tiny Demo', name: 'Tiny Synth', manufacturer: 'Example Audio', version: '0.9.0', format: 'AUv2', lastError: null, inProcess: false, canRunInProcess: true },
]

/** The mock plugin the system won't host out of process: it loads in process instead. */
export const MOCK_FALLBACK_ID = 'aumu Tiny Demo'

/** The mock plugin that plays heavy: a high CPU share and a few slow renders. */
export const MOCK_HEAVY_ID = 'aumu samp appl'

export function initialPlugins(): PluginsState {
  return { available: true, scanning: false, list: MOCK_PLUGINS.map((p) => ({ ...p })) }
}

/** How long a mock load and a rescan take. */
const LOAD_MS = 600
const RESCAN_MS = 800
const STAGES = ['queued', 'instantiating', 'initializing']

export class MockPlugins {
  /** Milliseconds into each part's load (-1: none). */
  private loading = [-1, -1, -1, -1]
  private scanLeft = 0

  constructor(
    private state: () => AppState,
    private say: (text: string, error: boolean) => void,
  ) {}

  cmd(cmd: PluginCmd) {
    const st = this.state()
    switch (cmd.type) {
      case 'setPartPlugin': {
        if (!st.plugins.available) return this.say('plugins play through the built-in synth, which is off', true)
        const e = st.plugins.list.find((p) => p.id === cmd.id)
        if (!e) return this.say(`no instrument Audio Unit ${cmd.id} is installed`, true)
        const part = st.keyboardParts[cmd.part & 3]
        part.plugin = {
          id: e.id,
          name: e.name,
          manufacturer: e.manufacturer,
          status: 'loading',
          stage: 'queued',
          error: null,
          outOfProcess: e.manufacturer !== 'Apple' && !e.inProcess,
          inProcessFallback: false,
          cpu: 0,
          overruns: 0,
          recentOverruns: 0,
          editor: false,
        }
        this.loading[cmd.part & 3] = 0
        break
      }
      case 'clearPartPlugin':
        delete st.keyboardParts[cmd.part & 3].plugin
        this.loading[cmd.part & 3] = -1
        break
      case 'savePartPluginState': {
        const p = st.keyboardParts[cmd.part & 3].plugin
        if (!p || p.status !== 'playing') this.say('the plugin is not playing yet', true)
        break
      }
      case 'rescanPlugins':
        st.plugins.scanning = true
        this.scanLeft = RESCAN_MS
        break
      case 'setPluginInProcess': {
        const e = st.plugins.list.find((p) => p.id === cmd.id)
        if (!e) return this.say(`no instrument Audio Unit ${cmd.id} is installed`, true)
        if (cmd.inProcess && !e.canRunInProcess) return this.say(`${e.manufacturer}: ${e.name} is an AUv3 that only runs out of process`, true)
        e.inProcess = cmd.inProcess
        if (st.keyboardParts.some((k) => k.plugin?.id === cmd.id && k.plugin.status === 'playing')) {
          this.say(`${e.name} runs ${cmd.inProcess ? 'inside yahaha' : 'in its own process'} from its next load (the next start, or pick it again)`, false)
        }
        break
      }
    }
  }

  step(ms: number) {
    const st = this.state()
    if (this.scanLeft > 0) {
      this.scanLeft -= ms
      if (this.scanLeft <= 0) st.plugins.scanning = false
    }
    st.keyboardParts.forEach((part: KeyboardPart, i) => {
      const p = part.plugin
      if (!p) return
      if (this.loading[i] >= 0) {
        this.loading[i] += ms
        const t = this.loading[i]
        if (t < LOAD_MS) {
          p.stage = STAGES[Math.min(STAGES.length - 1, Math.floor((t / LOAD_MS) * STAGES.length))]
          return
        }
        this.loading[i] = -1
        p.stage = null
        const e = st.plugins.list.find((x) => x.id === p.id)
        if (e?.lastError) {
          p.status = 'failed'
          p.error = e.lastError
          this.say(`${p.name} didn't load: ${e.lastError}`, true)
        } else {
          if (p.id === MOCK_FALLBACK_ID && !e?.inProcess) {
            p.outOfProcess = false
            p.inProcessFallback = true
            this.say(`${p.name} can't run in its own process; loading it inside yahaha instead (if it crashes, yahaha goes with it)`, false)
          }
          p.status = 'playing'
          p.editor = true
          p.cpu = p.id === MOCK_HEAVY_ID ? 0.31 : 0.012
          if (p.id === MOCK_HEAVY_ID) p.overruns = p.recentOverruns = 4
        }
      }
    })
  }
}
