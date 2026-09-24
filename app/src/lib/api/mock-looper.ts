// The mock session's Chord Looper: the engine's (src/engine/looper.rs) rules, simplified
// to whole beats. Recording, loop playback and a memory change start at the next bar;
// stopping the loop is immediate. See docs/chord-looper.md.

import type { LoopChord, LooperMemory, LooperState } from './types'

const MEMORIES = 8

export function emptyLooper(): LooperState {
  return {
    mode: 'off',
    hasData: false,
    bar: null,
    bars: 0,
    chords: [],
    memory: null,
    pendingMemory: null,
    memories: Array.from({ length: MEMORIES }, (): LooperMemory => ({ name: null, bars: 0, chords: [] })),
  }
}

interface Seq {
  bars: number
  chords: LoopChord[]
}

export class MockLooper {
  private seq: Seq = { bars: 0, chords: [] }
  private pending: Seq | null = null
  private recStart = 0
  private recBars = 0
  private loopBar = 0
  private loopStart = 0
  private kbd: string | null = null
  private stored = 0

  constructor(private readonly get: () => LooperState) {}

  private get s(): LooperState {
    return this.get()
  }

  /** A chord from the keyboard at `bar` (the band's bar count) and 1-based `beat`. False:
   *  the loop plays, the keyboard's chord is ignored. */
  keyboardChord(chord: string, bar: number, beat: number): boolean {
    if (this.s.mode === 'looping') {
      this.kbd = chord
      return false
    }
    if (this.s.mode === 'recording') this.record(bar - this.recStart + 1, beat, chord)
    return true
  }

  private record(bar: number, beat: number, chord: string) {
    const c = this.seq.chords
    const last = c[c.length - 1]
    if (last && last.bar === bar && last.beat === beat) last.chord = chord
    else if (!last || last.chord !== chord) c.push({ bar, beat, chord })
  }

  /** REC/STOP. Returns true when Sync Start must turn on (stopped). */
  rec(running: boolean): boolean {
    const s = this.s
    if (s.mode === 'recArmed') s.mode = 'off'
    else if (s.mode === 'recording') this.finish('off')
    else {
      s.mode = 'recArmed'
      this.pending = null
      s.pendingMemory = null
      return !running
    }
    return false
  }

  /** ON/OFF. Returns the keyboard's chord to follow when the loop stops. */
  onOff(): string | null {
    const s = this.s
    switch (s.mode) {
      case 'recording':
        this.finish('loopArmed')
        break
      case 'recArmed':
      case 'loopArmed':
        s.mode = 'off'
        break
      case 'off':
        if (this.seq.chords.length) s.mode = 'loopArmed'
        break
      case 'looping': {
        s.mode = 'off'
        this.pending = null
        s.pendingMemory = null
        const k = this.kbd
        this.kbd = null
        return k
      }
    }
    return null
  }

  private finish(next: 'off' | 'loopArmed') {
    this.seq.bars = this.recBars
    this.seq.chords = this.seq.chords.filter((c) => c.bar <= this.recBars)
    this.s.memory = null
    this.s.mode = this.seq.chords.length ? next : 'off'
  }

  /** A bar line (`bar`, the band's count): returns the loop chord to play there, if any. */
  onBar(bar: number, played: string | null): string | null {
    const s = this.s
    if (s.mode === 'recArmed') {
      s.mode = 'recording'
      this.seq = { bars: 0, chords: [] }
      this.recStart = bar
      this.recBars = 1
      if (played) this.record(1, 1, played)
      return null
    }
    if (s.mode === 'recording') {
      this.recBars = bar - this.recStart + 1
      return null
    }
    if (s.mode === 'loopArmed' || (s.mode === 'looping' && this.pending)) {
      if (this.pending) {
        this.seq = this.pending
        this.pending = null
        s.memory = s.pendingMemory
        s.pendingMemory = null
      }
      s.mode = 'looping'
      this.loopStart = bar
    }
    if (s.mode !== 'looping' || !this.seq.bars) return null
    this.loopBar = (bar - this.loopStart) % this.seq.bars
    const at = this.seq.chords.filter((c) => c.bar <= this.loopBar + 1)
    return (at.length ? at[at.length - 1] : this.seq.chords[this.seq.chords.length - 1])?.chord ?? null
  }

  /** The band stopped: a recording ends; a loop waits for the next start. */
  onStop() {
    const s = this.s
    if (s.mode === 'recording') this.finish('off')
    else if (s.mode === 'recArmed') s.mode = 'off'
    else if (s.mode === 'looping') s.mode = 'loopArmed'
  }

  select(i: number): string | null {
    const s = this.s
    if (s.mode === 'recording' || s.mode === 'recArmed') return 'Chord Looper: stop recording before choosing a memory'
    const m = s.memories[i]
    if (!m.name) {
      s.memory = i
      return null
    }
    const seq = { bars: m.bars, chords: m.chords.map((c) => ({ ...c })) }
    if (s.mode === 'looping') {
      this.pending = seq
      s.pendingMemory = i
    } else {
      this.seq = seq
      s.memory = i
    }
    return null
  }

  store(i: number): string | null {
    const s = this.s
    if (!this.seq.chords.length || s.mode === 'recording' || s.mode === 'recArmed') return 'Chord Looper: record a chord sequence first'
    this.stored += 1
    s.memories[i] = { name: `CLD_${String(this.stored).padStart(3, '0')}`, bars: this.seq.bars, chords: this.seq.chords.map((c) => ({ ...c })) }
    s.memory = i
    return null
  }

  clear(i: number) {
    this.s.memories[i] = { name: null, bars: 0, chords: [] }
  }

  newBank() {
    const s = this.s
    s.memories = emptyLooper().memories
    s.memory = null
    s.pendingMemory = null
    this.pending = null
  }

  /** Bring the derived fields up to date. */
  publish() {
    const s = this.s
    const recording = s.mode === 'recording'
    s.hasData = this.seq.chords.length > 0 && !recording
    s.bars = recording ? this.recBars : this.seq.bars
    s.bar = recording ? this.recBars : s.mode === 'looping' ? this.loopBar + 1 : null
    s.chords = recording ? [] : this.seq.chords.map((c) => ({ ...c }))
  }
}
