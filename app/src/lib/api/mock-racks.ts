// The rack commands in the mock (docs/app-api.md › Racks), kept in memory: new, load,
// save, save as, revert, rename, duplicate, delete, the unsaved-changes guard and the
// sound-names prompt, as the session does them.

import type { AppCmd, AppState, KeyboardPart, RackCmd, RackEntry, RackSwitch } from './types'

/** What a mock rack holds: each part's switch, voice, own patch and mix, the split, the transpose. */
interface MockRack {
  id: string
  name: string
  parts: Pick<KeyboardPart, 'on' | 'program' | 'patch' | 'volume' | 'octave' | 'pan' | 'reverb' | 'chorus' | 'variation'>[]
  names: string[]
  split: number
  transpose: number
}

export interface RackCtx {
  state: AppState
  command: (c: AppCmd) => void
  message: (text: string, error?: boolean) => void
  /** The live rack is now what plays, unmodified (the next publish sees no change). */
  clean: () => void
}

const RACK_TYPES = new Set<string>(['newRack', 'loadRack', 'saveRack', 'saveRackAs', 'revertRack', 'renameRack', 'duplicateRack', 'deleteRack', 'dismissRackPrompt'])
const NEW_NAME = 'New rack'
const DEFAULT_PROGRAMS = [0, 48, 61, 48]

export class MockRacks {
  private racks: MockRack[] = []
  private seq = 0

  handles(cmd: AppCmd): cmd is RackCmd {
    return RACK_TYPES.has(cmd.type)
  }

  /** The racks list, by name. */
  entries(): RackEntry[] {
    return [...this.racks]
      .sort((a, b) => a.name.localeCompare(b.name))
      .map((r) => ({ id: r.id, name: r.name, parts: [...r.names], on: r.parts.map((p) => p.on), needsAttention: false }))
  }

  cmd(cmd: RackCmd, ctx: RackCtx) {
    const live = ctx.state.liveRack
    const fail = (text: string) => ctx.message(text, true)
    const find = (id: string) => this.racks.find((r) => r.id === id)
    const taken = (name: string) => this.racks.some((r) => r.name === name)
    switch (cmd.type) {
      case 'newRack':
      case 'loadRack': {
        const rack = cmd.type === 'loadRack' ? find(cmd.id) : null
        if (cmd.type === 'loadRack' && !rack) return fail(`no rack ${cmd.id}`)
        if (live.modified && !cmd.discard) {
          const then: RackSwitch = rack ? { kind: 'load', id: rack.id, name: rack.name } : { kind: 'new' }
          live.prompt = { kind: 'unsavedChanges', then }
          return
        }
        this.enter(rack ?? null, ctx)
        return
      }
      case 'saveRack':
      case 'saveRackAs': {
        let name: string
        let id: string
        const own = live.id ? find(live.id) : undefined
        if (cmd.type === 'saveRackAs') {
          name = cmd.name.trim()
          if (!name) return fail('a rack needs a name')
          if (taken(name)) return fail(`there is a rack called ${name} already`)
          id = this.newId()
        } else if (own) {
          ;({ name, id } = own)
        } else {
          name = this.unique(live.name)
          id = this.newId()
        }
        // Edited sounds: the user's own saved over, presets need a name.
        const names = cmd.soundNames ?? {}
        const ask: { part: number; suggested: string }[] = []
        const saves: AppCmd[] = []
        ctx.state.keyboardParts.forEach((p, part) => {
          if (!p.soundEdited) return
          if (p.sound?.id.startsWith('saved:')) return void saves.push({ type: 'saveSound', part })
          const n = names[part]?.trim()
          if (n) saves.push({ type: 'saveSoundAs', part, name: n })
          else ask.push({ part, suggested: p.sound?.name ?? p.voiceName })
        })
        if (ask.length) {
          live.prompt = { kind: 'soundNames', parts: ask, saveAs: cmd.type === 'saveRackAs' ? name : null }
          return
        }
        for (const c of saves) ctx.command(c)
        const rack = this.capture(ctx.state, id, name)
        this.racks = [...this.racks.filter((r) => r.id !== id), rack]
        Object.assign(live, { name, id, modified: false, prompt: null })
        ctx.clean()
        ctx.message(`Saved ${name}`)
        return
      }
      case 'revertRack': {
        const own = live.id ? find(live.id) : undefined
        if (!own) return fail(`${live.name} has no saved rack to go back to`)
        this.enter(own, ctx)
        return
      }
      case 'renameRack': {
        const r = find(cmd.id)
        const name = cmd.name.trim()
        if (!r) return fail(`no rack ${cmd.id}`)
        if (!name) return fail('a rack needs a name')
        if (name !== r.name && taken(name)) return fail(`there is a rack called ${name} already`)
        r.name = name
        if (live.id === r.id) live.name = name
        return
      }
      case 'duplicateRack': {
        const r = find(cmd.id)
        if (!r) return fail(`no rack ${cmd.id}`)
        const copy = { ...structuredClone(r), id: this.newId(), name: this.unique(`${r.name} copy`) }
        this.racks.push(copy)
        ctx.message(`Duplicated as ${copy.name}`)
        return
      }
      case 'deleteRack': {
        if (!find(cmd.id)) return fail(`no rack ${cmd.id}`)
        if (live.id === cmd.id) return fail(`${live.name} is loaded: load another rack before deleting it`)
        this.racks = this.racks.filter((r) => r.id !== cmd.id)
        return
      }
      case 'dismissRackPrompt':
        live.prompt = null
        return
    }
  }

  /** The live rack becomes `rack` (null: a new one), unmodified. */
  private enter(rack: MockRack | null, ctx: RackCtx) {
    const st = ctx.state
    st.keyboardParts.forEach((p, part) => {
      const r = rack?.parts[part]
      ctx.command({ type: 'setPartPatch', part, id: r?.patch ?? null })
      Object.assign(p, r ?? { on: part === 0, program: DEFAULT_PROGRAMS[part], volume: 100, octave: 0, pan: 64, reverb: 0, chorus: 0, variation: 0 })
    })
    st.chord.split = rack?.split ?? 54
    st.chord.transposeKeyboard = rack?.transpose ?? 0
    Object.assign(st.liveRack, { name: rack?.name ?? NEW_NAME, id: rack?.id ?? null, modified: false, prompt: null })
    ctx.clean()
    ctx.message(`Loaded ${rack?.name ?? NEW_NAME}`)
  }

  private capture(st: AppState, id: string, name: string): MockRack {
    return {
      id,
      name,
      parts: st.keyboardParts.map(({ on, program, patch, volume, octave, pan, reverb, chorus, variation }) => ({ on, program, patch, volume, octave, pan, reverb, chorus, variation })),
      names: st.keyboardParts.map((p) => p.sound?.name ?? p.voiceName),
      split: st.chord.split,
      transpose: st.chord.transposeKeyboard,
    }
  }

  private unique(name: string): string {
    const base = name.trim() || NEW_NAME
    for (let n = 1; ; n++) {
      const candidate = n === 1 ? base : `${base} ${n}`
      if (!this.racks.some((r) => r.name === candidate)) return candidate
    }
  }

  private newId(): string {
    return `r-mock-${++this.seq}`
  }
}
