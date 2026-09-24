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
  allInputs: boolean
  paletteLeds: boolean
  roots: string[]
  scanning: boolean
  /** True while these come from this adapter's mock rather than the engine. */
  mocked: boolean
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

  /** Everything the drawer shows, from the engine when it has the field, else the mock. */
  view(s: AppState): SettingsView {
    const p = s as unknown as Proposed
    const connected = s.io.inputs
    const names = p.io?.sources?.map((x) => x.name) ?? [...connected, ...MOCK_EXTRA_SOURCES.filter((n) => !connected.includes(n))]
    const allInputs = p.io?.allInputs ?? this.allInputs
    const chosen = this.chosen ?? connected
    const sources = names.map((name) => ({
      name,
      pads: isPadsPort(name),
      listening: p.io?.sources?.find((x) => x.name === name)?.listening ?? (allInputs || chosen.includes(name)),
    }))
    const soundFonts = p.io?.soundFonts ?? MOCK_SOUNDFONTS
    const fromSynth = s.io.synth ? soundFonts.find((f) => f.replace(/\.sf2$/i, '') === s.io.synth!.soundFont) ?? null : null
    return {
      soundFonts,
      soundFontFile: p.io?.soundFontFile ?? this.soundFontFile ?? fromSynth,
      sources,
      allInputs,
      paletteLeds: p.pads?.paletteLeds ?? this.paletteLeds,
      roots: p.library?.roots ?? MOCK_ROOTS,
      scanning: p.library?.scanning ?? (this.scanning || s.library.pending > 0),
      mocked: p.io?.soundFonts === undefined,
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
