// The mock's Multi Pads: what the engine does (docs/multipad.md), closely enough to build
// and screenshot the Multi Pad panel. No audio. MockSession calls in on its commands, bar
// lines, band start/stop, Ending starts and chords.

import type { MultiPadCmd, MultiPadState, PadLamp } from './types'

const ROOT = '/Users/me/Styles'

/** The mock's bank files: the synthetic demo bank (`yahaha pad --demo`) and two more. */
const BANKS = [
  { name: 'Demo', folder: 'Pads', pads: ['Shaker Loop', 'Rise Arp', 'Bass Riff', 'Brass Hit'], repeat: [true, false, true, false], cm: [false, true, true, true] },
  { name: 'Strings FX', folder: 'Pads', pads: ['Swell', 'Pizz Run', 'Stab', 'Tremolo'], repeat: [false, false, false, true], cm: [true, true, true, true] },
  { name: 'Latin Perc', folder: 'Pads/Latin', pads: ['Conga Loop', 'Timbale Fill', 'Cowbell', ''], repeat: [true, false, true, false], cm: [false, false, false, false] },
]

export function initialMultiPad(): MultiPadState {
  return {
    bank: null,
    loading: false,
    pads: [0, 1, 2, 3].map((i) => ({ index: i, name: '', lamp: 'empty' as PadLamp, repeat: false, chordMatch: false, channel: 5 + i })),
    synchroStop: { styleStop: true, ending: false },
    banks: BANKS.map((b, id) => ({ id, name: b.name, folder: b.folder, path: `${ROOT}/${b.folder}/${b.name}.pad` })),
  }
}

export class MockPads {
  /** Beats left of each playing pad's pass (every mock pad is one 4-beat bar). */
  private left = [0, 0, 0, 0]

  constructor(private get: () => MultiPadState) {}

  private get st() {
    return this.get()
  }

  private setLamp(i: number, lamp: PadLamp) {
    this.st.pads[i].lamp = lamp
    if (lamp === 'playing') this.left[i] = 4
  }

  private has(i: number) {
    const p = this.st.pads[i]
    return p !== undefined && p.lamp !== 'empty'
  }

  /** A press or a Synchro Start: at once when stopped, at the next bar line while playing. */
  private start(i: number, running: boolean) {
    if (!this.has(i)) return
    this.setLamp(i, running ? 'queued' : 'playing')
  }

  /** A pad command. Returns an error message for a refused one. */
  cmd(c: MultiPadCmd, running: boolean): string | null {
    const st = this.st
    const pad = 'pad' in c ? c.pad : 0
    if ('pad' in c && (pad < 0 || pad > 3)) return `no Multi Pad ${pad} (pads are 0-3)`
    switch (c.type) {
      case 'loadMultiPad':
      case 'loadMultiPadPath': {
        const id = c.type === 'loadMultiPad' ? c.id : st.banks.findIndex((b) => b.path === c.path)
        const b = BANKS[id]
        if (!b) return c.type === 'loadMultiPad' ? `no Multi Pad bank ${c.id}` : `${c.path}: not a MIDI/Multi Pad file`
        st.bank = { id, name: b.name, path: st.banks[id].path }
        st.pads = b.pads.map((name, i) => ({
          index: i, name, lamp: (name ? 'ready' : 'empty') as PadLamp, repeat: name ? b.repeat[i] : false,
          chordMatch: name ? b.cm[i] : false, channel: 5 + i,
        }))
        break
      }
      case 'clearMultiPad':
        st.bank = null
        st.pads = initialMultiPad().pads
        break
      case 'triggerMultiPad':
        // Pressing any pad starts the pads in standby with it.
        st.pads.forEach((p, i) => p.lamp === 'armed' && this.start(i, running))
        this.start(pad, running)
        break
      case 'stopMultiPad':
        if (this.has(pad)) this.setLamp(pad, 'ready')
        break
      case 'stopAllMultiPads':
        st.pads.forEach((p, i) => this.has(i) && this.setLamp(i, 'ready'))
        break
      case 'armMultiPad':
        if (this.has(pad)) {
          const l = st.pads[pad].lamp
          if (l === 'armed') this.setLamp(pad, 'ready')
          else if (l === 'ready') this.setLamp(pad, 'armed')
        }
        break
      case 'setMultiPadRepeat':
        if (this.has(pad)) st.pads[pad].repeat = c.on
        break
      case 'setMultiPadChordMatch':
        if (this.has(pad)) st.pads[pad].chordMatch = c.on
        break
      case 'setMultiPadSynchroStop':
        st.synchroStop = { styleStop: c.styleStop, ending: c.ending }
        break
    }
    return null
  }

  /** The band moved on `beats` beats (or the clock did while stopped). */
  beats(beats: number) {
    this.st.pads.forEach((p, i) => {
      if (p.lamp !== 'playing') return
      this.left[i] -= beats
      if (this.left[i] <= 0) {
        if (p.repeat) this.left[i] += 4
        else p.lamp = 'ready'
      }
    })
  }

  /** A bar line while the band plays: queued pads start. */
  bar() {
    this.st.pads.forEach((p, i) => p.lamp === 'queued' && this.setLamp(i, 'playing'))
  }

  /** The band started: armed pads start with it. */
  bandStarted() {
    this.st.pads.forEach((p, i) => p.lamp === 'armed' && this.setLamp(i, 'playing'))
  }

  /** A chord in the chord section: armed pads start. */
  chord(running: boolean) {
    this.st.pads.forEach((p, i) => p.lamp === 'armed' && this.start(i, running))
  }

  private stopRepeating() {
    this.st.pads.forEach((p, i) => p.repeat && (p.lamp === 'playing' || p.lamp === 'queued') && this.setLamp(i, 'ready'))
  }

  bandStopped() {
    if (this.st.synchroStop.styleStop) this.stopRepeating()
  }

  endingStarted() {
    if (this.st.synchroStop.ending) this.stopRepeating()
  }

  panic() {
    this.st.pads.forEach((p, i) => this.has(i) && this.setLamp(i, 'ready'))
  }
}
