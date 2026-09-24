// The settings adapter: settings the Settings drawer shows that `AppCmd` / `AppState`
// don't have yet. Everything the engine already has goes straight through `app.send`;
// only the gaps below are mocked here, in the shape proposed to the engine (NEED lines on
// the coordination board). When the engine grows them, `view()` reads the real fields and
// `send()` forwards the command, and the mock state here stops being used.
//
// Proposed engine API (AppCmd JSON, camelCase like the rest):
//   {"type":"setSoundFont","file":"FluidR3_GM.sf2"}          reloads the synth's SoundFont from soundfonts/
//   {"type":"setMidiInputs","all":true,"names":[]}            merge every source, or only `names`
//   {"type":"setPaletteLeds","on":true}                        Novation palette colours instead of RGB SysEx
//   {"type":"rescanLibrary"}                                   re-walk the style folders
// Proposed AppState fields:
//   io.soundFonts: string[]            the .sf2 files in soundfonts/, by file name
//   io.soundFontFile: string           the file the synth plays (io.synth.soundFont is its display name)
//   io.sources: {name, listening}[]    every MIDI source, connected or chosen
//   io.allInputs: bool                 merging every source (--all-inputs)
//   pads.paletteLeds: bool             --palette-leds
//   library.roots: string[]            the style folders scanned (Options.paths)
//   library.scanning: bool             a rescan is running

import type { AppState } from './types'

export type ProposedCmd =
  | { type: 'setSoundFont'; file: string }
  | { type: 'setMidiInputs'; all: boolean; names: string[] }
  | { type: 'setPaletteLeds'; on: boolean }
  | { type: 'rescanLibrary' }

export interface MidiSource {
  name: string
  /** The Launchkey's DAW port (pads and buttons). */
  pads: boolean
  /** yahaha plays from it. */
  listening: boolean
}

export interface SettingsView {
  soundFonts: string[]
  soundFontFile: string | null
  sources: MidiSource[]
  /** null: the engine doesn't say (real engine, field not there yet). */
  allInputs: boolean | null
  paletteLeds: boolean | null
  roots: string[]
  scanning: boolean
  /**
   * Per setting: true while the engine doesn't have it. On the mock session these are
   * simulated here; on the real engine the drawer badges them and makes them inert, and
   * this view shows only what the engine really reports (never the mock's sample data).
   */
  mocked: { soundFont: boolean; inputs: boolean; paletteLeds: boolean; library: boolean }
}

/** The proposed fields, as they'd arrive on AppState. */
type Proposed = {
  io?: { soundFonts?: string[]; soundFontFile?: string; sources?: { name: string; listening: boolean }[]; allInputs?: boolean }
  pads?: { paletteLeds?: boolean }
  library?: { roots?: string[]; scanning?: boolean }
}

/** Sources the mock adds to what `io.inputs` lists: the owner's audio interface and the IAC bus. */
const MOCK_EXTRA_SOURCES = ['TASCAM Model 16', 'IAC Driver Bus 1']
const MOCK_SOUNDFONTS = ['GeneralUser-GS.sf2', 'FluidR3_GM.sf2', 'MuseScore_General.sf2']
const MOCK_ROOTS = ['~/Music/Styles/Genos', '~/Music/Styles/PSR-SX900', 'corpus']
const RESCAN_MS = 1200

function isPadsPort(name: string): boolean {
  return name.endsWith(' (pads)')
}

class SettingsAdapter {
  soundFontFile = $state<string | null>(null)
  allInputs = $state(true)
  chosen = $state<string[] | null>(null)
  paletteLeds = $state(false)
  scanning = $state(false)
  private timer: ReturnType<typeof setTimeout> | null = null

  /**
   * Everything the drawer shows, from the engine when it has the field. A missing field
   * is simulated on the mock session; on the real engine (`real`) it shows only what the
   * engine does report, so nothing made up appears there.
   */
  view(s: AppState, real = false): SettingsView {
    const p = s as unknown as Proposed
    const mocked = {
      soundFont: p.io?.soundFonts === undefined,
      inputs: p.io?.sources === undefined,
      // The engine reports pads.paletteLeds (#77) but can't switch it at run time yet
      // (no setPaletteLeds), so the switch stays mocked.
      paletteLeds: true,
      library: p.library?.roots === undefined,
    }
    const connected = s.io.inputs
    const synthFile = s.io.synth ? `${s.io.synth.soundFont}.sf2` : null

    let sources: MidiSource[]
    let allInputs: boolean | null
    if (!mocked.inputs) {
      sources = p.io!.sources!.map((x) => ({ name: x.name, pads: isPadsPort(x.name), listening: x.listening }))
      allInputs = p.io?.allInputs ?? null
    } else if (real) {
      // The engine lists only the inputs it has open, and all of them play.
      sources = connected.map((name) => ({ name, pads: isPadsPort(name), listening: true }))
      allInputs = null
    } else {
      const chosen = this.chosen ?? connected
      sources = [...connected, ...MOCK_EXTRA_SOURCES.filter((n) => !connected.includes(n))].map((name) => ({
        name,
        pads: isPadsPort(name),
        listening: this.allInputs || chosen.includes(name),
      }))
      allInputs = this.allInputs
    }

    let soundFonts: string[]
    let soundFontFile: string | null
    if (!mocked.soundFont) {
      soundFonts = p.io!.soundFonts!
      soundFontFile = p.io?.soundFontFile ?? null
    } else if (real) {
      // Only the one the synth is playing: the engine can't list or switch them yet.
      soundFonts = synthFile ? [synthFile] : []
      soundFontFile = synthFile
    } else {
      soundFonts = MOCK_SOUNDFONTS
      soundFontFile = this.soundFontFile ?? (synthFile && MOCK_SOUNDFONTS.includes(synthFile) ? synthFile : null)
    }

    return {
      soundFonts,
      soundFontFile,
      sources,
      allInputs,
      paletteLeds: real ? (p.pads?.paletteLeds ?? null) : this.paletteLeds,
      roots: p.library?.roots ?? (real ? [] : MOCK_ROOTS),
      scanning: p.library?.scanning ?? ((!real && this.scanning) || s.library.pending > 0),
      mocked,
    }
  }

  /** Applies a proposed command to the mock. (When the engine has it: `app.send(cmd)`.) */
  send(cmd: ProposedCmd) {
    switch (cmd.type) {
      case 'setSoundFont':
        this.soundFontFile = cmd.file
        break
      case 'setMidiInputs':
        this.allInputs = cmd.all
        this.chosen = cmd.names
        break
      case 'setPaletteLeds':
        this.paletteLeds = cmd.on
        break
      case 'rescanLibrary':
        this.scanning = true
        if (this.timer) clearTimeout(this.timer)
        this.timer = setTimeout(() => {
          this.scanning = false
          this.timer = null
        }, RESCAN_MS)
        break
    }
  }

  /** Back to the defaults (tests). */
  reset() {
    if (this.timer) clearTimeout(this.timer)
    this.timer = null
    this.soundFontFile = null
    this.allInputs = true
    this.chosen = null
    this.paletteLeds = false
    this.scanning = false
  }
}

export const settings = new SettingsAdapter()
