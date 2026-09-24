// The browser mock's Registration Memory and Playlist: a port of src/session/registration.rs
// and src/session/playlist.rs over in-memory "files" (nothing is written to disk). It
// starts with two demo banks and a demo playlist so the Registration bar and the Playlist
// drawer have something to show.

import fixture from './mock-fixture.json'
import type { AppCmd, AppState, Fingering } from './types'
import {
  emptyPlaylist, emptyRegistration, fileStem, REGIST_GROUPS,
  type PlaylistCmd, type PlaylistRecord, type PlaylistSort, type RegistGroup, type RegistrationCmd, type SequenceEnd,
} from './registration'

const DATA = '/Users/me/Documents/yahaha'
const BANK_DIR = `${DATA}/Registration`
const LIST_DIR = `${DATA}/Playlists`
const MAX_RECORDS = 2500
const GM: string[] = fixture.gm

/** One button's contents: what the engine's registrables store, flattened. */
interface Memory {
  name: string
  groups: RegistGroup[]
  style?: { path: string; name: string }
  tempo?: number
  control?: { main: number; otsLink: boolean; stopAcmp: boolean }
  chord?: { fingering: Fingering; upper: boolean; manualBass: boolean; split: number }
  mixer?: { volumes: number[]; on: boolean[] }
  /** Right 1, Right 2, Right 3, Left; null for a part outside the memorized groups. */
  parts?: ({ on: boolean; program: number; volume: number; octave: number } | null)[]
  transpose?: [number, number]
}

interface Bank {
  name: string
  memories: (Memory | null)[]
  sequence: { steps: number[]; end: SequenceEnd }
}

interface List {
  name: string
  records: PlaylistRecord[]
}

/** What the registration needs from the mock session. */
export interface RegistHost {
  state: AppState
  /** Run one of the session's own commands (load a style, press a Main). */
  command(cmd: AppCmd): void
  message(text: string, error?: boolean): void
  /** The library entry path for a style path or name, if the library has it. */
  findStyle(path: string, name: string): string | null
}

const bankPath = (name: string) => `${BANK_DIR}/${fileStem(name)}.regist.json`
const listPath = (name: string) => `${LIST_DIR}/${fileStem(name)}.playlist.json`
/** The key of `files` that is `path` on a case-insensitive file system (the Mac's). */
const existing = (files: Map<string, unknown>, path: string) => [...files.keys()].find((k) => k.toLowerCase() === path.toLowerCase())
const has = (m: Memory, g: RegistGroup) => m.groups.includes(g)

function demoMemory(name: string, style: [string, string], tempo: number, programs: number[], on: boolean[]): Memory {
  return {
    name,
    groups: REGIST_GROUPS.map((g) => g.id),
    style: { path: style[0], name: style[1] },
    tempo,
    control: { main: 0, otsLink: false, stopAcmp: false },
    chord: { fingering: 'fingeredOnBass', upper: false, manualBass: true, split: 54 },
    mixer: { volumes: [100, 100, 96, 80, 76, 70, 88, 84], on: Array(8).fill(true) },
    parts: programs.map((program, i) => ({ on: on[i], program, volume: 100, octave: 0 })),
    transpose: [0, 0],
  }
}

export class MockRegistration {
  private banks = new Map<string, Bank>()
  private lists = new Map<string, List>()
  private bank: Bank = { name: 'New Bank', memories: Array(10).fill(null), sequence: { steps: [], end: 'stop' } }
  private path: string | null = null
  private dirty = false
  private selected: number | null = null
  private memory = false
  private memorize: RegistGroup[] = REGIST_GROUPS.map((g) => g.id)
  private freeze = false
  private frozen: RegistGroup[] = []
  private seqPos: number | null = null
  /** Sequence On/Off: a panel setting, not part of the bank (Genos Data List). */
  private seqOn = true
  private list: List = { name: 'New Playlist', records: [] }
  private listPath: string | null = null
  private listDirty = false
  private sort: PlaylistSort = 'normal'
  private current: number | null = null

  /** With `demo`, two banks and a playlist, and the first bank loaded. */
  constructor(styles: { path: string; name: string }[], demo = true) {
    if (!demo || styles.length === 0) return
    const st = (i: number): [string, string] => [styles[i % styles.length].path, styles[i % styles.length].name]
    const gig: Bank = {
      name: 'Friday Gig',
      memories: [
        demoMemory('Intro ballad', st(0), 72, [0, 48, 61, 48], [true, true, false, false]),
        demoMemory('Verse', st(1), 96, [4, 48, 61, 32], [true, false, false, false]),
        demoMemory('Chorus', st(1), 96, [61, 48, 56, 32], [true, true, true, false]),
        demoMemory('Swing', st(2), 132, [26, 48, 65, 32], [true, false, true, false]),
        ...Array(6).fill(null),
      ],
      sequence: { steps: [0, 1, 2, 1, 2, 3], end: 'next' },
    }
    const jazz: Bank = {
      name: 'Jazz Set',
      memories: [demoMemory('Trio', st(3), 120, [0, 32, 48, 32], [true, false, false, false]), null, null, null, null, null, null, null, null, null],
      sequence: { steps: [], end: 'stop' },
    }
    this.banks.set(bankPath(gig.name), gig)
    this.banks.set(bankPath(jazz.name), jazz)
    this.lists.set(listPath('Friday'), {
      name: 'Friday',
      records: [
        { name: 'Opener', kind: 'bank', path: bankPath(gig.name), regist: 0 },
        { name: 'Swing number', kind: 'bank', path: bankPath(gig.name), regist: 3 },
        { name: 'Jazz Set', kind: 'bank', path: bankPath(jazz.name) },
        { name: styles[4 % styles.length].name, kind: 'style', path: styles[4 % styles.length].path },
      ],
    })
    this.loadBank(bankPath(gig.name))
    this.loadList(listPath('Friday'))
  }

  /** Registration and Playlist commands; false for anything else. */
  handles(cmd: AppCmd): cmd is RegistrationCmd | PlaylistCmd {
    return REGIST_TYPES.has(cmd.type) || PLAYLIST_TYPES.has(cmd.type)
  }

  cmd(cmd: RegistrationCmd | PlaylistCmd, host: RegistHost) {
    if (REGIST_TYPES.has(cmd.type)) this.regist(cmd as RegistrationCmd, host)
    else this.playlist(cmd as PlaylistCmd, host)
  }

  private fail(host: RegistHost, text: string) {
    host.message(text, true)
  }

  private regist(cmd: RegistrationCmd, host: RegistHost) {
    switch (cmd.type) {
      case 'pressRegist':
        if (this.memory) this.memorizeInto(cmd.index, host)
        else this.recall(cmd.index, host, true)
        break
      case 'recallRegist':
        this.recall(cmd.index, host, true)
        break
      case 'memorizeRegist':
        this.memorizeInto(cmd.index, host)
        break
      case 'toggleRegistMemory':
        this.memory = !this.memory
        break
      case 'setMemorizeGroup':
        this.memorize = toggled(this.memorize, cmd.group, cmd.on)
        break
      case 'clearRegist':
        this.bank.memories[cmd.index] = null
        if (this.selected === cmd.index) this.selected = null
        this.changed()
        break
      case 'renameRegist': {
        const m = this.bank.memories[cmd.index]
        if (!m) return this.fail(host, `Registration ${cmd.index + 1} is empty`)
        m.name = cmd.name.trim()
        this.changed()
        break
      }
      case 'stepRegistBank':
        this.stepBank(cmd.delta, host)
        break
      case 'selectRegistBank':
        if (!this.loadBank(cmd.path)) this.fail(host, `${cmd.path}: not found`)
        break
      case 'newRegistBank':
        this.bank = { name: 'New Bank', memories: Array(10).fill(null), sequence: { steps: [], end: 'stop' } }
        this.path = null
        this.dirty = false
        this.selected = null
        this.seqPos = null
        break
      case 'saveRegistBank': {
        if (cmd.name !== null || !this.path) {
          const name = (cmd.name ?? this.bank.name).trim() || 'Untitled'
          const path = bankPath(name)
          const there = existing(this.banks, path)
          if (there !== undefined && there !== this.path && !cmd.overwrite) {
            return this.fail(host, `a bank called ${name} already exists: save under another name, or overwrite it`)
          }
          // The same file in another case: renamed to the case typed.
          if (there !== undefined) this.banks.delete(there)
          this.bank.name = name
          this.path = path
        }
        this.banks.set(this.path, clone(this.bank))
        this.dirty = false
        host.message(`Saved bank ${this.bank.name}`)
        break
      }
      case 'setFreeze':
        this.freeze = cmd.on
        break
      case 'toggleFreeze':
        this.freeze = !this.freeze
        break
      case 'setFreezeGroup':
        this.frozen = toggled(this.frozen, cmd.group, cmd.on)
        break
      case 'setRegistSequence':
        this.bank.sequence = { steps: cmd.steps.filter((b) => b >= 0 && b < 10).slice(0, 128), end: cmd.end }
        this.seqPos = null
        this.changed()
        break
      case 'setRegistSequenceOn':
        this.seqOn = cmd.on
        break
      case 'toggleRegistSequence':
        this.seqOn = !this.seqOn
        break
      case 'stepRegistSequence':
        this.stepSequence(cmd.delta, host)
        break
    }
  }

  /** A saved bank is written at once; a new one waits for a name. */
  private changed() {
    this.dirty = true
    if (this.path) {
      this.banks.set(this.path, clone(this.bank))
      this.dirty = false
    }
  }

  private loadBank(path: string): boolean {
    const b = this.banks.get(path)
    if (!b) return false
    this.bank = clone(b)
    this.path = path
    this.dirty = false
    this.selected = null
    this.memory = false
    this.seqPos = null
    return true
  }

  private bankPaths(): string[] {
    return [...this.banks.keys()].sort((a, b) => a.toLowerCase().localeCompare(b.toLowerCase()))
  }

  private stepBank(delta: number, host: RegistHost, quiet = false): boolean {
    const all = this.bankPaths()
    if (all.length === 0) {
      if (!quiet) this.fail(host, 'no Registration banks saved yet')
      return false
    }
    const pos = this.path ? all.indexOf(this.path) : -1
    const next = pos < 0 ? (delta >= 0 ? 0 : all.length - 1) : pos + Math.sign(delta)
    if (next < 0 || next >= all.length) return false
    this.loadBank(all[next])
    host.message(`Bank: ${this.bank.name}`)
    return true
  }

  private stepSequence(delta: number, host: RegistHost) {
    const seq = this.bank.sequence
    if (!this.seqOn) return this.fail(host, 'Registration Sequence is off')
    const n = seq.steps.length
    if (n === 0 || delta === 0) return
    const pos = this.seqPos !== null && this.seqPos < n ? this.seqPos : null
    let next: number | 'stay' | 'next' | 'prev'
    if (delta > 0) next = pos === null ? 0 : pos + 1 < n ? pos + 1 : seq.end === 'stop' ? 'stay' : seq.end === 'top' ? 0 : 'next'
    else next = pos === null ? n - 1 : pos > 0 ? pos - 1 : seq.end === 'stop' ? 'stay' : seq.end === 'top' ? n - 1 : 'prev'
    if (next === 'stay') return
    if (next === 'next' || next === 'prev') {
      if (!this.stepBank(delta, host, true) || this.bank.sequence.steps.length === 0) return
      const s = this.bank.sequence.steps
      next = next === 'next' ? 0 : s.length - 1
    }
    this.seqPos = next
    this.recall(this.bank.sequence.steps[next], host, false)
  }

  private memorizeInto(index: number, host: RegistHost) {
    this.memory = false
    const g = this.memorize
    if (g.length === 0) return this.fail(host, 'Memorize: no groups ticked')
    const st = host.state
    const m: Memory = { name: '', groups: [...g] }
    if (g.includes('style')) {
      m.style = { path: st.style.path, name: st.style.name }
      m.control = { main: st.transport.main, otsLink: st.ots.link, stopAcmp: st.transport.stopAcmp }
      m.chord = { fingering: st.chord.fingering, upper: st.chord.upper, manualBass: st.chord.manualBass, split: st.chord.split }
      m.mixer = { volumes: st.mixer.styleParts.map((p) => p.volume), on: st.mixer.styleParts.map((p) => p.on) }
    }
    if (g.includes('style') || g.includes('voice')) {
      m.parts = st.keyboardParts.map((p, i) =>
        g.includes(i === 3 ? 'style' : 'voice') ? { on: p.on, program: p.program, volume: p.volume, octave: p.octave } : null,
      )
    }
    if (g.includes('tempo')) m.tempo = st.transport.tempo
    if (g.includes('transpose')) m.transpose = [st.chord.transposeKeyboard, st.chord.transposeMaster]
    m.name = m.style?.name ?? `Registration ${index + 1}`
    this.bank.memories[index] = m
    this.selected = index
    host.message(`Memorized to Registration ${index + 1}`)
    this.changed()
  }

  /** Recall a button and say so; `label` is the message (what could not be recalled follows it, as an error). */
  private recall(index: number, host: RegistHost, follow: boolean, label?: string) {
    const m = this.bank.memories[index]
    if (!m) return this.fail(host, `Registration ${index + 1} is empty`)
    const errors: string[] = []
    this.memory = false
    this.selected = index
    const seq = this.bank.sequence
    if (follow && seq.steps.length) {
      const start = this.seqPos === null ? 0 : this.seqPos + 1
      if (this.seqPos === null || seq.steps[this.seqPos] !== index) {
        for (let k = 0; k < seq.steps.length; k++) {
          const i = (start + k) % seq.steps.length
          if (seq.steps[i] === index) {
            this.seqPos = i
            break
          }
        }
      }
    }
    const allowed = (g: RegistGroup) => has(m, g) && !(this.freeze && this.frozen.includes(g))
    const st = host.state
    const keepTempo = has(m, 'tempo') && !allowed('tempo') ? st.transport.tempo : null
    if (allowed('style') && m.style) {
      const path = host.findStyle(m.style.path, m.style.name)
      if (path) {
        if (path !== st.style.path) host.command({ type: 'loadStylePath', path })
      } else errors.push(`style not found: ${m.style.path}`)
    }
    if (keepTempo !== null) st.transport.tempo = keepTempo
    if (allowed('tempo') && m.tempo !== undefined) st.transport.tempo = m.tempo
    if (allowed('style') && m.chord) {
      Object.assign(st.chord, { fingering: m.chord.fingering, upper: m.chord.upper, manualBass: m.chord.manualBass, split: m.chord.split })
    }
    if (allowed('style') && m.control) {
      st.ots.link = m.control.otsLink
      st.transport.stopAcmp = m.control.stopAcmp
      if (m.control.main !== st.transport.main) {
        // Pressing the Main with OTS Link off, so the registration's voices stay.
        const link = st.ots.link
        st.ots.link = false
        host.command({ type: 'main', index: m.control.main })
        st.ots.link = link
      }
    }
    if (allowed('style') && m.mixer) {
      st.mixer.styleParts.forEach((p, i) => {
        p.volume = m.mixer!.volumes[i]
        p.on = m.mixer!.on[i]
      })
    }
    m.parts?.forEach((p, i) => {
      if (!p || !allowed(i === 3 ? 'style' : 'voice')) return
      Object.assign(st.keyboardParts[i], { on: p.on, program: p.program, volume: p.volume, octave: p.octave })
    })
    if (allowed('transpose') && m.transpose) {
      st.chord.transposeKeyboard = m.transpose[0]
      st.chord.transposeMaster = m.transpose[1]
    }
    const text = label ?? `Registration ${index + 1}: ${m.name || `Registration ${index + 1}`}`
    if (errors.length) host.message(`${text}: ${errors.join('; ')}`, true)
    else host.message(text)
  }

  // ── Playlist ──────────────────────────────────────────────────────────

  private loadList(path: string): boolean {
    const l = this.lists.get(path)
    if (!l) return false
    this.list = clone(l)
    this.listPath = path
    this.listDirty = false
    this.sort = 'normal'
    this.current = null
    return true
  }

  private order(): number[] {
    const idx = this.list.records.map((_, i) => i)
    if (this.sort === 'normal') return idx
    const sorted = idx.sort((a, b) => this.list.records[a].name.toLowerCase().localeCompare(this.list.records[b].name.toLowerCase()))
    return this.sort === 'aToZ' ? sorted : sorted.reverse()
  }

  private add(record: PlaylistRecord, host: RegistHost) {
    if (this.list.records.length >= MAX_RECORDS) return this.fail(host, `a playlist holds at most ${MAX_RECORDS} records`)
    const name = record.name.trim() || (record.path.split('/').pop() ?? '').replace(/\.regist\.json$|\.[^.]+$/, '')
    this.list.records.push({ ...record, name })
    this.listDirty = true
  }

  private unsorted(host: RegistHost, what: string): boolean {
    if (this.sort === 'normal') return true
    this.fail(host, `sort the playlist back to Normal to ${what}`)
    return false
  }

  private loadRecord(index: number, host: RegistHost) {
    const r = this.list.records[index]
    if (!r) return this.fail(host, `no playlist record ${index + 1}`)
    this.current = index
    if (r.kind === 'bank') {
      if (!this.loadBank(r.path)) return this.fail(host, `${r.path}: not found`)
      if (r.regist !== undefined && r.regist !== null) return this.recall(r.regist, host, true, `Playlist ${index + 1}: ${r.name}`)
    } else host.command({ type: 'loadStylePath', path: r.path })
    host.message(`Playlist ${index + 1}: ${r.name}`)
  }

  private playlist(cmd: PlaylistCmd, host: RegistHost) {
    switch (cmd.type) {
      case 'newPlaylist':
        this.list = { name: 'New Playlist', records: [] }
        this.listPath = null
        this.listDirty = false
        this.sort = 'normal'
        this.current = null
        break
      case 'loadPlaylist':
        if (!this.loadList(cmd.path)) this.fail(host, `${cmd.path}: not found`)
        break
      case 'savePlaylist': {
        if (this.sort !== 'normal') {
          const order = this.order()
          this.current = this.current === null ? null : order.indexOf(this.current)
          this.list.records = order.map((i) => this.list.records[i])
          this.sort = 'normal'
        }
        if (cmd.name !== null || !this.listPath) {
          const name = (cmd.name ?? this.list.name).trim() || 'Untitled'
          const path = listPath(name)
          const there = existing(this.lists, path)
          if (there !== undefined && there !== this.listPath && !cmd.overwrite) {
            return this.fail(host, `a playlist called ${name} already exists: save under another name, or overwrite it`)
          }
          if (there !== undefined) this.lists.delete(there)
          this.list.name = name
          this.listPath = path
        }
        this.lists.set(this.listPath, clone(this.list))
        this.listDirty = false
        host.message(`Saved playlist ${this.list.name}`)
        break
      }
      case 'addPlaylistRecord':
        this.add(cmd.record, host)
        break
      case 'addCurrentBank':
        if (!this.path) return this.fail(host, 'save the bank first: a Playlist record links to a bank file')
        this.add({
          name: this.selected === null ? this.bank.name : `${this.bank.name} [${this.selected + 1}]`,
          kind: 'bank', path: this.path, regist: this.selected,
        }, host)
        break
      case 'addCurrentStyle':
        this.add({ name: host.state.style.name, kind: 'style', path: host.state.style.path }, host)
        break
      case 'appendPlaylist': {
        const other = this.lists.get(cmd.path)
        if (!other) return this.fail(host, `${cmd.path}: not found`)
        for (const r of other.records) this.add(clone(r), host)
        break
      }
      case 'setPlaylistRecord':
        if (!this.list.records[cmd.index]) return this.fail(host, `no playlist record ${cmd.index + 1}`)
        this.list.records[cmd.index] = { ...cmd.record }
        this.listDirty = true
        break
      case 'movePlaylistRecord': {
        if (!this.unsorted(host, 'move records')) return
        const j = cmd.index + cmd.delta
        const recs = this.list.records
        if (cmd.index >= recs.length || j < 0 || j >= recs.length) return
        const [r] = recs.splice(cmd.index, 1)
        recs.splice(j, 0, r)
        if (this.current === cmd.index) this.current = j
        else if (this.current === j) this.current = cmd.index
        this.listDirty = true
        break
      }
      case 'deletePlaylistRecord':
        if (!this.unsorted(host, 'delete records')) return
        if (!this.list.records[cmd.index]) return this.fail(host, `no playlist record ${cmd.index + 1}`)
        this.list.records.splice(cmd.index, 1)
        if (this.current === cmd.index) this.current = null
        else if (this.current !== null && this.current > cmd.index) this.current--
        this.listDirty = true
        break
      case 'setPlaylistSort':
        this.sort = cmd.sort
        break
      case 'loadPlaylistRecord':
        this.loadRecord(cmd.index, host)
        break
      case 'stepPlaylist': {
        const order = this.order()
        if (order.length === 0) return
        const pos = this.current === null ? -1 : order.indexOf(this.current)
        const next = pos < 0 ? (cmd.delta >= 0 ? 0 : order.length - 1) : Math.max(0, Math.min(order.length - 1, pos + Math.sign(cmd.delta)))
        if (next === pos) return
        this.loadRecord(order[next], host)
        break
      }
    }
  }

  /** Write `state.registration` and `state.playlist`. */
  fill(st: AppState) {
    const r = emptyRegistration()
    const banks = this.bankPaths()
    r.bank = { name: this.bank.name, path: this.path, dirty: this.dirty, position: this.path ? banks.indexOf(this.path) : null }
    r.banks = banks.map((path) => ({ name: this.banks.get(path)!.name, path }))
    r.folder = BANK_DIR
    r.buttons = this.bank.memories.map((m, index) =>
      m
        ? {
            index, stored: true, name: m.name, groups: [...m.groups], style: m.style?.name ?? null, tempo: m.tempo ?? null,
            voices: m.parts ? m.parts.map((p) => (p ? { name: GM[p.program] ?? '?', on: p.on } : { name: '', on: false })) : [],
          }
        : { index, stored: false, name: '', groups: [], style: null, tempo: null, voices: [] },
    )
    r.selected = this.selected
    r.memory = this.memory
    r.memorizeGroups = REGIST_GROUPS.map((g) => g.id).filter((g) => this.memorize.includes(g))
    r.freeze = this.freeze
    r.freezeGroups = REGIST_GROUPS.map((g) => g.id).filter((g) => this.frozen.includes(g))
    r.sequence = { on: this.seqOn, ...this.bank.sequence, steps: [...this.bank.sequence.steps], position: this.seqPos }
    st.registration = r

    const p = emptyPlaylist()
    p.name = this.list.name
    p.path = this.listPath
    p.dirty = this.listDirty
    p.sort = this.sort
    p.records = this.order().map((index) => {
      const record = this.list.records[index]
      const missing = record.kind === 'bank' ? !this.banks.has(record.path) : false
      return { index, record: clone(record), missing }
    })
    p.current = this.current
    p.playlists = [...this.lists.entries()].map(([path, l]) => ({ name: l.name, path })).sort((a, b) => a.name.localeCompare(b.name))
    p.folder = LIST_DIR
    st.playlist = p
  }
}

const REGIST_TYPES = new Set<string>([
  'pressRegist', 'recallRegist', 'memorizeRegist', 'toggleRegistMemory', 'setMemorizeGroup', 'clearRegist', 'renameRegist',
  'stepRegistBank', 'selectRegistBank', 'newRegistBank', 'saveRegistBank', 'setFreeze', 'toggleFreeze', 'setFreezeGroup',
  'setRegistSequence', 'setRegistSequenceOn', 'toggleRegistSequence', 'stepRegistSequence',
])
const PLAYLIST_TYPES = new Set<string>([
  'newPlaylist', 'loadPlaylist', 'savePlaylist', 'addPlaylistRecord', 'addCurrentBank', 'addCurrentStyle', 'appendPlaylist',
  'setPlaylistRecord', 'movePlaylistRecord', 'deletePlaylistRecord', 'setPlaylistSort', 'loadPlaylistRecord', 'stepPlaylist',
])

function toggled(list: RegistGroup[], g: RegistGroup, on: boolean): RegistGroup[] {
  const rest = list.filter((x) => x !== g)
  return on ? [...rest, g] : rest
}

function clone<T>(v: T): T {
  return JSON.parse(JSON.stringify(v))
}
